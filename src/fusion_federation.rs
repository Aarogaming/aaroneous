// Aaroneous Fusion Federation Integration
// NATS publishing and cross-specialist fusion coordination
// Enables federation-wide capability broadcasting and discovery

use crate::skill_fusion::{FusionSuggestion, SkillFusionEngine};
use crate::skill_system::{SpecialistSkillSet, SkillType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Federation fusion event - published to NATS
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionEvent {
    pub event_id: String,
    pub specialist_id: String,
    pub event_type: FusionEventType,
    pub fusion_name: String,
    pub parent_skills: Vec<String>,
    pub compatibility_score: f64,
    pub power_multiplier: f64,
    pub emergent_properties: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub topic: String, // NATS topic this was published to
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FusionEventType {
    FusionSuggested,     // New fusion suggestion found
    FusionRequested,     // Specialist requested a fusion
    FusionCompleted,     // Fusion successfully executed
    FusionFailed,        // Fusion attempt failed
    FusionTaught,        // Mentor taught fusion to apprentice
    MultiSkillFusion,    // 3+ skill fusion discovered
}

impl FusionEvent {
    pub fn new(
        specialist_id: String,
        event_type: FusionEventType,
        fusion_name: String,
        parent_skills: Vec<String>,
        compat: f64,
        power: f64,
        properties: Vec<String>,
    ) -> Self {
        let topic = format!("federation.fusions.{}", specialist_id);
        Self {
            event_id: format!("fusion_evt_{}", uuid::Uuid::new_v4()),
            specialist_id,
            event_type,
            fusion_name,
            parent_skills,
            compatibility_score: compat,
            power_multiplier: power,
            emergent_properties: properties,
            timestamp: Utc::now(),
            topic,
        }
    }
}

/// Fusion capability registered in federation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionCapability {
    pub capability_id: String,
    pub specialist_id: String,
    pub fusion_name: String,
    pub skill_types_involved: Vec<SkillType>,
    pub compatibility_score: f64,
    pub power_level: f64,
    pub emergent_properties: Vec<String>,
    pub can_teach: bool,
    pub teaching_cost: u32, // XP cost to teach to apprentice
    pub discovery_date: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

impl FusionCapability {
    pub fn new(
        specialist_id: String,
        fusion_name: String,
        skill_types: Vec<SkillType>,
        compat: f64,
        power: f64,
        properties: Vec<String>,
    ) -> Self {
        Self {
            capability_id: format!("cap_{}", uuid::Uuid::new_v4()),
            specialist_id,
            fusion_name,
            skill_types_involved: skill_types,
            compatibility_score: compat,
            power_level: power,
            emergent_properties: properties,
            can_teach: false,
            teaching_cost: 200, // Base cost
            discovery_date: Utc::now(),
            last_used: None,
        }
    }
    
    /// Mark capability as teachable (Rank 4+)
    pub fn enable_teaching(&mut self) {
        self.can_teach = true;
    }
}

/// Federation fusion broadcast channel
pub struct FusionFederationBroadcaster {
    capabilities_registry: HashMap<String, FusionCapability>,
    fusion_events: Vec<FusionEvent>,
    specialist_fusions: HashMap<String, Vec<String>>, // specialist_id -> fusion_ids
}

impl FusionFederationBroadcaster {
    pub fn new() -> Self {
        Self {
            capabilities_registry: HashMap::new(),
            fusion_events: Vec::new(),
            specialist_fusions: HashMap::new(),
        }
    }
    
    /// Publish a fusion event to federation
    pub fn publish_fusion_event(&mut self, event: FusionEvent) -> String {
        let topic = event.topic.clone();
        self.fusion_events.push(event);
        topic
    }
    
    /// Register a fusion capability in federation
    pub fn register_fusion_capability(&mut self, capability: FusionCapability) -> String {
        let capability_id = capability.capability_id.clone();
        let specialist_id = capability.specialist_id.clone();
        
        self.capabilities_registry.insert(capability_id.clone(), capability);
        
        // Track fusion per specialist
        self.specialist_fusions
            .entry(specialist_id)
            .or_insert_with(Vec::new)
            .push(capability_id.clone());
        
        capability_id
    }
    
    /// Find specialists with specific skill combinations
    pub fn find_specialists_with_fusion_types(&self, skill_types: &[SkillType]) -> Vec<String> {
        self.capabilities_registry
            .values()
            .filter(|cap| {
                skill_types.iter().all(|st| cap.skill_types_involved.contains(st))
            })
            .map(|cap| cap.specialist_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
    
    /// Get all capabilities for a specialist
    pub fn get_specialist_capabilities(&self, specialist_id: &str) -> Vec<&FusionCapability> {
        if let Some(fusion_ids) = self.specialist_fusions.get(specialist_id) {
            fusion_ids
                .iter()
                .filter_map(|id| self.capabilities_registry.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Find fusion mentors (specialists who can teach a fusion)
    pub fn find_fusion_mentors(&self, fusion_name: &str) -> Vec<String> {
        self.capabilities_registry
            .values()
            .filter(|cap| cap.can_teach && cap.fusion_name.contains(fusion_name))
            .map(|cap| cap.specialist_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
    
    /// Get top-rated fusions
    pub fn get_top_fusions(&self, limit: usize) -> Vec<&FusionCapability> {
        let mut fusions: Vec<_> = self.capabilities_registry.values().collect();
        fusions.sort_by(|a, b| {
            match b.power_level.partial_cmp(&a.power_level) {
                Some(std::cmp::Ordering::Equal) => b.compatibility_score.partial_cmp(&a.compatibility_score).unwrap_or(std::cmp::Ordering::Equal),
                other => other.unwrap_or(std::cmp::Ordering::Equal),
            }
        });
        fusions.into_iter().take(limit).collect()
    }
    
    /// Get all fusion events for a specialist
    pub fn get_specialist_fusion_history(&self, specialist_id: &str) -> Vec<&FusionEvent> {
        self.fusion_events
            .iter()
            .filter(|e| e.specialist_id == specialist_id)
            .collect()
    }
    
    /// Query fusions by minimum compatibility
    pub fn query_fusions_by_compatibility(&self, min_compat: f64) -> Vec<&FusionCapability> {
        self.capabilities_registry
            .values()
            .filter(|cap| cap.compatibility_score >= min_compat)
            .collect()
    }
    
    /// Query fusions by skill type combination
    pub fn query_fusions_by_types(&self, skill_types: &[SkillType]) -> Vec<&FusionCapability> {
        self.capabilities_registry
            .values()
            .filter(|cap| {
                for skill_type in skill_types {
                    if !cap.skill_types_involved.contains(skill_type) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
}

/// Fusion Query API for cross-specialist capability discovery
pub struct FusionQueryAPI {
    broadcaster: FusionFederationBroadcaster,
    engine: SkillFusionEngine,
}

impl FusionQueryAPI {
    pub fn new(broadcaster: FusionFederationBroadcaster, engine: SkillFusionEngine) -> Self {
        Self {
            broadcaster,
            engine,
        }
    }
    
    /// Find optimal fusion partners in federation
    pub fn find_fusion_partners(
        &self,
        specialist_id: &str,
        skill_id: &str,
        skillset: &SpecialistSkillSet,
    ) -> Vec<(String, String, f64)> {
        let mut partners = Vec::new();
        
        if let Some(skill) = skillset.skills.get(skill_id) {
            // Find specialists with compatible skills
            let compatible_types = match skill.skill_type {
                SkillType::DAG => vec![SkillType::RAG, SkillType::MCP, SkillType::API],
                SkillType::RAG => vec![SkillType::DAG, SkillType::MCP, SkillType::API],
                SkillType::MCP => vec![SkillType::DAG, SkillType::RAG, SkillType::API],
                SkillType::API => vec![SkillType::DAG, SkillType::RAG, SkillType::MCP],
                _ => vec![],
            };
            
            for skill_type in compatible_types {
                let specialists = self.broadcaster.find_specialists_with_fusion_types(&[skill.skill_type, skill_type]);
                for other_specialist in specialists {
                    if other_specialist != specialist_id {
                        // Get this specialist's capabilities
                        let capabilities = self.broadcaster.get_specialist_capabilities(&other_specialist);
                        for cap in capabilities {
                            if cap.skill_types_involved.contains(&skill.skill_type) {
                                partners.push((
                                    other_specialist.clone(),
                                    cap.fusion_name.clone(),
                                    cap.compatibility_score,
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by compatibility
        partners.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        partners
    }
    
    /// Get recommended fusions for specialist
    pub fn get_recommendations(&self, skillset: &SpecialistSkillSet) -> Vec<FusionSuggestion> {
        self.engine.discover_fusions(skillset)
    }
    
    /// Check if specialist can teach fusion
    pub fn can_teach_fusion(&self, mentor_id: &str, mentor_set: &SpecialistSkillSet, fusion_name: &str) -> bool {
        let mentors = self.broadcaster.find_fusion_mentors(fusion_name);
        mentors.contains(&mentor_id.to_string()) && self.engine.can_mentor_fusion(mentor_set, mentor_set)
    }
    
    /// Get teaching cost for fusion
    pub fn get_teaching_cost(&self, fusion_name: &str) -> Option<u32> {
        self.broadcaster
            .capabilities_registry
            .values()
            .find(|cap| cap.fusion_name == fusion_name && cap.can_teach)
            .map(|cap| cap.teaching_cost)
    }
    
    /// Broadcast capability to federation
    pub fn broadcast_fusion_capability(
        &mut self,
        specialist_id: String,
        fusion_name: String,
        skill_types: Vec<SkillType>,
        compat: f64,
        power: f64,
        properties: Vec<String>,
    ) -> String {
        let capability = FusionCapability::new(
            specialist_id,
            fusion_name,
            skill_types,
            compat,
            power,
            properties,
        );
        
        self.broadcaster.register_fusion_capability(capability)
    }
}

/// Mentorship fusion transfer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionMentorshipTransfer {
    pub transfer_id: String,
    pub mentor_id: String,
    pub apprentice_id: String,
    pub fusion_capability_id: String,
    pub fusion_name: String,
    pub xp_cost: u32,
    pub teaching_progress: f64, // 0.0-1.0
    pub status: MentorshipStatus,
    pub started: DateTime<Utc>,
    pub completed: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum MentorshipStatus {
    Proposed,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl FusionMentorshipTransfer {
    pub fn new(
        mentor_id: String,
        apprentice_id: String,
        capability_id: String,
        fusion_name: String,
        xp_cost: u32,
    ) -> Self {
        Self {
            transfer_id: format!("mentor_fusion_{}", uuid::Uuid::new_v4()),
            mentor_id,
            apprentice_id,
            fusion_capability_id: capability_id,
            fusion_name,
            xp_cost,
            teaching_progress: 0.0,
            status: MentorshipStatus::Proposed,
            started: Utc::now(),
            completed: None,
        }
    }
    
    /// Start mentorship
    pub fn start(&mut self) {
        self.status = MentorshipStatus::InProgress;
    }
    
    /// Update progress
    pub fn update_progress(&mut self, delta: f64) {
        self.teaching_progress = (self.teaching_progress + delta).min(1.0);
        
        if self.teaching_progress >= 1.0 {
            self.status = MentorshipStatus::Completed;
            self.completed = Some(Utc::now());
        }
    }
    
    /// Mark as failed
    pub fn mark_failed(&mut self) {
        self.status = MentorshipStatus::Failed;
        self.completed = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fusion_event_creation() {
        let event = FusionEvent::new(
            "specialist_1".to_string(),
            FusionEventType::FusionCompleted,
            "Adaptive Strategy".to_string(),
            vec!["skill1".to_string(), "skill2".to_string()],
            0.85,
            2.0,
            vec!["Emergent Property 1".to_string()],
        );
        
        assert_eq!(event.specialist_id, "specialist_1");
        assert!(event.event_id.starts_with("fusion_evt_"));
    }
    
    #[test]
    fn test_fusion_capability_registration() {
        let mut broadcaster = FusionFederationBroadcaster::new();
        
        let cap = FusionCapability::new(
            "specialist_1".to_string(),
            "Test Fusion".to_string(),
            vec![SkillType::DAG, SkillType::RAG],
            0.9,
            2.5,
            vec!["prop1".to_string()],
        );
        
        let cap_id = broadcaster.register_fusion_capability(cap);
        assert!(!cap_id.is_empty());
        
        let capabilities = broadcaster.get_specialist_capabilities("specialist_1");
        assert_eq!(capabilities.len(), 1);
    }
    
    #[test]
    fn test_fusion_capability_query() {
        let mut broadcaster = FusionFederationBroadcaster::new();
        
        let cap1 = FusionCapability::new(
            "spec1".to_string(),
            "Fusion1".to_string(),
            vec![SkillType::DAG, SkillType::RAG],
            0.9,
            2.5,
            vec![],
        );
        
        let cap2 = FusionCapability::new(
            "spec2".to_string(),
            "Fusion2".to_string(),
            vec![SkillType::MCP, SkillType::API],
            0.7,
            1.8,
            vec![],
        );
        
        broadcaster.register_fusion_capability(cap1);
        broadcaster.register_fusion_capability(cap2);
        
        let high_compat = broadcaster.query_fusions_by_compatibility(0.85);
        assert_eq!(high_compat.len(), 1);
    }
    
    #[test]
    fn test_mentorship_transfer() {
        let mut transfer = FusionMentorshipTransfer::new(
            "mentor_1".to_string(),
            "apprentice_1".to_string(),
            "cap_1".to_string(),
            "Fusion Name".to_string(),
            200,
        );
        
        transfer.start();
        assert_eq!(transfer.status, MentorshipStatus::InProgress);
        
        transfer.update_progress(0.5);
        assert_eq!(transfer.teaching_progress, 0.5);
        
        transfer.update_progress(0.5);
        assert_eq!(transfer.status, MentorshipStatus::Completed);
    }
}
