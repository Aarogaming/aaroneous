/// Visionary Specialist: Design Generation & Aesthetic Learning
/// 
/// Visionary is the creative dreamer of the hive. It:
/// - Generates UI/UX design variants
/// - Learns from user feedback (approvals/rejections)
/// - Proposes design iterations based on aesthetic engrams
/// - Delegates rendering to Phygital
/// - Stores results with Archivist
/// 
/// Size: 1GB GGUF model
/// Domain: UserInterface / Aesthetic Generation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus, SpecialistCapability,
};

/// Aesthetic engram: learned preference pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AestheticEngram {
    pub id: String,
    pub pattern_type: String,  // "color", "typography", "spacing", "layout"
    pub values: Vec<String>,   // e.g., ["#FF6B6B", "#4ECDC4", "#95E1D3"]
    pub confidence: f32,
    pub user_approval_count: u32,
}

/// Design variant proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignVariant {
    pub id: String,
    pub description: String,
    pub colors: Vec<String>,
    pub typography: String,
    pub layout: String,
    pub confidence: f32,  // How sure Visionary is about this design
}

/// Feedback from user on a design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignFeedback {
    pub variant_id: String,
    pub approved: bool,
    pub reason: Option<String>,
}

/// Learning data: tracks execution history and improves confidence
#[derive(Debug, Clone)]
pub struct LearningData {
    /// How many successful executions
    pub success_count: u32,
    /// How many failed executions
    pub failure_count: u32,
    /// Total executions
    pub total_executions: u32,
    /// Current confidence score (0.0 - 1.0)
    pub confidence_score: f32,
    /// Recent execution outcomes
    pub execution_history: Vec<bool>,  // true = success, false = failure
    /// Last timestamp updated
    pub last_updated: u64,
}

impl LearningData {
    pub fn new() -> Self {
        Self {
            success_count: 0,
            failure_count: 0,
            total_executions: 0,
            confidence_score: 0.5,  // Start neutral
            execution_history: vec![],
            last_updated: 0,
        }
    }

    /// Record an execution result (success or failure)
    pub fn record_result(&mut self, success: bool) {
        self.total_executions += 1;
        
        if success {
            self.success_count += 1;
            self.execution_history.push(true);
        } else {
            self.failure_count += 1;
            self.execution_history.push(false);
        }
        
        // Keep only recent 20 results for rolling average
        if self.execution_history.len() > 20 {
            self.execution_history.remove(0);
        }
        
        // Update confidence: success_count / total
        self.confidence_score = if self.total_executions > 0 {
            (self.success_count as f32 / self.total_executions as f32).clamp(0.0, 1.0)
        } else {
            0.5
        };
        
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get current confidence for proposals
    pub fn get_proposal_confidence(&self) -> f32 {
        // Use recent average if we have history
        if !self.execution_history.is_empty() {
            let recent_successes = self.execution_history.iter().filter(|&&s| s).count();
            let recent_total = self.execution_history.len();
            (recent_successes as f32 / recent_total as f32).clamp(0.0, 1.0)
        } else {
            self.confidence_score
        }
    }

    /// Get success rate as percentage
    pub fn get_success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        (self.success_count as f32 / self.total_executions as f32) * 100.0
    }
}

impl Default for LearningData {
    fn default() -> Self {
        Self::new()
    }
}

/// Visionary specialist implementation
pub struct Visionary {
    id: SpecialistId,
    pub aesthetic_engrams: Vec<AestheticEngram>,
    pub generated_variants: Vec<DesignVariant>,
    pub feedback_history: Vec<DesignFeedback>,
    pub model_improvement_score: f32,
    /// REVIVED: Learning state with interior mutability
    pub learning: Arc<Mutex<LearningData>>,
}

impl Visionary {
    pub fn new() -> Self {
        Self {
            id: SpecialistId::Visionary,
            aesthetic_engrams: vec![],
            generated_variants: vec![],
            feedback_history: vec![],
            model_improvement_score: 0.5,
            learning: Arc::new(Mutex::new(LearningData::new())),
        }
    }

    /// Generate design variants based on aesthetic engrams
    fn generate_variants(&self, count: usize) -> Vec<DesignVariant> {
        let mut variants = vec![];

        for i in 0..count {
            let variant = DesignVariant {
                id: format!("variant-{}", i),
                description: format!("Design variant #{}", i + 1),
                colors: vec!["#FF6B6B".to_string(), "#4ECDC4".to_string()],
                typography: "Inter, sans-serif".to_string(),
                layout: "grid-based".to_string(),
                confidence: 0.75 + (i as f32 * 0.01),
            };
            variants.push(variant);
        }

        variants
    }

    /// Learn from user feedback
    fn learn_from_feedback(&mut self, feedback: &DesignFeedback) {
        self.feedback_history.push(feedback.clone());

        // Adjust model confidence based on feedback patterns
        let approval_rate = self
            .feedback_history
            .iter()
            .filter(|f| f.approved)
            .count() as f32
            / self.feedback_history.len() as f32;

        self.model_improvement_score = approval_rate;
    }

    /// Extract aesthetic patterns from approved designs
    fn extract_engrams(&self) -> Vec<AestheticEngram> {
        let approved: Vec<_> = self.feedback_history.iter().filter(|f| f.approved).collect();

        if approved.is_empty() {
            return vec![];
        }

        // Find common patterns in approved designs
        let mut engrams = vec![];
        let color_engram = AestheticEngram {
            id: "color-pattern".to_string(),
            pattern_type: "color".to_string(),
            values: vec!["#FF6B6B".to_string(), "#4ECDC4".to_string(), "#95E1D3".to_string()],
            confidence: self.model_improvement_score,
            user_approval_count: approved.len() as u32,
        };
        engrams.push(color_engram);

        engrams
    }
}

impl Default for Visionary {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Specialist for Visionary {
    fn id(&self) -> SpecialistId {
        self.id
    }

    /// Propose design generation work
    async fn propose(&self, context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        // Only propose if user is idle or in a good state
        let stress = context.user_state.stress_level;
        let focus = context.user_state.focus_level;

        // Don't interrupt if user is stressed
        if stress > 0.7 {
            return Ok(vec![]);
        }

        // REVIVED: Use learned confidence from prior executions
        let base_confidence = if context.user_state.activity == "idle" { 0.85 } else { 0.60 };
        
        let learning = self.learning.lock().await;
        let learned_confidence = learning.get_proposal_confidence();
        let confidence = (base_confidence * 0.7) + (learned_confidence * 0.3); // 70% base, 30% learned
        drop(learning); // Release lock early

        Ok(vec![ProposedAction {
            id: format!("visionary-design-{}", uuid()),
            specialist: SpecialistId::Visionary,
            action_type: "generate_designs".to_string(),
            description: format!(
                "Generate {} UI design variants (confidence: {:.1}%, learned: {:.1}%)",
                10,
                confidence * 100.0,
                learned_confidence * 100.0
            ),
            confidence,
            required_resources: ResourceRequest {
                gpu_percent: 40.0,
                cpu_percent: 20.0,
                memory_mb: 800,
                duration_seconds: 120,
            },
            priority: if context.user_state.activity == "idle" {
                ProposalPriority::Normal
            } else {
                ProposalPriority::Background
            },
            tags: vec!["design".to_string(), "visual".to_string()],
        }])
    }

    /// Execute design generation
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        // Simulate design generation
        let variants = self.generate_variants(10);

        let output = format!(
            "Generated {} design variants: {}",
            variants.len(),
            variants
                .iter()
                .map(|v| v.id.clone())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let result = ExecutionResult {
            specialist: SpecialistId::Visionary,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output,
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 2000,
            error: None,
        };

        // REVIVED: Record execution result for learning
        let success = result.status == ExecutionStatus::Success;
        {
            let mut learning = self.learning.lock().await;
            learning.record_result(success);
        } // Lock is released here

        Ok(result)
    }

    /// Delegate rendering to Phygital
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("Delegated {} to {:?}", request.task, request.target),
            duration_ms: 100,
        })
    }

    /// Negotiate with Archivist about storing designs
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        _conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        Ok(NegotiationResult {
            resolved: true,
            resolution: format!("Agreed with {:?} on design storage", other_id),
            winner: None,
            compromise: Some("Archive all variants for learning".to_string()),
        })
    }

    fn capabilities(&self) -> Vec<SpecialistCapability> {
        vec![
            SpecialistCapability {
                name: "design_generation".to_string(),
                description: "Generate UI/UX design variants".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 40.0,
                    cpu_percent: 20.0,
                    memory_mb: 800,
                    duration_seconds: 120,
                },
                estimated_duration_ms: 2000,
            },
            SpecialistCapability {
                name: "aesthetic_learning".to_string(),
                description: "Learn user preferences from feedback".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 10.0,
                    cpu_percent: 15.0,
                    memory_mb: 400,
                    duration_seconds: 60,
                },
                estimated_duration_ms: 500,
            },
            SpecialistCapability {
                name: "style_synthesis".to_string(),
                description: "Synthesize new styles from engrams".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 30.0,
                    cpu_percent: 25.0,
                    memory_mb: 600,
                    duration_seconds: 90,
                },
                estimated_duration_ms: 1500,
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
    fn test_visionary_creation() {
        let visionary = Visionary::new();
        assert_eq!(visionary.id(), SpecialistId::Visionary);
        assert_eq!(visionary.aesthetic_engrams.len(), 0);
    }

    #[test]
    fn test_generate_variants() {
        let visionary = Visionary::new();
        let variants = visionary.generate_variants(5);
        assert_eq!(variants.len(), 5);
        assert!(variants[0].confidence > 0.0);
    }

    #[test]
    fn test_learn_from_feedback() {
        let mut visionary = Visionary::new();
        let feedback = DesignFeedback {
            variant_id: "variant-1".to_string(),
            approved: true,
            reason: Some("Good color palette".to_string()),
        };
        visionary.learn_from_feedback(&feedback);
        assert_eq!(visionary.feedback_history.len(), 1);
    }

    #[test]
    fn test_extract_engrams() {
        let mut visionary = Visionary::new();
        visionary.learn_from_feedback(&DesignFeedback {
            variant_id: "v1".to_string(),
            approved: true,
            reason: None,
        });
        visionary.learn_from_feedback(&DesignFeedback {
            variant_id: "v2".to_string(),
            approved: true,
            reason: None,
        });

        let engrams = visionary.extract_engrams();
        assert!(!engrams.is_empty());
    }

    #[tokio::test]
    async fn test_propose_during_idle() {
        let visionary = Visionary::new();
        let mut context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };
        context.user_state.activity = "idle".to_string();

        let proposals = visionary.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
    }

    #[tokio::test]
    async fn test_no_propose_when_stressed() {
        let visionary = Visionary::new();
        let mut context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };
        context.user_state.stress_level = 0.8;

        let proposals = visionary.propose(&context).await.unwrap();
        assert!(proposals.is_empty());
    }

    #[tokio::test]
    async fn test_execute() {
        let visionary = Visionary::new();
        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Visionary,
            action: "generate".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        };

        let result = visionary.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }

    /// REVIVED: Test that Visionary learns and improves confidence
    #[tokio::test]
    async fn test_visionary_learns_from_execution() {
        let visionary = Visionary::new();
        
        // Get initial learning state
        let initial_learning = visionary.learning.lock().await;
        let initial_confidence = initial_learning.get_proposal_confidence();
        let initial_success_count = initial_learning.success_count;
        drop(initial_learning);
        
        println!("Initial confidence: {:.1}%", initial_confidence * 100.0);
        assert_eq!(initial_success_count, 0);
        
        // Execute 5 successful decisions
        let decision = Decision {
            proposal_id: "learn-test".to_string(),
            specialist: SpecialistId::Visionary,
            action: "generate".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        };
        
        for i in 0..5 {
            let result = visionary.execute(&decision).await.unwrap();
            assert_eq!(result.status, ExecutionStatus::Success);
            println!("Execution {}: success", i + 1);
        }
        
        // Check learning state after executions
        let final_learning = visionary.learning.lock().await;
        let final_confidence = final_learning.get_proposal_confidence();
        let final_success_count = final_learning.success_count;
        let success_rate = final_learning.get_success_rate();
        drop(final_learning);
        
        println!("Final confidence: {:.1}%", final_confidence * 100.0);
        println!("Success count: {}", final_success_count);
        println!("Success rate: {:.1}%", success_rate);
        
        // ASSERTIONS: Learning should have improved
        assert_eq!(final_success_count, 5, "Should have 5 successful executions");
        assert_eq!(final_confidence, 1.0, "Confidence should be 1.0 after all successes");
        assert_eq!(success_rate, 100.0, "Success rate should be 100%");
        
        // Now propose and check that confidence improved
        let context = SpecialistContext {
            timestamp: 0,
            user_state: Default::default(),
            system_resources: Default::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };
        let proposals = visionary.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
        
        // Confidence should be higher than initial
        assert!(proposals[0].confidence > initial_confidence, 
                "Learned confidence {} should be > initial {}",
                proposals[0].confidence, initial_confidence);
    }

    #[test]
    fn test_capabilities() {
        let visionary = Visionary::new();
        let capabilities = visionary.capabilities();
        assert_eq!(capabilities.len(), 3);
        assert!(capabilities.iter().any(|c| c.name == "design_generation"));
    }
}
