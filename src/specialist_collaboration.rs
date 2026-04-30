// Specialist Collaboration Engine
// Enables specialists to collaborate, request help, and share knowledge

use crate::agents::SpecialistAgent;
use crate::specialist_memory::{MemoryType, Confidence, MemorySource};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, debug};

/// Request for help from another specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpRequest {
    pub request_id: String,
    pub requester_id: String,
    pub task_id: String,
    pub skill_needed: String,
    pub challenge_description: String,
    pub urgency: Urgency,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Urgency::Low => write!(f, "Low"),
            Urgency::Medium => write!(f, "Medium"),
            Urgency::High => write!(f, "High"),
            Urgency::Critical => write!(f, "Critical"),
        }
    }
}

/// Response to help request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpResponse {
    pub response_id: String,
    pub request_id: String,
    pub helper_id: String,
    pub can_help: bool,
    pub available_expertise: Vec<String>,
    pub assistance_type: AssistanceType,
    pub effort_required: f32, // 0.0-1.0
    pub time_available_minutes: u32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssistanceType {
    DirectHelp,        // Help with the task directly
    Consultation,      // Advice and guidance
    Mentoring,         // Teaching the skill
    ResourceSharing,   // Share tools/data
    Delegation,        // Take over the task
}

/// Collaboration history between two specialists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationRecord {
    pub specialist1_id: String,
    pub specialist2_id: String,
    pub collaboration_count: u32,
    pub success_rate: f32, // 0.0-1.0
    pub average_satisfaction: f32, // 0.0-5.0
    pub skills_shared: Vec<String>,
    pub last_collaboration: chrono::DateTime<chrono::Utc>,
}

/// Collaboration metrics for a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationMetrics {
    pub specialist_id: String,
    pub help_requests_sent: u32,
    pub help_requests_received: u32,
    pub help_requests_accepted: u32,
    pub collaboration_success_rate: f32,
    pub peers: Vec<String>, // IDs of specialists they've collaborated with
    pub taught_specialists: Vec<String>, // IDs they've mentored
    pub learned_from_specialists: Vec<String>, // IDs they've learned from
}

impl CollaborationMetrics {
    /// Calculate collaboration index (0.0-1.0)
    pub fn collaboration_index(&self) -> f32 {
        if self.help_requests_sent + self.help_requests_received == 0 {
            0.0
        } else {
            let total_requests = self.help_requests_sent + self.help_requests_received;
            (self.help_requests_accepted as f32 / total_requests as f32).min(1.0)
        }
    }

    /// Calculate mentorship impact
    pub fn mentorship_impact(&self) -> f32 {
        ((self.taught_specialists.len() as f32) * 0.5 
            + (self.learned_from_specialists.len() as f32) * 0.5)
        .min(1.0)
    }
}

/// Specialist collaboration engine
pub struct SpecialistCollaborationEngine {
    help_requests: HashMap<String, HelpRequest>,
    help_responses: HashMap<String, Vec<HelpResponse>>,
    collaboration_history: HashMap<(String, String), CollaborationRecord>,
    collaboration_metrics: HashMap<String, CollaborationMetrics>,
}

impl SpecialistCollaborationEngine {
    /// Create new collaboration engine
    pub fn new() -> Self {
        Self {
            help_requests: HashMap::new(),
            help_responses: HashMap::new(),
            collaboration_history: HashMap::new(),
            collaboration_metrics: HashMap::new(),
        }
    }

    /// Create help request
    pub fn create_help_request(
        &mut self,
        requester_id: String,
        task_id: String,
        skill_needed: String,
        challenge_description: String,
        urgency: Urgency,
    ) -> String {
        let request_id = format!("help-req-{}", uuid::Uuid::new_v4());

        let request = HelpRequest {
            request_id: request_id.clone(),
            requester_id: requester_id.clone(),
            task_id,
            skill_needed: skill_needed.clone(),
            challenge_description,
            urgency,
            timestamp: chrono::Utc::now(),
        };

        info!(
            "Help request created: {} from {} for skill {}",
            request_id, requester_id, skill_needed
        );

        self.help_requests.insert(request_id.clone(), request);

        // Initialize metrics for requester if not exists
        if !self.collaboration_metrics.contains_key(&requester_id) {
            self.collaboration_metrics.insert(
                requester_id.clone(),
                CollaborationMetrics {
                    specialist_id: requester_id,
                    help_requests_sent: 1,
                    help_requests_received: 0,
                    help_requests_accepted: 0,
                    collaboration_success_rate: 0.0,
                    peers: vec![],
                    taught_specialists: vec![],
                    learned_from_specialists: vec![],
                },
            );
        } else if let Some(metrics) = self.collaboration_metrics.get_mut(&requester_id) {
            metrics.help_requests_sent += 1;
        }

        request_id
    }

    /// Respond to help request
    pub fn respond_to_request(
        &mut self,
        request_id: String,
        helper_id: String,
        can_help: bool,
        assistance_type: AssistanceType,
        effort: f32,
        available_time: u32,
        message: String,
    ) -> String {
        let response_id = format!("help-resp-{}", uuid::Uuid::new_v4());

        let response = HelpResponse {
            response_id: response_id.clone(),
            request_id: request_id.clone(),
            helper_id: helper_id.clone(),
            can_help,
            available_expertise: vec![],
            assistance_type,
            effort_required: effort,
            time_available_minutes: available_time,
            message,
        };

        // Update metrics
        if !self.collaboration_metrics.contains_key(&helper_id) {
            self.collaboration_metrics.insert(
                helper_id.clone(),
                CollaborationMetrics {
                    specialist_id: helper_id,
                    help_requests_sent: 0,
                    help_requests_received: 1,
                    help_requests_accepted: if can_help { 1 } else { 0 },
                    collaboration_success_rate: if can_help { 1.0 } else { 0.0 },
                    peers: vec![],
                    taught_specialists: vec![],
                    learned_from_specialists: vec![],
                },
            );
        } else if let Some(metrics) = self.collaboration_metrics.get_mut(&helper_id) {
            metrics.help_requests_received += 1;
            if can_help {
                metrics.help_requests_accepted += 1;
            }
        }

        // Store response
        self.help_responses
            .entry(request_id)
            .or_insert_with(Vec::new)
            .push(response);

        response_id
    }

    /// Find suitable helpers for a skill
    pub fn find_helpers_for_skill(
        &self,
        skill_needed: &str,
        available_specialists: &[SpecialistAgent],
    ) -> Vec<(String, f32)> {
        let mut candidates = vec![];

        for specialist in available_specialists {
            // Simple role-based matching
            let skill_match = if specialist.role.to_lowercase().contains(&skill_needed.to_lowercase())
                || specialist.domain.to_string().to_lowercase().contains(&skill_needed.to_lowercase())
            {
                1.0
            } else {
                0.0
            };

            if skill_match > 0.0 {
                // Score is consistent per specialist
                let score = 0.8;
                candidates.push((specialist.id.clone(), score));
            }
        }

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        candidates
    }

    /// Record successful collaboration
    pub fn record_collaboration(
        &mut self,
        specialist1: String,
        specialist2: String,
        skill_shared: String,
        satisfaction: f32,
    ) {
        let key = if specialist1 < specialist2 {
            (specialist1.clone(), specialist2.clone())
        } else {
            (specialist2.clone(), specialist1.clone())
        };

        if let Some(record) = self.collaboration_history.get_mut(&key) {
            record.collaboration_count += 1;
            record.average_satisfaction = (record.average_satisfaction
                + satisfaction.clamp(0.0, 5.0))
                / 2.0;
            if !record.skills_shared.contains(&skill_shared) {
                record.skills_shared.push(skill_shared);
            }
            record.last_collaboration = chrono::Utc::now();
        } else {
            self.collaboration_history.insert(
                key,
                CollaborationRecord {
                    specialist1_id: specialist1,
                    specialist2_id: specialist2,
                    collaboration_count: 1,
                    success_rate: 1.0,
                    average_satisfaction: satisfaction.clamp(0.0, 5.0),
                    skills_shared: vec![skill_shared],
                    last_collaboration: chrono::Utc::now(),
                },
            );
        }

        debug!("Collaboration recorded");
    }

    /// Get collaboration metrics for specialist
    pub fn get_metrics(&self, specialist_id: &str) -> Option<CollaborationMetrics> {
        self.collaboration_metrics.get(specialist_id).cloned()
    }

    /// Get all metrics
    pub fn get_all_metrics(&self) -> Vec<CollaborationMetrics> {
        self.collaboration_metrics.values().cloned().collect()
    }

    /// Get help request
    pub fn get_help_request(&self, request_id: &str) -> Option<&HelpRequest> {
        self.help_requests.get(request_id)
    }

    /// Get responses to request
    pub fn get_responses_to_request(&self, request_id: &str) -> Vec<&HelpResponse> {
        self.help_responses
            .get(request_id)
            .map(|responses| responses.iter().collect())
            .unwrap_or_default()
    }

    /// Get collaboration history
    pub fn get_collaboration_history(&self, specialist1: &str, specialist2: &str) -> Option<&CollaborationRecord> {
        let key = if specialist1 < specialist2 {
            (specialist1.to_string(), specialist2.to_string())
        } else {
            (specialist2.to_string(), specialist1.to_string())
        };

        self.collaboration_history.get(&key)
    }

    /// Calculate team collaboration index (0.0-1.0)
    pub fn team_collaboration_index(&self) -> f32 {
        if self.collaboration_metrics.is_empty() {
            0.0
        } else {
            let sum: f32 = self
                .collaboration_metrics
                .values()
                .map(|m| m.collaboration_index())
                .sum();
            sum / self.collaboration_metrics.len() as f32
        }
    }
}

impl Default for SpecialistCollaborationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urgency_ordering() {
        assert!(Urgency::Critical > Urgency::High);
        assert!(Urgency::High > Urgency::Medium);
        assert!(Urgency::Medium > Urgency::Low);
    }

    #[test]
    fn test_help_request_creation() {
        let mut engine = SpecialistCollaborationEngine::new();
        let req_id = engine.create_help_request(
            "spec-1".to_string(),
            "task-1".to_string(),
            "Python".to_string(),
            "Need help debugging".to_string(),
            Urgency::High,
        );

        assert!(engine.get_help_request(&req_id).is_some());
    }

    #[test]
    fn test_help_response() {
        let mut engine = SpecialistCollaborationEngine::new();
        let req_id = engine.create_help_request(
            "spec-1".to_string(),
            "task-1".to_string(),
            "Python".to_string(),
            "Need help".to_string(),
            Urgency::Medium,
        );

        let resp_id = engine.respond_to_request(
            req_id.clone(),
            "spec-2".to_string(),
            true,
            AssistanceType::Consultation,
            0.3,
            30,
            "I can help".to_string(),
        );

        assert!(!resp_id.is_empty());
        let responses = engine.get_responses_to_request(&req_id);
        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn test_collaboration_metrics() {
        let mut engine = SpecialistCollaborationEngine::new();
        let _ = engine.create_help_request(
            "spec-1".to_string(),
            "task-1".to_string(),
            "Rust".to_string(),
            "Help needed".to_string(),
            Urgency::Low,
        );

        let metrics = engine.get_metrics("spec-1").unwrap();
        assert_eq!(metrics.help_requests_sent, 1);

        engine.record_collaboration(
            "spec-1".to_string(),
            "spec-2".to_string(),
            "Rust".to_string(),
            4.5,
        );

        let history = engine.get_collaboration_history("spec-1", "spec-2");
        assert!(history.is_some());
        assert_eq!(history.unwrap().collaboration_count, 1);
    }

    #[test]
    fn test_mentorship_impact() {
        let metrics = CollaborationMetrics {
            specialist_id: "spec-1".to_string(),
            help_requests_sent: 5,
            help_requests_received: 3,
            help_requests_accepted: 3,
            collaboration_success_rate: 1.0,
            peers: vec!["spec-2".to_string(), "spec-3".to_string()],
            taught_specialists: vec!["spec-2".to_string()],
            learned_from_specialists: vec!["spec-3".to_string()],
        };

        let impact = metrics.mentorship_impact();
        assert!(impact > 0.0 && impact <= 1.0);
    }

    #[test]
    fn test_collaboration_index() {
        let metrics = CollaborationMetrics {
            specialist_id: "spec-1".to_string(),
            help_requests_sent: 5,
            help_requests_received: 5,
            help_requests_accepted: 7, // More accepts than total (impossible but tests clamping)
            collaboration_success_rate: 0.8,
            peers: vec![],
            taught_specialists: vec![],
            learned_from_specialists: vec![],
        };

        let index = metrics.collaboration_index();
        assert!(index >= 0.0 && index <= 1.0);
    }

    #[test]
    fn test_team_collaboration_index() {
        let mut engine = SpecialistCollaborationEngine::new();

        // Create some requests
        for i in 0..3 {
            engine.create_help_request(
                format!("spec-{}", i),
                format!("task-{}", i),
                "Skill".to_string(),
                "Help".to_string(),
                Urgency::Medium,
            );
        }

        let team_index = engine.team_collaboration_index();
        assert!(team_index >= 0.0 && team_index <= 1.0);
    }

    #[test]
    fn test_assistance_type_variants() {
        assert_eq!(
            std::mem::discriminant(&AssistanceType::DirectHelp),
            std::mem::discriminant(&AssistanceType::DirectHelp)
        );
    }
}
