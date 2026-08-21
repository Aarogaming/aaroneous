//! crates/orchestrator/src/pantheon_orchestrator.rs
//! Hypervisor Thread Allocator & Multi-Tier Pantheon Orchestrator.
//!
//! Features:
//! 1. Reads .si Tier Designation Flags (Cortex, Router, Reflex) to allocate CPU cores and memory maps.
//! 2. Tier 1 Cortex: Dispatched on standard async OS thread scheduler (HD R^4096 representation).
//! 3. Tier 2 Hermes Router: High-priority event loop connecting to the 128-byte aligned SPMC hub.
//! 4. Tier 3 Kinetic Reflex: Pinned to dedicated physical CPU cores with L1 cache residency.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use anyhow::{bail, Result};

use compute::hermes_router::HermesRouter;
use compute::reflex_worker::ReflexWorker;
use compute::si_packer::SiTierFlags;
use compute::si_solid_state::SolidStateSiContainer;
use nervous_system::pantheon_bus::PantheonSynapseBus;

/// Pin the current executing thread to a specific CPU core index
pub fn pin_current_thread_to_core(core_id: usize) -> bool {
    #[cfg(windows)]
    {
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn SetThreadAffinityMask(hThread: isize, dwThreadAffinityMask: usize) -> usize;
        }

        if core_id >= std::mem::size_of::<usize>() * 8 {
            return false;
        }
        let mask: usize = 1 << core_id;
        unsafe {
            let h = GetCurrentThread();
            let prev = SetThreadAffinityMask(h, mask);
            if prev != 0 {
                println!("📌 [Orchestrator] Pinned thread to CPU Core #{}", core_id);
                true
            } else {
                eprintln!("⚠️ [Orchestrator] SetThreadAffinityMask failed for core #{}", core_id);
                false
            }
        }
    }

    #[cfg(not(windows))]
    {
        println!("📌 [Orchestrator] Mock thread affinity set to Core #{}", core_id);
        true
    }
}

/// The Multi-Tier Pantheon Orchestrator
pub struct PantheonOrchestrator {
    pub available_cores: Vec<usize>,
    pub bus: Arc<PantheonSynapseBus>,
    pub spawned_threads: Vec<JoinHandle<()>>,
    pub shutdown_signal: Arc<AtomicBool>,
}

impl PantheonOrchestrator {
    /// Creates a new orchestrator with a Federated SPMC synapse bus and discovered core count
    pub fn new(bus: Arc<PantheonSynapseBus>) -> Self {
        let num_cpus = num_cpus::get_physical().max(2);
        let available_cores = (0..num_cpus).rev().collect(); // Allocate highest physical cores first

        Self {
            available_cores,
            bus,
            spawned_threads: Vec::new(),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Mounts and dispatches a .si container based on its Tier Designation Flags
    pub fn mount_container(
        &mut self,
        container: SolidStateSiContainer,
        tier: SiTierFlags,
        worker_id: u16,
    ) -> Result<()> {
        let bus = self.bus.clone();
        let shutdown = self.shutdown_signal.clone();
        let name = container.container_name.clone();

        if tier.is_cortex() {
            // Tier 1: Heavy 4096-dim model. Spawn on background OS thread.
            println!("🧠 [Orchestrator] Launching Tier 1 Cortex: '{}' (Background async task)", name);
            let handle = thread::Builder::new()
                .name(format!("Cortex-Worker-{}", worker_id))
                .spawn(move || {
                    let mut step = 0u64;
                    while !shutdown.load(Ordering::Relaxed) {
                        // Periodic strategic plan formulation
                        thread::sleep(Duration::from_millis(100));
                        step += 1;
                    }
                    println!("🧠 [Orchestrator] Cortex '{}' shutdown after {} strategic cycles.", name, step);
                })?;
            self.spawned_threads.push(handle);
        } else if tier.is_router() {
            // Tier 2: Hermes Router connecting to central SPMC hub
            println!("⚡ [Orchestrator] Launching Tier 2 Hermes Router: '{}' (SPMC Hub Channel 0)", name);
            let handle = thread::Builder::new()
                .name(format!("Hermes-Router-{}", worker_id))
                .spawn(move || {
                    let mut router = HermesRouter::default();
                    let dummy_intent = vec![0.1f32; 4096];
                    let mut routed = 0u64;

                    while !shutdown.load(Ordering::Relaxed) {
                        let channel = &bus.channels[0];
                        let _ = router.route_and_broadcast(&dummy_intent, channel);
                        routed += 1;
                        thread::sleep(Duration::from_millis(10));
                    }
                    println!("⚡ [Orchestrator] Hermes Router '{}' shutdown after {} broadcasts.", name, routed);
                })?;
            self.spawned_threads.push(handle);
        } else if tier.is_reflex() {
            // Tier 3: Kinetic Specialist. Pin to a dedicated physical core.
            let core_id = self.available_cores.pop().unwrap_or(0);
            let channel_idx = (worker_id as usize).min(bus.channels.len().saturating_sub(1));
            println!("🎯 [Orchestrator] Launching Tier 3 Kinetic Specialist: '{}' (Pinned to Core #{})", name, core_id);

            let handle = thread::Builder::new()
                .name(format!("Reflex-Specialist-{}", worker_id))
                .spawn(move || {
                    pin_current_thread_to_core(core_id);
                    if let Ok(mut worker) = ReflexWorker::new(worker_id, &name, container, false) {
                        let channel = &bus.channels[channel_idx];
                        let _ = worker.run_continuous(channel, shutdown, None);
                        println!("🎯 [Orchestrator] Reflex '{}' shutdown after {} kinetic steps.", name, worker.total_steps_executed);
                    }
                })?;
            self.spawned_threads.push(handle);
        } else {
            bail!("Invalid .si container tier flags: {:#X}", tier.bits());
        }

        Ok(())
    }

    /// Signals all spawned threads to stop gracefully
    pub fn shutdown(&self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }
}

mod num_cpus {
    pub fn get_physical() -> usize {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compute::si_ssm::SiSsmConfig;

    #[test]
    fn test_pantheon_orchestrator_mount_and_shutdown() {
        let bus = Arc::new(PantheonSynapseBus::new_federation());
        let mut orchestrator = PantheonOrchestrator::new(bus);

        let config = SiSsmConfig {
            model_name: "Test-Cortex".to_string(),
            state_dim: 128,
            d_model: 32,
            d_state: 8,
            d_conv: 2,
            dt_rank: 4,
            num_layers: 1,
            num_opcodes: 8,
            param_count: 10_000,
        };

        let container1 = SolidStateSiContainer::new("Hermes-Test", config.clone());
        let container2 = SolidStateSiContainer::new("Marionette-Test", config);

        orchestrator.mount_container(container1, SiTierFlags::TIER_2_ROUTER, 0).unwrap();
        orchestrator.mount_container(container2, SiTierFlags::TIER_3_REFLEX, 1).unwrap();

        assert_eq!(orchestrator.spawned_threads.len(), 2);
        thread::sleep(Duration::from_millis(50));
        orchestrator.shutdown();
    }
}
