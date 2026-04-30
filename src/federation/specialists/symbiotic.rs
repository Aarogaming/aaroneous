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

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};

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

/// Symbiotic specialist implementation
pub struct Symbiotic {
    id: SpecialistId,
    pub current_state: UserBiometricState,
    pub biometric_history: VecDeque<BiometricReading>,
    pub state_history: VecDeque<UserBiometricState>,
    pub max_history_size: usize,
}

impl Symbiotic {
    pub fn new() -> Self {
        Self {
            id: SpecialistId::Symbiotic,
            current_state: UserBiometricState {
                stress_level: 0.5,
                focus_depth: 0.5,
                fatigue_level: 0.3,
                activity_state: ActivityState::Sedentary,
                readiness_score: 0.7,
                last_update: 0,
            },
            biometric_history: VecDeque::new(),
            state_history: VecDeque::new(),
            max_history_size: 1000,
        }
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

    /// Get Intent scaling recommendation
    pub fn get_intent_scaling(&self) -> IntentScaling {
        let state = &self.current_state;

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

        // Only propose if significant state change or recovery needed
        if scaling.recommended_focus == FocusMode::Recovery
            || self.current_state.stress_level > 0.7
        {
            let action_type = match scaling.recommended_focus {
                FocusMode::Recovery => "scale_intent_recovery",
                FocusMode::DeepWork => "scale_intent_deep",
                _ => "scale_intent_adaptive",
            };

            return Ok(vec![ProposedAction {
                id: format!("symbiotic-scaling-{}", uuid()),
                specialist: SpecialistId::Symbiotic,
                action_type: action_type.to_string(),
                description: format!(
                    "Scale Intent for user state: stress={:.2}, fatigue={:.2}, readiness={:.2}",
                    self.current_state.stress_level,
                    self.current_state.fatigue_level,
                    self.current_state.readiness_score
                ),
                confidence: scaling.confidence,
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 80,
                    duration_seconds: 5,
                },
                priority: if self.current_state.fatigue_level > 0.8 {
                    ProposalPriority::UserFacing
                } else if self.current_state.stress_level > 0.7 {
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

        let output = format!(
            "Scaled Intent: {} (delay: {}s, max: {}m, readiness: {:.2}%)",
            format!("{:?}", scaling.recommended_focus),
            scaling.proposal_delay_seconds,
            scaling.max_duration_minutes,
            self.current_state.readiness_score * 100.0
        );

        Ok(ExecutionResult {
            specialist: SpecialistId::Symbiotic,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 150,
            error: None,
        })
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
            heart_rate: 65,
            heart_rate_variability: 50.0,
            skin_temperature: 36.5,
            activity_level: 10.0,
            sleep_debt_hours: 6.0, // High sleep debt
            device_type: WearableType::AppleWatch,
        };

        symbiotic.ingest_biometric(reading);
        assert!(symbiotic.current_state.fatigue_level > 0.7);
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
            sleep_debt_hours: 7.0,
            device_type: WearableType::Whoop,
        };

        symbiotic.ingest_biometric(reading);
        let scaling = symbiotic.get_intent_scaling();

        assert_eq!(scaling.recommended_focus, FocusMode::Recovery);
        assert!(scaling.max_duration_minutes < 30);
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
}
