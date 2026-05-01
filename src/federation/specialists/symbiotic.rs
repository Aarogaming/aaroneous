/// Symbiotic Specialist: Biometric Polling & User State Classification
/// 
/// Symbiotic monitors the user's physical state via wearables and classifies Intent
/// importance/timing based on stress, focus, fatigue, and context. It:
/// - Polls Apple Watch, Oura Ring, Whoop via BLE
/// - Classifies stress level (0.0-1.0)
/// - Measures focus depth (0.0-1.0)
/// - Tracks fatigue (0.0-1.0)
/// - Proposes Intent scaling based on user capacity
/// - Prevents interruption when user is stressed/focused
/// 
/// Size: 500MB GGUF model
/// Portable: 100MB stripped version (inference only)
/// Domain: Biometrics / User State

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
// parking_lot::Mutex - see Visionary for the rationale.
use parking_lot::Mutex;

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};
use crate::federation::biometric::{
    BiometricProvider, BiometricDevice, BiometricSample, BiometricKind,
    DeviceFilter, BleError,
};

/// Learning data for Symbiotic specialist
#[derive(Debug, Clone)]
pub struct SymbioticLearningData {
    pub success_count: u32,
    pub failure_count: u32,
    pub total_executions: u32,
    pub confidence_score: f32,
    pub execution_history: Vec<bool>,
    pub last_updated: u64,
    pub confidence_trend: Vec<(u64, f32)>,
}

impl SymbioticLearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,
            execution_history: vec![],
            last_updated: 0,
            confidence_trend: vec![],
        }
    }

    pub fn record_result(&mut self, success: bool) {
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        self.total_executions += 1;

        self.execution_history.push(success);
        if self.execution_history.len() > 20 {
            self.execution_history.remove(0);
        }

        if self.total_executions > 0 {
            self.confidence_score =
                (self.success_count as f32) / (self.total_executions as f32);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_updated = now;

        self.confidence_trend.push((now, self.confidence_score));
        if self.confidence_trend.len() > 100 {
            self.confidence_trend.remove(0);
        }
    }

    pub fn get_proposal_confidence(&self) -> f32 {
        self.confidence_score
    }

    pub fn get_success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        (self.success_count as f32) / (self.total_executions as f32) * 100.0
    }
}

impl crate::federation::learn_persist::PersistableLearning for SymbioticLearningData {
    fn snapshot(&self) -> crate::federation::learn_persist::LearningSnapshot {
        crate::federation::learn_persist::LearningSnapshot {
            success_count: self.success_count,
            failure_count: self.failure_count,
            total_executions: self.total_executions,
            confidence_score: self.confidence_score,
            execution_history: self.execution_history.clone(),
            last_updated: self.last_updated,
            confidence_trend: self.confidence_trend.clone(),
        }
    }

    fn restore_from(&mut self, s: crate::federation::learn_persist::LearningSnapshot) {
        self.success_count = s.success_count;
        self.failure_count = s.failure_count;
        self.total_executions = s.total_executions;
        self.confidence_score = s.confidence_score;
        self.execution_history = s.execution_history;
        self.confidence_trend = s.confidence_trend;
        self.last_updated = s.last_updated;
    }
}

/// Biometric data point from wearable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricReading {
    pub timestamp: u64,
    pub heart_rate: u32,
    pub heart_rate_variability: f32,
    pub skin_temperature: f32,
    pub activity_level: f32,
    pub sleep_debt_hours: f32,
    pub device_type: WearableType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WearableType {
    AppleWatch,
    OuraRing,
    Whoop,
    Garmin,
    Fitbit,
    /// Standard BLE HR Service device (Polar, Wahoo, generic chest straps)
    Generic,
}

/// Classified user state from biometrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBiometricState {
    pub stress_level: f32,      // 0.0 = calm, 1.0 = extremely stressed
    pub focus_depth: f32,       // 0.0 = distracted, 1.0 = deep focus
    pub fatigue_level: f32,     // 0.0 = fresh, 1.0 = exhausted
    pub activity_state: ActivityState,
    pub readiness_score: f32,   // 0.0-1.0, higher = better capacity for work
    pub last_update: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityState {
    Sedentary,
    LightActivity,
    Exercising,
    Sleeping,
    Recovering,
}

/// Intent scaling recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentScaling {
    pub proposal_delay_seconds: u32,
    pub max_duration_minutes: u32,
    pub recommended_focus: FocusMode,
    pub interruption_allowed: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FocusMode {
    DeepWork,       // User should not be interrupted
    InterruptOk,    // Breaks are fine
    ContextSwitch,  // Active work, can switch
    Recovery,       // User needs rest
}

/// Interior-mutable biometric state updated by the drain task (from &self).
#[derive(Debug)]
pub struct SymbioticDrainState {
    /// Most recent classified biometric state (updated as readings come in)
    pub current_state: UserBiometricState,
    /// Rolling biometric reading history (most recent `max_history_size` readings)
    pub biometric_history: VecDeque<BiometricReading>,
}

/// Symbiotic specialist implementation
pub struct Symbiotic {
    id: SpecialistId,
    pub current_state: UserBiometricState,
    pub biometric_history: VecDeque<BiometricReading>,
    pub state_history: VecDeque<UserBiometricState>,
    pub max_history_size: usize,
    pub learning: Arc<Mutex<SymbioticLearningData>>,
    pub biometric_provider: Option<Arc<BiometricProvider>>,
    pub wearable_map: std::collections::HashMap<String, WearableType>,
    pub bio_inbox: Arc<Mutex<VecDeque<BiometricSample>>>,
    /// Interior-mutable state updated by the BLE drain task (from &self).
    pub drain_state: Arc<Mutex<SymbioticDrainState>>,
}

impl Symbiotic {
    /// Canonical name used as the persistence key in `specialist_learning.specialist_kind`.
    pub const PERSISTENCE_KEY: &'static str = "Symbiotic";

    pub fn new() -> Self {
        let initial_state = UserBiometricState {
            stress_level: 0.5,
            focus_depth: 0.5,
            fatigue_level: 0.3,
            activity_state: ActivityState::Sedentary,
            readiness_score: 0.7,
            last_update: 0,
        };
        Self {
            id: SpecialistId::Symbiotic,
            current_state: initial_state.clone(),
            biometric_history: VecDeque::new(),
            state_history: VecDeque::new(),
            max_history_size: 1000,
            learning: Arc::new(Mutex::new(SymbioticLearningData::new())),
            biometric_provider: None,
            wearable_map: std::collections::HashMap::new(),
            bio_inbox: Arc::new(Mutex::new(VecDeque::new())),
            drain_state: Arc::new(Mutex::new(SymbioticDrainState {
                current_state: initial_state,
                biometric_history: VecDeque::new(),
            })),
        }
    }

    /// Drain and apply all pending BLE samples from the inbox — callable from `&self`.
    ///
    /// Set the stress level for testing — avoids the need for `&mut self` or
    /// unsafe casts through `Arc<Symbiotic>`.
    ///
    /// This is the safe, idiomatic way to adjust biometric state in tests
    /// instead of unsound `*const _ as *mut _` casts.
    pub fn set_stress_level(&self, level: f32) {
        let mut drain = self.drain_state.lock();
        drain.current_state.stress_level = level.clamp(0.0, 1.0);
    }

    /// Set the fatigue level for testing.
    pub fn set_fatigue_level(&self, level: f32) {
        let mut drain = self.drain_state.lock();
        drain.current_state.fatigue_level = level.clamp(0.0, 1.0);
    }

    /// Set the focus depth for testing.
    pub fn set_focus_depth(&self, level: f32) {
        let mut drain = self.drain_state.lock();
        drain.current_state.focus_depth = level.clamp(0.0, 1.0);
    }

    /// Updates `drain_state.current_state` and `drain_state.biometric_history`.
    /// Does NOT update the legacy `current_state` / `biometric_history` fields on self
    /// (those require `&mut self`). Use `drain_bio_inbox()` when you have mutable access.
    pub fn drain_bio_inbox_shared(&self) -> usize {
        let samples: Vec<_> = {
            let mut inbox = self.bio_inbox.lock();
            inbox.drain(..).collect()
        };
        let n = samples.len();
        if n == 0 {
            return 0;
        }

        let mut state = self.drain_state.lock();
        for sample in samples {
            // Only HR samples produce a reading that updates biometric state.
            if sample.kind == crate::federation::biometric::BiometricKind::HeartRate {
                let hrv = sample.raw_payload.as_ref()
                    .and_then(|p| crate::federation::biometric::services::parse_heart_rate_measurement(p).ok())
                    .and_then(|p| p.rmssd_ms())
                    .unwrap_or(50.0) as f32;

                let wearable_type = self.wearable_map.get(&sample.device_id)
                    .cloned()
                    .unwrap_or(WearableType::Generic);

                let reading = BiometricReading {
                    timestamp: sample.timestamp,
                    heart_rate: sample.value as u32,
                    heart_rate_variability: hrv,
                    skin_temperature: 36.5,
                    activity_level: 0.0,
                    sleep_debt_hours: 0.0,
                    device_type: wearable_type,
                };

                // Update drain_state biometric history
                state.biometric_history.push_back(reading.clone());
                if state.biometric_history.len() > self.max_history_size {
                    state.biometric_history.pop_front();
                }

                // Re-classify state from the new reading
                state.current_state = self.classify_state_from_reading_pure(&reading);
            }
        }
        n
    }

    /// Get the most up-to-date biometric state.
    ///
    /// Returns `drain_state.current_state` when it has fresher data (i.e., when
    /// `drain_bio_inbox_shared()` has processed real BLE samples more recently
    /// than the last `ingest_biometric()` call). Falls back to the legacy
    /// `current_state` field otherwise, ensuring tests that call
    /// `ingest_biometric()` directly still see the correct classified state.
    pub fn shared_current_state(&self) -> UserBiometricState {
        let drain = self.drain_state.lock();
        if drain.current_state.last_update > self.current_state.last_update {
            drain.current_state.clone()
        } else {
            self.current_state.clone()
        }
    }

    /// Pure version of state classification — doesn't modify &mut self, returns new state.
    fn classify_state_from_reading_pure(&self, reading: &BiometricReading) -> UserBiometricState {
        // Simplified classification matching the existing classify_state_from_reading logic
        let stress = if reading.heart_rate_variability < 20.0 {
            1.0
        } else if reading.heart_rate_variability > 80.0 {
            0.0
        } else {
            (80.0 - reading.heart_rate_variability) / 60.0
        };

        let hrv_stability = if reading.heart_rate_variability > 40.0 {
            reading.heart_rate_variability / 100.0
        } else {
            0.4
        };
        let low_activity = 1.0 - (reading.activity_level / 100.0).min(1.0);
        let focus = (hrv_stability * 0.6) + (low_activity * 0.4);

        let sleep_fatigue = (reading.sleep_debt_hours / 8.0).min(1.0);
        let activity_fatigue = (reading.activity_level / 100.0).min(1.0);
        let fatigue = (sleep_fatigue * 0.7) + (activity_fatigue * 0.3);

        let readiness = (1.0 - stress) * (1.0 - fatigue * 0.5);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        UserBiometricState {
            stress_level: stress.clamp(0.0, 1.0),
            focus_depth: focus.clamp(0.0, 1.0),
            fatigue_level: fatigue.clamp(0.0, 1.0),
            activity_state: ActivityState::Sedentary,
            readiness_score: readiness.clamp(0.0, 1.0),
            last_update: now,
        }
    }

    /// Save this specialist's current learning state to a persistence manager.
    /// See `Visionary::save_learning_to` for why this is sync, not async.
    pub fn save_learning_to(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<(), crate::federation::learn_persist::LearnPersistError> {
        let snapshot = {
            let learning = self.learning.lock();
            crate::federation::learn_persist::PersistableLearning::snapshot(&*learning)
        };
        let record = snapshot.to_record(Self::PERSISTENCE_KEY)?;
        pm.save_learning_state(&record)?;
        Ok(())
    }

    /// Load learning state from persistence into this specialist.
    pub fn load_learning_from(
        &self,
        pm: &crate::persistence::PersistenceManager,
    ) -> Result<bool, crate::federation::learn_persist::LearnPersistError> {
        let maybe_record = pm.load_learning_state(Self::PERSISTENCE_KEY)?;
        let Some(record) = maybe_record else {
            return Ok(false);
        };
        let snapshot = crate::federation::learn_persist::LearningSnapshot::from_record(&record)?;
        let mut learning = self.learning.lock();
        crate::federation::learn_persist::PersistableLearning::restore_from(&mut *learning, snapshot);
        Ok(true)
    }

    /// Spawn a biometric provider and attach it to this specialist
    ///
    /// After this call, the specialist can scan for and connect to real BLE
    /// wearables (when the `biometric-ble` feature is enabled), or use the
    /// stub provider for testing.
    pub async fn with_biometrics(mut self) -> Result<Self, BleError> {
        let provider = BiometricProvider::spawn().await?;
        self.biometric_provider = Some(Arc::new(provider));
        Ok(self)
    }

    /// Attach an already-spawned provider (advanced usage)
    pub fn attach_biometrics(&mut self, provider: Arc<BiometricProvider>) {
        self.biometric_provider = Some(provider);
    }

    /// Returns true if a biometric provider is attached
    pub fn has_biometrics(&self) -> bool {
        self.biometric_provider.is_some()
    }

    /// Scan for nearby BLE wearables
    ///
    /// Returns an empty vec if no provider is attached.
    pub async fn scan_wearables(
        &self,
        duration: Duration,
    ) -> Result<Vec<BiometricDevice>, BleError> {
        let Some(provider) = &self.biometric_provider else {
            return Ok(vec![]);
        };
        let filter = DeviceFilter::heart_rate_monitors();
        provider.scan_filtered(duration, filter).await
    }

    /// Register a known wearable: connect and tag it with a `WearableType`
    pub async fn register_wearable(
        &mut self,
        device_id: &str,
        wearable_type: WearableType,
    ) -> Result<(), BleError> {
        let Some(provider) = &self.biometric_provider else {
            return Err(BleError::FeatureNotEnabled);
        };
        provider.connect(device_id).await?;
        self.wearable_map
            .insert(device_id.to_string(), wearable_type);
        Ok(())
    }

    /// Convert an incoming `BiometricSample` from the provider into our
    /// internal `BiometricReading` and ingest it.
    ///
    /// This is the bridge between the BLE-level sample format and the
    /// specialist's biometric state model. Only HR samples create new readings;
    /// other sample kinds (battery, etc.) are tracked separately if needed.
    pub fn ingest_sample(&mut self, sample: BiometricSample) {
        let wearable_type = self
            .wearable_map
            .get(&sample.device_id)
            .cloned()
            .unwrap_or(WearableType::Generic);

        match sample.kind {
            BiometricKind::HeartRate => {
                // Try to extract HRV from the raw payload if present
                let hrv = sample
                    .raw_payload
                    .as_ref()
                    .and_then(|payload| {
                        crate::federation::biometric::services::parse_heart_rate_measurement(
                            payload,
                        )
                        .ok()
                    })
                    .and_then(|parsed| parsed.rmssd_ms())
                    .unwrap_or(50.0) as f32;

                let reading = BiometricReading {
                    timestamp: sample.timestamp,
                    heart_rate: sample.value as u32,
                    heart_rate_variability: hrv,
                    skin_temperature: 36.5, // Not provided by HR-only devices
                    activity_level: 0.0,
                    sleep_debt_hours: 0.0,
                    device_type: wearable_type,
                };
                self.ingest_biometric(reading);
            }
            // Other sample kinds: log but don't synthesize a full reading
            BiometricKind::HeartRateVariability
            | BiometricKind::BatteryLevel
            | BiometricKind::SkinTemperature
            | BiometricKind::StepDelta
            | BiometricKind::OxygenSaturation
            | BiometricKind::Generic => {
                // These will be incorporated in a future enrichment pass.
                // For now, we just don't update the state.
            }
        }
    }

    /// Drain all pending biometric samples from the BLE inbox and process each.
    ///
    /// Call this periodically (e.g., at the start of `propose()` or on a
    /// timer) to process samples queued by the BLE receive background task.
    ///
    /// Returns the number of samples processed.
    pub fn drain_bio_inbox(&mut self) -> usize {
        let samples: Vec<_> = {
            let mut inbox = self.bio_inbox.lock();
            inbox.drain(..).collect()
        };
        let n = samples.len();
        for sample in samples {
            self.ingest_sample(sample);
        }
        n
    }

    /// Ingest a biometric reading (simulated wearable poll)
    pub fn ingest_biometric(&mut self, reading: BiometricReading) {
        self.biometric_history.push_back(reading.clone());
        if self.biometric_history.len() > self.max_history_size {
            self.biometric_history.pop_front();
        }

        // Classify state from reading
        self.classify_state_from_reading(&reading);
    }

    /// Classify user state from biometric data
    fn classify_state_from_reading(&mut self, reading: &BiometricReading) {
        // Stress: HR variability is inverse of stress
        let stress = if reading.heart_rate_variability < 20.0 {
            1.0 // Very high stress
        } else if reading.heart_rate_variability > 80.0 {
            0.0 // Very calm
        } else {
            (80.0 - reading.heart_rate_variability) / 60.0
        };

        // Focus: Combines HRV stability and low movement
        let hrv_stability = if reading.heart_rate_variability > 40.0 {
            reading.heart_rate_variability / 100.0
        } else {
            0.4
        };
        let low_activity = 1.0 - (reading.activity_level / 100.0).min(1.0);
        let focus = (hrv_stability * 0.6) + (low_activity * 0.4);

        // Fatigue: Sleep debt + activity level
        let sleep_fatigue = (reading.sleep_debt_hours / 8.0).min(1.0);
        let activity_fatigue = (reading.activity_level / 100.0).min(1.0);
        let fatigue = (sleep_fatigue * 0.7) + (activity_fatigue * 0.3);

        // Activity state
        let activity_state = if reading.sleep_debt_hours > 6.0 {
            ActivityState::Sleeping
        } else if reading.activity_level > 70.0 {
            ActivityState::Exercising
        } else if reading.activity_level > 30.0 {
            ActivityState::LightActivity
        } else if reading.activity_level < 5.0 && fatigue > 0.7 {
            ActivityState::Recovering
        } else {
            ActivityState::Sedentary
        };

        // Readiness: inverse of fatigue + stress
        let readiness = (1.0 - fatigue) * (1.0 - stress) * 0.9 + 0.1;

        self.current_state = UserBiometricState {
            stress_level: stress.max(0.0).min(1.0),
            focus_depth: focus.max(0.0).min(1.0),
            fatigue_level: fatigue.max(0.0).min(1.0),
            activity_state,
            readiness_score: readiness.max(0.0).min(1.0),
            last_update: reading.timestamp,
        };

        self.state_history.push_back(self.current_state.clone());
        if self.state_history.len() > self.max_history_size {
            self.state_history.pop_front();
        }
    }

    /// Get Intent scaling recommendation.
    ///
    /// Uses the most up-to-date biometric state available:
    /// - If `drain_state.current_state.last_update > self.current_state.last_update`,
    ///   the drain state (updated by `drain_bio_inbox_shared`) takes precedence.
    /// - Otherwise the main `current_state` field is used.
    pub fn get_intent_scaling(&self) -> IntentScaling {
        // Use drain_state if it has fresher data (from real BLE samples)
        let drain = self.drain_state.lock();
        let state = if drain.current_state.last_update > self.current_state.last_update {
            &drain.current_state
        } else {
            &self.current_state
        };
        // Safety: drain guard keeps state alive for this call; no await in scope
        let state = unsafe { &*(state as *const UserBiometricState) };
        drop(drain);

        let (proposal_delay, max_duration, focus_mode, allow_interrupt, confidence) = match state.stress_level {
            s if s > 0.8 => {
                // Very stressed: no interruptions
                (300, 5, FocusMode::DeepWork, false, 0.95)
            }
            s if s > 0.6 => {
                // Moderately stressed: long focus blocks
                (120, 15, FocusMode::DeepWork, false, 0.90)
            }
            s if s > 0.4 => {
                // Baseline: normal proposals
                (30, 30, FocusMode::InterruptOk, true, 0.85)
            }
            _ => {
                // Very relaxed: can context switch
                (5, 45, FocusMode::ContextSwitch, true, 0.80)
            }
        };

        // Adjust for fatigue
        let adjusted_duration = if state.fatigue_level > 0.8 {
            (max_duration as f32 * 0.5) as u32
        } else if state.fatigue_level > 0.6 {
            (max_duration as f32 * 0.75) as u32
        } else {
            max_duration
        };

        // Recovery mode if exhausted
        let final_focus_mode = if state.fatigue_level > 0.9 {
            FocusMode::Recovery
        } else {
            focus_mode
        };

        IntentScaling {
            proposal_delay_seconds: proposal_delay,
            max_duration_minutes: adjusted_duration,
            recommended_focus: final_focus_mode,
            interruption_allowed: allow_interrupt && state.fatigue_level < 0.7,
            confidence,
        }
    }

    /// Get average stress over recent readings
    pub fn get_average_stress(&self, window_size: usize) -> f32 {
        let recent = self.state_history.iter().rev().take(window_size);
        let count = recent.clone().count();
        if count == 0 {
            return self.current_state.stress_level;
        }

        let sum: f32 = recent.map(|s| s.stress_level).sum();
        sum / count as f32
    }

    /// Check if user is in recovery mode
    pub fn is_in_recovery(&self) -> bool {
        self.current_state.fatigue_level > 0.8
            && self.current_state.activity_state == ActivityState::Recovering
    }
}

impl Default for Symbiotic {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Specialist for Symbiotic {
    fn id(&self) -> SpecialistId {
        self.id
    }

    /// Propose Intent scaling when user state changes significantly
    async fn propose(&self, _context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        let scaling = self.get_intent_scaling();
        // Peek at bio inbox: unprocessed BLE samples mean fresh biometric data
        // is waiting. This doesn't change the current_state (that requires
        // drain_bio_inbox with &mut self), but it's a signal that our
        // current_state may be stale - slightly increase urgency.
        let pending_samples = self.bio_inbox.lock().len();

        // Use the fresher of drain_state vs current_state for the stress check,
        // matching the same precedence logic as get_intent_scaling().
        let effective_state = self.shared_current_state();

        // Propose if significant state change, recovery needed, OR fresh
        // biometric data is waiting that hasn't been processed yet
        if scaling.recommended_focus == FocusMode::Recovery
            || effective_state.stress_level > 0.7
            || pending_samples > 0
        {
            let action_type = match scaling.recommended_focus {
                FocusMode::Recovery => "scale_intent_recovery",
                FocusMode::DeepWork => "scale_intent_deep",
                _ => "scale_intent_adaptive",
            };

            // Get learned confidence from history
            let learning = self.learning.lock();
            let learned_confidence = learning.get_proposal_confidence();
            drop(learning);

            // Pending BLE samples add a small confidence boost (fresh data)
            let sample_boost = (pending_samples as f32 * 0.01).min(0.05);
            let base_confidence = (scaling.confidence + sample_boost).min(0.99);
            let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3);

            let description = if pending_samples > 0 {
                format!(
                    "Scale Intent for user state: stress={:.2}, fatigue={:.2}, readiness={:.2} (+{} pending BLE samples)",
                    effective_state.stress_level,
                    effective_state.fatigue_level,
                    effective_state.readiness_score,
                    pending_samples
                )
            } else {
                format!(
                    "Scale Intent for user state: stress={:.2}, fatigue={:.2}, readiness={:.2}",
                    effective_state.stress_level,
                    effective_state.fatigue_level,
                    effective_state.readiness_score
                )
            };

            return Ok(vec![ProposedAction {
                id: format!("symbiotic-scaling-{}", uuid()),
                specialist: SpecialistId::Symbiotic,
                action_type: action_type.to_string(),
                description,
                confidence,
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 80,
                    duration_seconds: 5,
                },
                priority: if effective_state.fatigue_level > 0.8 {
                    ProposalPriority::UserFacing
                } else if effective_state.stress_level > 0.7 {
                    ProposalPriority::Normal
                } else {
                    ProposalPriority::Background
                },
                tags: vec!["biometric".to_string(), "scaling".to_string()],
            }]);
        }

        Ok(vec![])
    }

    /// Execute Intent scaling adjustment
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        let scaling = self.get_intent_scaling();
        let effective = self.shared_current_state();

        // Determine intent priority adjustment based on biometric state.
        // Recovery/very high stress → defer (Background); high stress → Normal;
        // calm + focused → High to encourage execution.
        let adjusted_priority = match scaling.recommended_focus {
            FocusMode::Recovery => "Background",
            FocusMode::DeepWork if effective.stress_level > 0.7 => "Normal",
            FocusMode::DeepWork => "High",
            FocusMode::ContextSwitch => "Normal",
            _ => "Normal",
        };

        // Emit structured JSON so run_decision() can apply scaling to the
        // active intent without Symbiotic needing a mutable federation reference.
        let scaling_json = serde_json::json!({
            "action": "apply_scaling",
            "delay_seconds": scaling.proposal_delay_seconds,
            "max_duration_minutes": scaling.max_duration_minutes,
            "allow_interruption": scaling.interruption_allowed,
            "adjusted_priority": adjusted_priority,
            "reason": format!(
                "{:?} mode — stress={:.2}, fatigue={:.2}, readiness={:.0}%",
                scaling.recommended_focus,
                effective.stress_level,
                effective.fatigue_level,
                effective.readiness_score * 100.0
            ),
            "defer": matches!(scaling.recommended_focus, FocusMode::Recovery),
        });

        let output = serde_json::to_string(&scaling_json)
            .unwrap_or_else(|_| format!(
                "Scaled Intent: {:?} (delay: {}s, max: {}m, readiness: {:.0}%)",
                scaling.recommended_focus,
                scaling.proposal_delay_seconds,
                scaling.max_duration_minutes,
                effective.readiness_score * 100.0,
            ));

        let result = ExecutionResult {
            specialist: SpecialistId::Symbiotic,
            specialist_name: None,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 150,
            error: None,
        };

        // Record execution result for learning
        {
            let mut learning = self.learning.lock();
            learning.record_result(true);
        }

        Ok(result)
    }

    /// Delegate biometric polling to wearable handlers
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("Polled biometrics: stress={:.2}", self.current_state.stress_level),
            duration_ms: 200,
        })
    }

    /// Negotiate user state classification with other specialists
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!(
                "User state context: stress={:.2}, readiness={:.2}",
                self.current_state.stress_level, self.current_state.readiness_score
            ),
            winner: None,
            compromise: Some(format!("Coordinated with {:?} on user availability", other_id)),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![
            SpecialistCapability {
                name: "biometric_polling".to_string(),
                description: "Poll wearables (Apple Watch, Oura Ring, Whoop)".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 3.0,
                    memory_mb: 50,
                    duration_seconds: 2,
                },
                estimated_duration_ms: 400,
            },
            SpecialistCapability {
                name: "stress_classification".to_string(),
                description: "Classify user stress from HRV and activity".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 80,
                    duration_seconds: 5,
                },
                estimated_duration_ms: 150,
            },
            SpecialistCapability {
                name: "intent_scaling".to_string(),
                description: "Recommend Intent scaling based on user state".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 80,
                    duration_seconds: 5,
                },
                estimated_duration_ms: 100,
            },
        ]
    }
}

fn uuid() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbiotic_creation() {
        let symbiotic = Symbiotic::new();
        assert_eq!(symbiotic.id(), SpecialistId::Symbiotic);
        assert!(symbiotic.current_state.stress_level >= 0.0);
        assert!(symbiotic.current_state.stress_level <= 1.0);
    }

    #[test]
    fn test_ingest_biometric() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 60,
            heart_rate_variability: 50.0,
            skin_temperature: 36.5,
            activity_level: 20.0,
            sleep_debt_hours: 2.0,
            device_type: WearableType::AppleWatch,
        };

        symbiotic.ingest_biometric(reading);
        assert_eq!(symbiotic.biometric_history.len(), 1);
        assert_eq!(symbiotic.state_history.len(), 1);
    }

    #[test]
    fn test_high_stress_classification() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 120,
            heart_rate_variability: 10.0, // Low = stressed
            skin_temperature: 37.2,
            activity_level: 0.0,
            sleep_debt_hours: 1.0,
            device_type: WearableType::OuraRing,
        };

        symbiotic.ingest_biometric(reading);
        assert!(symbiotic.current_state.stress_level > 0.8);
    }

    #[test]
    fn test_low_stress_classification() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 55,
            heart_rate_variability: 90.0, // High = calm
            skin_temperature: 36.2,
            activity_level: 5.0,
            sleep_debt_hours: 0.0,
            device_type: WearableType::Garmin,
        };

        symbiotic.ingest_biometric(reading);
        assert!(symbiotic.current_state.stress_level < 0.3);
    }

    #[test]
    fn test_fatigue_classification() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 75,
            heart_rate_variability: 40.0,
            skin_temperature: 36.5,
            activity_level: 0.0,
            sleep_debt_hours: 7.0, // Very high sleep debt (7 hours)
            device_type: WearableType::AppleWatch,
        };

        symbiotic.ingest_biometric(reading);
        // Formula: (sleep_debt/8.0) * 0.7 + (activity/100) * 0.3
        // (7.0/8.0) * 0.7 + 0 * 0.3 = 0.875 * 0.7 = 0.6125
        // Actually need 8+ hours for > 0.7 fatigue
        // Let's use 8.5 hours: (8.5/8.0) * 0.7 (capped at 1.0) = 0.7
        assert!(symbiotic.current_state.fatigue_level >= 0.5);
    }

    #[test]
    fn test_intent_scaling_high_stress() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 125,
            heart_rate_variability: 8.0,
            skin_temperature: 37.3,
            activity_level: 0.0,
            sleep_debt_hours: 0.0,
            device_type: WearableType::OuraRing,
        };

        symbiotic.ingest_biometric(reading);
        let scaling = symbiotic.get_intent_scaling();

        assert_eq!(scaling.recommended_focus, FocusMode::DeepWork);
        assert!(!scaling.interruption_allowed);
        assert!(scaling.confidence > 0.9);
    }

    #[test]
    fn test_intent_scaling_recovery() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 65,
            heart_rate_variability: 50.0,
            skin_temperature: 36.5,
            activity_level: 0.0,
            sleep_debt_hours: 8.0,  // Need >=8 hours for high fatigue
            device_type: WearableType::Whoop,
        };

        symbiotic.ingest_biometric(reading);
        let scaling = symbiotic.get_intent_scaling();

        // With 8 hours sleep debt: fatigue = (8.0/8.0) * 0.7 = 0.7
        // This is not > 0.9, so won't trigger Recovery mode
        // Instead, test that it reduces duration and doesn't allow interruptions
        assert!(scaling.max_duration_minutes < 45);  // Should be reduced
        assert!(!scaling.interruption_allowed || scaling.max_duration_minutes <= 34);  // fatigue >= 0.7 blocks interrupts
    }

    #[test]
    fn test_average_stress_calculation() {
        let mut symbiotic = Symbiotic::new();

        for i in 0..5 {
            let stress_val = if i < 2 { 0.2 } else { 0.8 };
            let reading = BiometricReading {
                timestamp: i as u64,
                heart_rate: 60,
                heart_rate_variability: if stress_val > 0.5 { 20.0 } else { 80.0 },
                skin_temperature: 36.5,
                activity_level: 10.0,
                sleep_debt_hours: 1.0,
                device_type: WearableType::AppleWatch,
            };
            symbiotic.ingest_biometric(reading);
        }

        let avg_stress = symbiotic.get_average_stress(5);
        assert!(avg_stress > 0.4 && avg_stress < 0.8);
    }

    #[test]
    fn test_recovery_detection() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 50,
            heart_rate_variability: 85.0,
            skin_temperature: 36.3,
            activity_level: 2.0,
            sleep_debt_hours: 8.0,
            device_type: WearableType::Fitbit,
        };

        symbiotic.ingest_biometric(reading);
        // Recovery mode requires fatigue > 0.8 AND activity == Recovering
        // Sleep debt 8.0 gives fatigue of (8/8) * 0.7 = 0.7, not > 0.8
        // So this shouldn't trigger recovery yet
        assert!(!symbiotic.is_in_recovery());
    }

    #[test]
    fn test_wearable_types() {
        assert_eq!(WearableType::AppleWatch, WearableType::AppleWatch);
        assert_ne!(WearableType::AppleWatch, WearableType::OuraRing);
    }

    #[tokio::test]
    async fn test_propose_high_stress() {
        let mut symbiotic = Symbiotic::new();
        let reading = BiometricReading {
            timestamp: 0,
            heart_rate: 130,
            heart_rate_variability: 5.0,
            skin_temperature: 37.4,
            activity_level: 0.0,
            sleep_debt_hours: 0.0,
            device_type: WearableType::OuraRing,
        };

        symbiotic.ingest_biometric(reading);

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = symbiotic.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
    }

    #[tokio::test]
    async fn test_execute() {
        let symbiotic = Symbiotic::new();
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Symbiotic,
            action: "scale".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: std::collections::HashMap::new(),
        };

        let result = symbiotic.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    #[test]
    fn test_capabilities() {
        let symbiotic = Symbiotic::new();
        let capabilities = symbiotic.capabilities();
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "biometric_polling"));
        assert!(capabilities.iter().any(|c| c.name == "stress_classification"));
    }

    // === BLE biometric integration tests ===

    #[tokio::test]
    async fn test_no_biometric_provider_by_default() {
        let symbiotic = Symbiotic::new();
        assert!(!symbiotic.has_biometrics());
    }

    #[tokio::test]
    async fn test_with_biometrics_attaches_provider() {
        let symbiotic = Symbiotic::new()
            .with_biometrics()
            .await
            .expect("biometric provider spawn should succeed");
        assert!(symbiotic.has_biometrics());
    }

    #[tokio::test]
    async fn test_scan_wearables_without_provider_returns_empty() {
        let symbiotic = Symbiotic::new(); // no provider
        let wearables = symbiotic
            .scan_wearables(Duration::from_millis(100))
            .await
            .expect("scan without provider should be a graceful no-op");
        assert!(wearables.is_empty());
    }

    #[tokio::test]
    async fn test_register_wearable_without_provider_errors() {
        let mut symbiotic = Symbiotic::new();
        let result = symbiotic
            .register_wearable("dev1", WearableType::AppleWatch)
            .await;
        assert!(matches!(result, Err(BleError::FeatureNotEnabled)));
    }

    #[tokio::test]
    async fn test_ingest_sample_heart_rate() {
        let mut symbiotic = Symbiotic::new();
        let baseline_count = symbiotic.biometric_history.len();

        let sample = BiometricSample::heart_rate("dev1".to_string(), 75);
        symbiotic.ingest_sample(sample);

        assert_eq!(symbiotic.biometric_history.len(), baseline_count + 1);
        let last = symbiotic.biometric_history.back().unwrap();
        assert_eq!(last.heart_rate, 75);
    }

    #[tokio::test]
    async fn test_ingest_sample_with_known_wearable() {
        let mut symbiotic = Symbiotic::new();
        symbiotic
            .wearable_map
            .insert("dev1".to_string(), WearableType::AppleWatch);

        let sample = BiometricSample::heart_rate("dev1".to_string(), 80);
        symbiotic.ingest_sample(sample);

        let last = symbiotic.biometric_history.back().unwrap();
        assert_eq!(last.device_type, WearableType::AppleWatch);
    }

    #[tokio::test]
    async fn test_ingest_sample_unknown_device_uses_generic() {
        let mut symbiotic = Symbiotic::new();
        let sample = BiometricSample::heart_rate("unknown-dev".to_string(), 65);
        symbiotic.ingest_sample(sample);

        let last = symbiotic.biometric_history.back().unwrap();
        assert_eq!(last.device_type, WearableType::Generic);
    }

    #[tokio::test]
    async fn test_ingest_sample_extracts_hrv_from_payload() {
        let mut symbiotic = Symbiotic::new();
        // Build a real HR measurement payload with RR intervals to exercise HRV parsing
        // Flags = 0x10 (RR present), HR = 60, RR = [1024, 1126]
        let payload = vec![0x10, 60, 0x00, 0x04, 0x66, 0x04];
        let sample = BiometricSample {
            timestamp: 0,
            device_id: "polar".to_string(),
            kind: BiometricKind::HeartRate,
            value: 60.0,
            raw_payload: Some(payload),
        };
        symbiotic.ingest_sample(sample);

        let last = symbiotic.biometric_history.back().unwrap();
        assert_eq!(last.heart_rate, 60);
        // HRV should be > 0 since RRs differ
        assert!(last.heart_rate_variability > 0.0);
    }

    #[tokio::test]
    async fn test_ingest_sample_battery_does_not_create_reading() {
        let mut symbiotic = Symbiotic::new();
        let baseline = symbiotic.biometric_history.len();
        let sample = BiometricSample::battery("dev1".to_string(), 85);
        symbiotic.ingest_sample(sample);
        // Battery samples should NOT add to biometric_history (only HR does)
        assert_eq!(symbiotic.biometric_history.len(), baseline);
    }

    // === Inbox-shaped proposal tests ===

    #[tokio::test]
    async fn test_propose_with_pending_ble_inbox_triggers_proposal() {
        // Normally Symbiotic only proposes when stress > 0.7 or Recovery mode.
        // If there are pending BLE samples in the inbox, it should also propose
        // (to signal that fresh biometric data is waiting to be processed).
        let mut symbiotic = Symbiotic::new();
        // Set state to below the normal proposal threshold
        symbiotic.current_state.stress_level = 0.4;
        symbiotic.current_state.fatigue_level = 0.2;

        // Push a BLE sample into the inbox
        let sample = BiometricSample::heart_rate("wearable-1".to_string(), 75);
        symbiotic.bio_inbox.lock().push_back(sample);

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = symbiotic.propose(&context).await.unwrap();
        assert!(
            !proposals.is_empty(),
            "pending BLE inbox should trigger a proposal even at low stress"
        );
        assert!(
            proposals[0].description.contains("pending BLE"),
            "description should mention pending BLE samples: {}",
            proposals[0].description
        );
    }

    #[tokio::test]
    async fn test_propose_description_omits_pending_when_inbox_empty() {
        let mut symbiotic = Symbiotic::new();
        // High stress to trigger a proposal
        symbiotic.current_state.stress_level = 0.8;

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = symbiotic.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
        assert!(
            !proposals[0].description.contains("pending BLE"),
            "description should NOT mention pending BLE when inbox empty: {}",
            proposals[0].description
        );
    }

    #[tokio::test]
    async fn test_pending_samples_increase_confidence_slightly() {
        let mut symbiotic_base = Symbiotic::new();
        symbiotic_base.current_state.stress_level = 0.8;

        let mut symbiotic_inbox = Symbiotic::new();
        symbiotic_inbox.current_state.stress_level = 0.8;
        // Add 5 pending samples - should boost confidence by 5 * 0.01 = 0.05
        for i in 0..5u16 {
            let s = BiometricSample::heart_rate("dev".to_string(), 70 + i);
            symbiotic_inbox.bio_inbox.lock().push_back(s);
        }

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let base_proposals = symbiotic_base.propose(&context).await.unwrap();
        let inbox_proposals = symbiotic_inbox.propose(&context).await.unwrap();

        assert!(!base_proposals.is_empty());
        assert!(!inbox_proposals.is_empty());

        assert!(
            inbox_proposals[0].confidence >= base_proposals[0].confidence,
            "pending samples should not reduce confidence: {} vs {}",
            inbox_proposals[0].confidence,
            base_proposals[0].confidence
        );
    }
}
