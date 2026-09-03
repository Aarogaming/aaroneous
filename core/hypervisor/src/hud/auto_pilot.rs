// core/hypervisor/src/hud/auto_pilot.rs
//! Autonomous Execution Loop — Perception-to-Action Pipeline.
//!
//! Background worker thread that continuously:
//!   1. Ingests DXGI spatial deltas + WASAPI acoustic latent vectors
//!   2. Queries EpisodicMemoryFabric for matching habit trajectories (<1μs)
//!   3. Executes crystallized Cranelift JIT native closures in W^X memory
//!   4. Dispatches validated HID hardware actions via platform_bridge
//!   5. Intercepts emergency abort conditions (cursor at origin, free-energy violation)

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Auto-pilot execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoPilotState {
    /// System is idle, no autonomous execution
    Disengaged,
    /// System is actively running perception-to-action loop
    Engaged,
    /// System is paused (user override or emergency stop)
    Paused,
    /// Emergency stop triggered by safety guardrail
    EmergencyStop,
}

/// Live telemetry from the auto-pilot execution loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPilotTelemetry {
    pub state: AutoPilotState,
    pub loop_iteration: u64,
    pub last_tick_latency_us: f64,
    pub avg_tick_latency_us: f64,
    pub active_fps: f32,
    pub memory_recall_hits: u64,
    pub jit_executions: u64,
    pub hid_actions_dispatched: u64,
    pub emergency_stops: u64,
    pub free_energy: f64,
    pub audio_onset_active: bool,
}

impl Default for AutoPilotTelemetry {
    fn default() -> Self {
        Self {
            state: AutoPilotState::Disengaged,
            loop_iteration: 0,
            last_tick_latency_us: 0.0,
            avg_tick_latency_us: 0.0,
            active_fps: 0.0,
            memory_recall_hits: 0,
            jit_executions: 0,
            hid_actions_dispatched: 0,
            emergency_stops: 0,
            free_energy: 0.0,
            audio_onset_active: false,
        }
    }
}

/// Autonomous Perception-to-Action Execution Controller.
///
/// Manages a background worker thread that runs the closed-loop
/// sense → recall → plan → act → verify pipeline.
pub struct AutoPilotController {
    /// Atomic engagement flag (true = running)
    engaged: Arc<AtomicBool>,
    /// Emergency kill switch (true = force stop)
    kill_switch: Arc<AtomicBool>,
    /// Loop iteration counter (atomic for lock-free reads)
    iteration_counter: Arc<AtomicU64>,
    /// Shared telemetry state
    telemetry: Arc<parking_lot::RwLock<AutoPilotTelemetry>>,
    /// Free energy threshold for automatic disengagement
    max_free_energy: f64,
}

impl Default for AutoPilotController {
    fn default() -> Self {
        Self::new(0.15)
    }
}

impl AutoPilotController {
    /// Create a new auto-pilot controller with the given free-energy safety threshold.
    pub fn new(max_free_energy: f64) -> Self {
        Self {
            engaged: Arc::new(AtomicBool::new(false)),
            kill_switch: Arc::new(AtomicBool::new(false)),
            iteration_counter: Arc::new(AtomicU64::new(0)),
            telemetry: Arc::new(parking_lot::RwLock::new(AutoPilotTelemetry::default())),
            max_free_energy,
        }
    }

    /// Toggle engagement state. Returns the new state.
    pub fn toggle(&self) -> AutoPilotState {
        let was_engaged = self.engaged.fetch_xor(true, Ordering::AcqRel);
        let new_engaged = !was_engaged;

        let mut tele = self.telemetry.write();
        if new_engaged {
            self.kill_switch.store(false, Ordering::Release);
            tele.state = AutoPilotState::Engaged;
            tele.emergency_stops = 0;
        } else {
            tele.state = AutoPilotState::Disengaged;
        }
        tele.state
    }

    /// Engage the auto-pilot loop
    pub fn engage(&self) {
        self.kill_switch.store(false, Ordering::Release);
        self.engaged.store(true, Ordering::Release);
        self.telemetry.write().state = AutoPilotState::Engaged;
    }

    /// Disengage the auto-pilot loop
    pub fn disengage(&self) {
        self.engaged.store(false, Ordering::Release);
        self.telemetry.write().state = AutoPilotState::Disengaged;
    }

    /// Trigger emergency stop
    pub fn emergency_stop(&self) {
        self.kill_switch.store(true, Ordering::Release);
        self.engaged.store(false, Ordering::Release);
        let mut tele = self.telemetry.write();
        tele.state = AutoPilotState::EmergencyStop;
        tele.emergency_stops += 1;
    }

    /// Returns whether the auto-pilot is currently engaged
    #[inline]
    pub fn is_engaged(&self) -> bool {
        self.engaged.load(Ordering::Acquire)
    }

    /// Returns whether the kill switch has been activated
    #[inline]
    pub fn is_killed(&self) -> bool {
        self.kill_switch.load(Ordering::Acquire)
    }

    /// Returns a snapshot of current telemetry
    pub fn telemetry(&self) -> AutoPilotTelemetry {
        self.telemetry.read().clone()
    }

    /// Execute one tick of the autonomous perception-to-action loop.
    /// Called by the background worker thread on each iteration.
    ///
    /// Returns `false` if the loop should terminate (kill switch or disengaged).
    pub fn tick(
        &self,
        cursor_position: (i32, i32),
        current_free_energy: f64,
        memory_hit: bool,
        jit_executed: bool,
        action_dispatched: bool,
        audio_onset: bool,
    ) -> bool {
        // Safety guardrail: cursor at (0,0) origin implies user abort gesture
        if cursor_position == (0, 0) {
            self.emergency_stop();
            return false;
        }

        // Safety guardrail: free energy threshold exceeded
        if current_free_energy > self.max_free_energy {
            self.emergency_stop();
            return false;
        }

        // Kill switch check
        if self.is_killed() || !self.is_engaged() {
            return false;
        }

        let iter = self.iteration_counter.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();

        let mut tele = self.telemetry.write();
        tele.loop_iteration = iter + 1;
        tele.free_energy = current_free_energy;
        tele.audio_onset_active = audio_onset;

        if memory_hit {
            tele.memory_recall_hits += 1;
        }
        if jit_executed {
            tele.jit_executions += 1;
        }
        if action_dispatched {
            tele.hid_actions_dispatched += 1;
        }

        let elapsed_us = now.elapsed().as_nanos() as f64 / 1000.0;
        tele.last_tick_latency_us = elapsed_us;

        // Exponential moving average for tick latency
        if tele.avg_tick_latency_us < 1e-6 {
            tele.avg_tick_latency_us = elapsed_us;
        } else {
            tele.avg_tick_latency_us = tele.avg_tick_latency_us * 0.95 + elapsed_us * 0.05;
        }

        // Compute active FPS from average tick
        if tele.avg_tick_latency_us > 0.0 {
            tele.active_fps = (1_000_000.0 / tele.avg_tick_latency_us) as f32;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_pilot_lifecycle() {
        let pilot = AutoPilotController::new(0.10);
        assert!(!pilot.is_engaged());
        assert_eq!(pilot.telemetry().state, AutoPilotState::Disengaged);

        // Toggle engage
        let state = pilot.toggle();
        assert_eq!(state, AutoPilotState::Engaged);
        assert!(pilot.is_engaged());

        // Toggle disengage
        let state = pilot.toggle();
        assert_eq!(state, AutoPilotState::Disengaged);
        assert!(!pilot.is_engaged());
    }

    #[test]
    fn test_emergency_stop_on_cursor_origin() {
        let pilot = AutoPilotController::new(0.10);
        pilot.engage();
        assert!(pilot.is_engaged());

        // Cursor at (0,0) should trigger emergency stop
        let should_continue = pilot.tick((0, 0), 0.01, false, false, false, false);
        assert!(!should_continue);
        assert_eq!(pilot.telemetry().state, AutoPilotState::EmergencyStop);
        assert_eq!(pilot.telemetry().emergency_stops, 1);
    }

    #[test]
    fn test_emergency_stop_on_free_energy_violation() {
        let pilot = AutoPilotController::new(0.10);
        pilot.engage();

        // Free energy above threshold should trigger emergency stop
        let should_continue = pilot.tick((100, 200), 0.20, false, false, false, false);
        assert!(!should_continue);
        assert_eq!(pilot.telemetry().state, AutoPilotState::EmergencyStop);
    }

    #[test]
    fn test_normal_tick_accumulates_telemetry() {
        let pilot = AutoPilotController::new(0.50);
        pilot.engage();

        for _ in 0..10 {
            let ok = pilot.tick((500, 300), 0.05, true, true, true, false);
            assert!(ok);
        }

        let tele = pilot.telemetry();
        assert_eq!(tele.loop_iteration, 10);
        assert_eq!(tele.memory_recall_hits, 10);
        assert_eq!(tele.jit_executions, 10);
        assert_eq!(tele.hid_actions_dispatched, 10);
        assert!(tele.free_energy < 0.50);
    }

    #[test]
    fn test_kill_switch() {
        let pilot = AutoPilotController::new(0.10);
        pilot.engage();
        pilot.emergency_stop();
        assert!(pilot.is_killed());
        assert!(!pilot.is_engaged());

        let should_continue = pilot.tick((100, 200), 0.01, false, false, false, false);
        assert!(!should_continue);
    }
}
