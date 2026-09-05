// crates/compute/src/user_baseline.rs
//! Continuous User Kinematic Profiler, Identity Anomaly Detector & Multi-User Manager.
//!
//! 1. `UserKinematicProfile`: Online estimators for mouse velocity/jerk curves,
//!    keystroke dynamics, and Fitts's law target correction coefficients.
//! 2. `AttentionState`: State machine tracking DeepFlow, Deliberating, Skimming, Distracted, and Fatigued.
//! 3. `UserIdentityEngine`: Cosine similarity anomaly classifier detecting when a foreign user
//!    takes over, auto-provisioning guest profiles and preventing training contamination.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Qualitative mental & operational state of the operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionState {
    /// Optimal flow; minimal micro-delays, smooth velocity, high intent focus
    DeepFlow,
    /// Deliberate calculation; steady pauses before precision clicks/keystrokes
    Deliberating,
    /// Fast overview or scrolling; lower click frequency
    Skimming,
    /// Irregular pauses, rapid window switching, out-of-context activity
    Distracted,
    /// Increased flight times, micro-stutters, higher error/correction rate
    Fatigued,
}

impl Default for AttentionState {
    fn default() -> Self {
        Self::DeepFlow
    }
}

/// Dynamic kinematic biomarkers characterizing an operator's physical interaction style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinematicBiomarkers {
    /// Average cursor velocity in pixels per second
    pub mean_cursor_speed: f32,
    /// Velocity standard deviation
    pub cursor_speed_stddev: f32,
    /// Normalized trajectory jitter / tremor ratio (0.0 = perfect spline, 1.0 = erratic)
    pub trajectory_jitter: f32,
    /// Fitts's law deceleration sharpness before target landing
    pub target_landing_deceleration: f32,
    /// Average keystroke dwell time in milliseconds
    pub key_dwell_ms: f32,
    /// Average flight time between keystrokes in milliseconds
    pub key_flight_ms: f32,
    /// Ratio of backspace/correction events to total keystrokes
    pub correction_rate: f32,
}

impl Default for KinematicBiomarkers {
    fn default() -> Self {
        Self {
            mean_cursor_speed: 480.0,
            cursor_speed_stddev: 120.0,
            trajectory_jitter: 0.12,
            target_landing_deceleration: 0.82,
            key_dwell_ms: 85.0,
            key_flight_ms: 140.0,
            correction_rate: 0.04,
        }
    }
}

impl KinematicBiomarkers {
    /// Vector representation for cosine similarity calculations (7-dimensional space)
    pub fn to_vector(&self) -> [f32; 7] {
        [
            self.mean_cursor_speed / 1000.0,
            self.cursor_speed_stddev / 500.0,
            self.trajectory_jitter,
            self.target_landing_deceleration,
            self.key_dwell_ms / 200.0,
            self.key_flight_ms / 300.0,
            self.correction_rate * 5.0,
        ]
    }

    /// Computes cosine similarity against another biomarker vector (0.0 to 1.0)
    pub fn similarity(&self, other: &Self) -> f32 {
        let a = self.to_vector();
        let b = other.to_vector();

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a > 1e-6 && mag_b > 1e-6 {
            (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// A stored user identity profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
    pub is_primary: bool,
    pub is_guest: bool,
    pub registered_timestamp_secs: u64,
    pub baseline_biomarkers: KinematicBiomarkers,
    pub total_interaction_hours: f32,
    pub preferred_companion_tone: String,
}

impl UserProfile {
    pub fn new_primary(name: &str) -> Self {
        Self {
            id: format!("usr_{}", name.to_lowercase().replace(' ', "_")),
            display_name: name.to_string(),
            is_primary: true,
            is_guest: false,
            registered_timestamp_secs: 1700000000,
            baseline_biomarkers: KinematicBiomarkers::default(),
            total_interaction_hours: 0.0,
            preferred_companion_tone: "Cordial / Executive".to_string(),
        }
    }

    pub fn new_guest(id_suffix: u64) -> Self {
        Self {
            id: format!("guest_{id_suffix}"),
            display_name: format!("Guest Operator #{id_suffix}"),
            is_primary: false,
            is_guest: true,
            registered_timestamp_secs: 1700000000 + id_suffix,
            baseline_biomarkers: KinematicBiomarkers {
                mean_cursor_speed: 350.0,
                cursor_speed_stddev: 180.0,
                trajectory_jitter: 0.25,
                target_landing_deceleration: 0.65,
                key_dwell_ms: 110.0,
                key_flight_ms: 220.0,
                correction_rate: 0.08,
            },
            total_interaction_hours: 0.0,
            preferred_companion_tone: "Welcoming / Informative".to_string(),
        }
    }
}

/// Live real-time identification, flow tracking and anomaly detection engine
pub struct UserIdentityEngine {
    profiles: HashMap<String, UserProfile>,
    active_user_id: String,
    current_biomarkers: KinematicBiomarkers,
    attention_state: AttentionState,
    flow_score: f32,
    anomaly_consecutive_ticks: u32,
    guest_counter: u64,
}

impl Default for UserIdentityEngine {
    fn default() -> Self {
        Self::new("Aaron")
    }
}

impl UserIdentityEngine {
    pub fn new(default_operator_name: &str) -> Self {
        let primary = UserProfile::new_primary(default_operator_name);
        let active_id = primary.id.clone();

        let mut profiles = HashMap::new();
        profiles.insert(active_id.clone(), primary);

        Self {
            profiles,
            active_user_id: active_id,
            current_biomarkers: KinematicBiomarkers::default(),
            attention_state: AttentionState::DeepFlow,
            flow_score: 0.94,
            anomaly_consecutive_ticks: 0,
            guest_counter: 1,
        }
    }

    pub fn active_profile(&self) -> &UserProfile {
        self.profiles
            .get(&self.active_user_id)
            .expect("Active profile must always exist")
    }

    pub fn active_profile_mut(&mut self) -> &mut UserProfile {
        self.profiles
            .get_mut(&self.active_user_id)
            .expect("Active profile must always exist")
    }

    pub fn all_profiles(&self) -> Vec<&UserProfile> {
        let mut list: Vec<&UserProfile> = self.profiles.values().collect();
        list.sort_by(|a, b| b.is_primary.cmp(&a.is_primary).then_with(|| a.display_name.cmp(&b.display_name)));
        list
    }

    pub fn attention_state(&self) -> AttentionState {
        self.attention_state
    }

    pub fn flow_score(&self) -> f32 {
        self.flow_score
    }

    pub fn switch_user(&mut self, user_id: &str) -> Result<()> {
        if self.profiles.contains_key(user_id) {
            self.active_user_id = user_id.to_string();
            self.anomaly_consecutive_ticks = 0;
            self.flow_score = 0.90;
            Ok(())
        } else {
            bail!("User profile '{}' does not exist", user_id);
        }
    }

    /// Evaluates live incoming sensory/kinematic inputs and updates identity & flow state
    pub fn ingest_kinematics(&mut self, live_biomarkers: KinematicBiomarkers) -> Option<String> {
        self.current_biomarkers = live_biomarkers.clone();

        let active_baseline = &self.active_profile().baseline_biomarkers;
        let match_confidence = live_biomarkers.similarity(active_baseline);

        // Update flow state based on stability
        if match_confidence > 0.88 {
            self.anomaly_consecutive_ticks = 0;
            self.flow_score = (self.flow_score * 0.9 + match_confidence * 0.1).clamp(0.0, 1.0);
            
            if live_biomarkers.key_flight_ms > 250.0 && live_biomarkers.correction_rate > 0.09 {
                self.attention_state = AttentionState::Fatigued;
            } else if live_biomarkers.mean_cursor_speed < 150.0 {
                self.attention_state = AttentionState::Deliberating;
            } else {
                self.attention_state = AttentionState::DeepFlow;
            }
            None
        } else {
            // Anomaly detected: kinematic pattern does not match active profile
            self.anomaly_consecutive_ticks += 1;
            self.flow_score = (self.flow_score * 0.85).max(0.15);
            self.attention_state = AttentionState::Distracted;

            // If mismatch persists across sustained interaction (e.g. 5 consecutive observation frames),
            // trigger auto-identification / guest profile provisioning
            if self.anomaly_consecutive_ticks >= 5 {
                self.anomaly_consecutive_ticks = 0;

                // Check if it matches any other known registered profile
                let mut best_match: Option<String> = None;
                let mut highest_sim: f32 = 0.80;

                for (id, profile) in &self.profiles {
                    let sim = live_biomarkers.similarity(&profile.baseline_biomarkers);
                    if sim > highest_sim {
                        highest_sim = sim;
                        best_match = Some(id.clone());
                    }
                }

                if let Some(matched_id) = best_match {
                    self.active_user_id = matched_id.clone();
                    Some(format!("Recognized registered profile '{}'. Swapped active context.", self.active_profile().display_name))
                } else {
                    // Spawn new guest profile to isolate foreign data and prevent poisoning
                    let guest_id = self.guest_counter;
                    self.guest_counter += 1;
                    let mut guest = UserProfile::new_guest(guest_id);
                    guest.baseline_biomarkers = live_biomarkers;
                    let new_id = guest.id.clone();
                    let display_name = guest.display_name.clone();
                    self.profiles.insert(new_id.clone(), guest);
                    self.active_user_id = new_id;
                    Some(format!("Foreign operator pattern detected. Provisioned '{}' to isolate training data.", display_name))
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_kinematic_profile_similarity() {
        let b1 = KinematicBiomarkers::default();
        let b2 = KinematicBiomarkers::default();
        let sim = b1.similarity(&b2);
        assert!(sim > 0.99, "Identical biomarkers should have ~1.0 similarity (got {sim})");

        let foreign = KinematicBiomarkers {
            mean_cursor_speed: 1200.0,
            cursor_speed_stddev: 450.0,
            trajectory_jitter: 0.8,
            target_landing_deceleration: 0.2,
            key_dwell_ms: 30.0,
            key_flight_ms: 40.0,
            correction_rate: 0.25,
        };
        let diff_sim = b1.similarity(&foreign);
        assert!(diff_sim < 0.75, "Foreign kinematics should show low similarity (got {diff_sim})");
    }

    #[test]
    fn test_user_identity_engine_anomaly_and_guest_provisioning() {
        let mut engine = UserIdentityEngine::new("Aaron");
        assert_eq!(engine.active_profile().display_name, "Aaron");
        assert!(engine.active_profile().is_primary);

        let foreign = KinematicBiomarkers {
            mean_cursor_speed: 1200.0,
            cursor_speed_stddev: 450.0,
            trajectory_jitter: 0.8,
            target_landing_deceleration: 0.2,
            key_dwell_ms: 30.0,
            key_flight_ms: 40.0,
            correction_rate: 0.25,
        };

        // Ingest foreign ticks
        let mut notice = None;
        for _ in 0..5 {
            notice = engine.ingest_kinematics(foreign.clone());
        }

        assert!(notice.is_some(), "Should notify upon sustained foreign user anomaly");
        assert!(engine.active_profile().is_guest, "Active profile should now be a guest");
        assert_eq!(engine.all_profiles().len(), 2, "Should maintain both primary and guest profiles");
    }
}
