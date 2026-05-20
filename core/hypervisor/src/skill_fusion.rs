// Aaroneous Skill Fusion System
// Advanced fusion mechanics with compatibility scoring, discovery, and federation integration
// Supports skill combination, emergent properties, and cross-specialist capabilities

use crate::skill_system::{Skill, SkillType, FusedSkill, SpecialistSkillSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Compatibility score between two skills (0.0-1.0)
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CompatibilityScore {
    pub semantic_affinity: f64,      // How well concepts align
    pub power_synergy: f64,          // How much power combines
    pub emergence_potential: f64,    // Likelihood of new properties
    pub overall_score: f64,          // Weighted average
}

impl CompatibilityScore {
    /// Calculate overall compatibility from components
    pub fn calculate(semantic: f64, synergy: f64, emergence: f64) -> Self {
        let overall = (semantic * 0.4) + (synergy * 0.35) + (emergence * 0.25);
        Self {
            semantic_affinity: semantic.clamp(0.0, 1.0),
            power_synergy: synergy.clamp(0.0, 1.0),
            emergence_potential: emergence.clamp(0.0, 1.0),
            overall_score: overall.clamp(0.0, 1.0),
        }
    }
    
    /// Check if compatibility is sufficient for fusion (minimum 0.6)
    pub fn is_viable(&self) -> bool {
        self.overall_score >= 0.6
    }
}

/// Compatibility matrix for skill type combinations
pub struct CompatibilityMatrix {
    matrix: HashMap<(SkillType, SkillType), CompatibilityScore>,
}

impl CompatibilityMatrix {
    pub fn new() -> Self {
        let mut matrix = HashMap::new();
        
        // DAG + RAG: High semantic affinity, strong synergy
        // (decomposition + synthesis = adaptive strategy)
        matrix.insert((SkillType::DAG, SkillType::RAG), CompatibilityScore::calculate(0.95, 0.85, 0.90));
        matrix.insert((SkillType::RAG, SkillType::DAG), CompatibilityScore::calculate(0.95, 0.85, 0.90));
        
        // DAG + MCP: High synergy, good emergence
        // (decomposition + tool use = orchestrated automation)
        matrix.insert((SkillType::DAG, SkillType::MCP), CompatibilityScore::calculate(0.85, 0.88, 0.80));
        matrix.insert((SkillType::MCP, SkillType::DAG), CompatibilityScore::calculate(0.85, 0.88, 0.80));
        
        // RAG + MCP: Very high synergy
        // (synthesis + tool use = informed execution)
        matrix.insert((SkillType::RAG, SkillType::MCP), CompatibilityScore::calculate(0.90, 0.92, 0.85));
        matrix.insert((SkillType::MCP, SkillType::RAG), CompatibilityScore::calculate(0.90, 0.92, 0.85));
        
        // DAG + API: Good federation coordination
        matrix.insert((SkillType::DAG, SkillType::API), CompatibilityScore::calculate(0.75, 0.80, 0.70));
        matrix.insert((SkillType::API, SkillType::DAG), CompatibilityScore::calculate(0.75, 0.80, 0.70));
        
        // RAG + API: Shared knowledge discovery
        matrix.insert((SkillType::RAG, SkillType::API), CompatibilityScore::calculate(0.80, 0.78, 0.75));
        matrix.insert((SkillType::API, SkillType::RAG), CompatibilityScore::calculate(0.80, 0.78, 0.75));
        
        // MCP + API: Tool distribution and coordination
        matrix.insert((SkillType::MCP, SkillType::API), CompatibilityScore::calculate(0.82, 0.85, 0.78));
        matrix.insert((SkillType::API, SkillType::MCP), CompatibilityScore::calculate(0.82, 0.85, 0.78));
        
        // Same type fusions (rare but powerful)
        matrix.insert((SkillType::DAG, SkillType::DAG), CompatibilityScore::calculate(0.70, 0.75, 0.65));
        matrix.insert((SkillType::RAG, SkillType::RAG), CompatibilityScore::calculate(0.72, 0.78, 0.68));
        matrix.insert((SkillType::MCP, SkillType::MCP), CompatibilityScore::calculate(0.68, 0.80, 0.70));
        matrix.insert((SkillType::API, SkillType::API), CompatibilityScore::calculate(0.65, 0.72, 0.60));
        
        // Fusion + any type: Very high potential (fusion creates emergence)
        matrix.insert((SkillType::Fusion, SkillType::DAG), CompatibilityScore::calculate(0.88, 0.85, 0.95));
        matrix.insert((SkillType::Fusion, SkillType::RAG), CompatibilityScore::calculate(0.90, 0.87, 0.97));
        matrix.insert((SkillType::Fusion, SkillType::MCP), CompatibilityScore::calculate(0.87, 0.86, 0.94));
        matrix.insert((SkillType::Fusion, SkillType::API), CompatibilityScore::calculate(0.85, 0.83, 0.92));
        
        Self { matrix }
    }
    
    /// Get compatibility score between two skill types
    pub fn get_score(&self, type1: SkillType, type2: SkillType) -> CompatibilityScore {
        self.matrix.get(&(type1, type2))
            .copied()
            .unwrap_or_else(|| CompatibilityScore::calculate(0.5, 0.5, 0.5))
    }
}

/// Request to fuse two or more skills
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionRequest {
    pub requester_id: String,         // Specialist requesting fusion
    pub parent_skill_ids: Vec<String>, // IDs of skills to fuse (2-4)
    pub proposed_name: Option<String>, // Optional custom name for fusion
    pub proposed_properties: Option<Vec<String>>, // Suggested emergent properties
    pub requested_at: DateTime<Utc>,
}

impl FusionRequest {
    pub fn new(requester_id: String, parent_skill_ids: Vec<String>) -> Self {
        Self {
            requester_id,
            parent_skill_ids,
            proposed_name: None,
            proposed_properties: None,
            requested_at: Utc::now(),
        }
    }
    
    /// Validate request
    pub fn validate(&self) -> Result<(), String> {
        if self.parent_skill_ids.len() < 2 {
            return Err("Need at least 2 skills to fuse".to_string());
        }
        if self.parent_skill_ids.len() > 4 {
            return Err("Cannot fuse more than 4 skills at once".to_string());
        }
        if self.parent_skill_ids.len() != self.parent_skill_ids.iter().collect::<std::collections::HashSet<_>>().len() {
            return Err("Duplicate skills in fusion request".to_string());
        }
        Ok(())
    }
}

/// Result of a fusion operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionResult {
    pub success: bool,
    pub fusion_id: Option<String>,
    pub fused_skill_name: Option<String>,
    pub compatibility_score: f64,
    pub power_multiplier: f64,
    pub emergent_properties: Vec<String>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

impl FusionResult {
    pub fn success(
        fusion_id: String,
        name: String,
        compat: f64,
        power: f64,
        properties: Vec<String>,
    ) -> Self {
        Self {
            success: true,
            fusion_id: Some(fusion_id),
            fused_skill_name: Some(name),
            compatibility_score: compat,
            power_multiplier: power,
            emergent_properties: properties,
            message: "Fusion successful".to_string(),
            created_at: Utc::now(),
        }
    }
    
    pub fn failure(message: String) -> Self {
        Self {
            success: false,
            fusion_id: None,
            fused_skill_name: None,
            compatibility_score: 0.0,
            power_multiplier: 0.0,
            emergent_properties: Vec::new(),
            message,
            created_at: Utc::now(),
        }
    }
}

/// Fusion suggestion for a specialist
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FusionSuggestion {
    pub skill_id_1: String,
    pub skill_id_2: String,
    pub skill_id_3: Option<String>,
    pub compatibility_score: f64,
    pub expected_name: String,
    pub expected_properties: Vec<String>,
    pub power_improvement: f64,
    pub priority: u8, // 1-5, higher = more recommended
}

impl FusionSuggestion {
    pub fn new(
        skill_id_1: &str,
        skill_id_2: &str,
        compat: f64,
        name: String,
        properties: Vec<String>,
        power_improvement: f64,
    ) -> Self {
        let priority = match compat {
            c if c >= 0.95 => 5,
            c if c >= 0.85 => 4,
            c if c >= 0.75 => 3,
            c if c >= 0.65 => 2,
            _ => 1,
        };
        
        Self {
            skill_id_1: skill_id_1.to_string(),
            skill_id_2: skill_id_2.to_string(),
            skill_id_3: None,
            compatibility_score: compat,
            expected_name: name,
            expected_properties: properties,
            power_improvement,
            priority,
        }
    }
}

/// Main Skill Fusion Engine
pub struct SkillFusionEngine {
    compatibility_matrix: CompatibilityMatrix,
    fusion_history: Vec<FusionResult>,
    active_fusion_requests: HashMap<String, FusionRequest>,
}

impl SkillFusionEngine {
    pub fn new() -> Self {
        Self {
            compatibility_matrix: CompatibilityMatrix::new(),
            fusion_history: Vec::new(),
            active_fusion_requests: HashMap::new(),
        }
    }
    
    /// Calculate compatibility between two skills
    pub fn calculate_compatibility(&self, skill1: &Skill, skill2: &Skill) -> CompatibilityScore {
        let mut base_score = self.compatibility_matrix.get_score(skill1.skill_type, skill2.skill_type);
        
        // Level bonus: higher levels = better compatibility
        let level_factor = ((skill1.level as f64 + skill2.level as f64) / 40.0).min(0.2);
        
        // Success rate bonus: more reliable skills = better fusion
        let success_factor = ((skill1.success_rate + skill2.success_rate) / 2.0) * 0.1;
        
        // Quality bonus: higher quality skills = better results
        let quality_factor = ((skill1.average_quality + skill2.average_quality) / 20.0).min(0.1);
        
        // Apply bonuses
        base_score.semantic_affinity = (base_score.semantic_affinity + level_factor).min(1.0);
        base_score.power_synergy = (base_score.power_synergy + success_factor).min(1.0);
        base_score.emergence_potential = (base_score.emergence_potential + quality_factor).min(1.0);
        
        // Recalculate overall
        base_score.overall_score = (base_score.semantic_affinity * 0.4) 
            + (base_score.power_synergy * 0.35) 
            + (base_score.emergence_potential * 0.25);
        
        base_score
    }
    
    /// Find all viable fusion pairs in a skillset
    pub fn discover_fusions(&self, skillset: &SpecialistSkillSet) -> Vec<FusionSuggestion> {
        let mut suggestions = Vec::new();
        let skills: Vec<&Skill> = skillset.skills.values().collect();
        
        // Check all pairs
        for i in 0..skills.len() {
            for j in (i + 1)..skills.len() {
                let skill1 = skills[i];
                let skill2 = skills[j];
                
                // Both must be at least level 3
                if skill1.level < 3 || skill2.level < 3 {
                    continue;
                }
                
                let compat = self.calculate_compatibility(skill1, skill2);
                if compat.is_viable() {
                    let (fused_name, properties) = self.generate_fusion_name_and_properties(skill1, skill2);
                    let power_improvement = self.calculate_power_improvement(skill1, skill2, &compat);
                    
                    suggestions.push(FusionSuggestion::new(
                        &skill1.skill_id,
                        &skill2.skill_id,
                        compat.overall_score,
                        fused_name,
                        properties,
                        power_improvement,
                    ));
                }
            }
        }
        
        // Sort by priority and compatibility
        suggestions.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| b.compatibility_score.partial_cmp(&a.compatibility_score).unwrap())
        });
        
        suggestions
    }
    
    /// Submit a fusion request
    pub fn request_fusion(&mut self, request: FusionRequest) -> Result<String, String> {
        request.validate()?;
        
        let request_id = format!("fusion_req_{}", uuid::Uuid::new_v4());
        self.active_fusion_requests.insert(request_id.clone(), request);
        
        Ok(request_id)
    }
    
    /// Execute a fusion (after validation)
    pub fn execute_fusion(
        &mut self,
        parent_skills: Vec<&Skill>,
        specialist_id: String,
    ) -> FusionResult {
        if parent_skills.len() < 2 || parent_skills.len() > 4 {
            return FusionResult::failure("Invalid number of parent skills".to_string());
        }
        
        // Verify all at least level 3
        if parent_skills.iter().any(|s| s.level < 3) {
            return FusionResult::failure("All skills must be level 3+".to_string());
        }
        
        // Calculate overall compatibility
        let mut total_compat = 0.0;
        for i in 0..parent_skills.len() {
            for j in (i + 1)..parent_skills.len() {
                let compat = self.calculate_compatibility(parent_skills[i], parent_skills[j]);
                total_compat += compat.overall_score;
            }
        }
        let pairwise_count = (parent_skills.len() * (parent_skills.len() - 1)) / 2;
        let avg_compat = total_compat / pairwise_count as f64;
        
        if avg_compat < 0.6 {
            return FusionResult::failure("Insufficient compatibility for fusion".to_string());
        }
        
        // Generate fusion
        let (fused_name, properties) = self.generate_fusion_name_and_properties(parent_skills[0], parent_skills[1]);
        let power_multiplier = self.calculate_fusion_power(&parent_skills, avg_compat);
        let fusion_id = format!("fused_{}_{}", specialist_id, uuid::Uuid::new_v4());
        
        let result = FusionResult::success(
            fusion_id,
            fused_name,
            avg_compat,
            power_multiplier,
            properties,
        );
        
        self.fusion_history.push(result.clone());
        result
    }
    
    /// Generate appropriate name and emergent properties for a fusion
    fn generate_fusion_name_and_properties(&self, skill1: &Skill, skill2: &Skill) -> (String, Vec<String>) {
        // Name generation based on skill types
        match (skill1.skill_type, skill2.skill_type) {
            (SkillType::DAG, SkillType::RAG) | (SkillType::RAG, SkillType::DAG) => {
                (
                    "Adaptive Strategic Integration".to_string(),
                    vec![
                        "Real-time problem decomposition".to_string(),
                        "Knowledge-informed decision making".to_string(),
                        "Emergent pattern adaptation".to_string(),
                    ]
                )
            },
            (SkillType::DAG, SkillType::MCP) | (SkillType::MCP, SkillType::DAG) => {
                (
                    "Orchestrated Task Automation".to_string(),
                    vec![
                        "Parallel tool coordination".to_string(),
                        "Dependency-aware execution".to_string(),
                        "Failure recovery automation".to_string(),
                    ]
                )
            },
            (SkillType::RAG, SkillType::MCP) | (SkillType::MCP, SkillType::RAG) => {
                (
                    "Informed Tool Synthesis".to_string(),
                    vec![
                        "Knowledge-driven tool selection".to_string(),
                        "Context-aware execution".to_string(),
                        "Dynamic capability discovery".to_string(),
                    ]
                )
            },
            _ => {
                let combined = format!("{} & {}", skill1.skill_name, skill2.skill_name);
                (
                    format!("Combined {}", combined),
                    vec![
                        "Emergent capability".to_string(),
                        "Synergistic properties".to_string(),
                    ]
                )
            }
        }
    }
    
    /// Calculate power improvement from fusion
    fn calculate_power_improvement(&self, skill1: &Skill, skill2: &Skill, compat: &CompatibilityScore) -> f64 {
        let base_power = (skill1.power_score() + skill2.power_score()) / 2.0;
        let synergy_bonus = compat.power_synergy * 2.0;
        let emergence_bonus = compat.emergence_potential * 1.5;
        
        base_power + synergy_bonus + emergence_bonus
    }
    
    /// Calculate final power of fused skill
    fn calculate_fusion_power(&self, parent_skills: &[&Skill], compatibility: f64) -> f64 {
        let parent_power: f64 = parent_skills.iter().map(|s| s.power_score()).sum();
        let avg_power = parent_power / parent_skills.len() as f64;
        
        // Fusion power bonus: 1.5x to 3.0x depending on compatibility
        let multiplier = 1.5 + (compatibility * 1.5);
        (avg_power * multiplier).min(30.0)
    }
    
    /// Get all fusions for a specialist
    pub fn get_specialist_fusions<'a>(&self, specialist_id: &str, skillset: &'a SpecialistSkillSet) -> Vec<&'a FusedSkill> {
        skillset.fused_skills.iter()
            .filter(|f| f.specialist_id == specialist_id)
            .collect()
    }
    
    /// Record fusion in history
    pub fn add_fusion_to_history(&mut self, result: FusionResult) {
        self.fusion_history.push(result);
    }
    
    /// Get fusion history
    pub fn get_fusion_history(&self) -> &[FusionResult] {
        &self.fusion_history
    }
    
    /// Find specialists who can teach fusion
    pub fn find_fusion_mentors(&self, fusion_name: &str, skillset: &SpecialistSkillSet) -> Vec<String> {
        skillset.fused_skills.iter()
            .filter(|f| f.fused_skill_name.contains(fusion_name))
            .map(|f| f.specialist_id.clone())
            .collect()
    }
    
    /// Check if specialist can learn this fusion from mentor
    pub fn can_mentor_fusion(&self, mentor: &SpecialistSkillSet, _apprentice: &SpecialistSkillSet) -> bool {
        // Mentor must have awakened skills or high-level fusions
        let has_advanced_skills = mentor.skills.values().any(|s| s.is_awakened || s.level >= 10);
        let has_fusions = !mentor.fused_skills.is_empty();
        
        has_advanced_skills && has_fusions
    }
    
    /// Get all viable multi-skill fusions (3+ skills)
    pub fn discover_triple_fusions(&self, skillset: &SpecialistSkillSet) -> Vec<(Vec<String>, f64)> {
        let mut triple_fusions = Vec::new();
        let skills: Vec<&Skill> = skillset.skills.values().collect();
        
        // Check all triples
        if skills.len() >= 3 {
            for i in 0..skills.len() {
                for j in (i + 1)..skills.len() {
                    for k in (j + 1)..skills.len() {
                        let skill1 = skills[i];
                        let skill2 = skills[j];
                        let skill3 = skills[k];
                        
                        if skill1.level >= 3 && skill2.level >= 3 && skill3.level >= 3 {
                            let c12 = self.calculate_compatibility(skill1, skill2);
                            let c13 = self.calculate_compatibility(skill1, skill3);
                            let c23 = self.calculate_compatibility(skill2, skill3);
                            
                            let avg_compat = (c12.overall_score + c13.overall_score + c23.overall_score) / 3.0;
                            
                            if avg_compat >= 0.65 {
                                triple_fusions.push((
                                    vec![
                                        skill1.skill_id.clone(),
                                        skill2.skill_id.clone(),
                                        skill3.skill_id.clone(),
                                    ],
                                    avg_compat,
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by compatibility
        triple_fusions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        triple_fusions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_system::SkillOrigin;
    
    #[test]
    fn test_compatibility_matrix() {
        let matrix = CompatibilityMatrix::new();
        
        // DAG + RAG should be very compatible
        let score = matrix.get_score(SkillType::DAG, SkillType::RAG);
        assert!(score.overall_score >= 0.8);
        assert!(score.is_viable());
    }
    
    #[test]
    fn test_fusion_request_validation() {
        let mut req = FusionRequest::new("spec_1".to_string(), vec!["skill1".to_string()]);
        assert!(req.validate().is_err()); // Too few skills
        
        req.parent_skill_ids.push("skill2".to_string());
        assert!(req.validate().is_ok()); // Valid
        
        for _ in 0..5 {
            req.parent_skill_ids.push("skill_extra".to_string());
        }
        assert!(req.validate().is_err()); // Too many skills
    }
    
    #[test]
    fn test_fusion_engine_discovery() {
        let engine = SkillFusionEngine::new();
        let mut skillset = SpecialistSkillSet::new("spec_1".to_string());
        
        // Add two compatible skills
        let mut skill1 = Skill::new(
            "dag_skill".to_string(),
            "Decomposition".to_string(),
            SkillType::DAG,
            "spec_1".to_string(),
            SkillOrigin::Genetic,
            "Task decomposition".to_string(),
            "Breaks tasks into parts".to_string(),
        );
        skill1.level = 5;
        skill1.usage_count = 10;
        skill1.success_rate = 0.85;
        skill1.average_quality = 7.5;
        
        let mut skill2 = Skill::new(
            "rag_skill".to_string(),
            "Synthesis".to_string(),
            SkillType::RAG,
            "spec_1".to_string(),
            SkillOrigin::Genetic,
            "Knowledge synthesis".to_string(),
            "Combines information".to_string(),
        );
        skill2.level = 5;
        skill2.usage_count = 10;
        skill2.success_rate = 0.80;
        skill2.average_quality = 7.0;
        
        skillset.add_skill(skill1);
        skillset.add_skill(skill2);
        
        let suggestions = engine.discover_fusions(&skillset);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].compatibility_score > 0.8);
    }
    
    #[test]
    fn test_fusion_execution() {
        let mut engine = SkillFusionEngine::new();
        
        let mut skill1 = Skill::new(
            "skill1".to_string(),
            "Skill 1".to_string(),
            SkillType::DAG,
            "spec".to_string(),
            SkillOrigin::Genetic,
            "Test".to_string(),
            "Test".to_string(),
        );
        skill1.level = 5;
        
        let mut skill2 = Skill::new(
            "skill2".to_string(),
            "Skill 2".to_string(),
            SkillType::RAG,
            "spec".to_string(),
            SkillOrigin::Genetic,
            "Test".to_string(),
            "Test".to_string(),
        );
        skill2.level = 5;
        
        let result = engine.execute_fusion(vec![&skill1, &skill2], "spec".to_string());
        assert!(result.success);
        assert!(result.power_multiplier > 1.5);
        assert!(!result.emergent_properties.is_empty());
    }
}
