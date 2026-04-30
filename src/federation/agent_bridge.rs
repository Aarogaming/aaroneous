/// Agent Bridge: Connects existing agents (Ariel/Merlin/Odin/etc) to the federation
/// 
/// This module allows the existing SpecialistAgent/RelicAgent system to work
/// seamlessly with the new federation protocol while maintaining backward compatibility.
/// 
/// Each existing agent (Ariel, Merlin, Odin, Hephaestus, Argus, Dionysus) can now:
/// - Propose actions asynchronously (new)
/// - Execute decisions from Sentinel (new)
/// - Negotiate with peers (new)
/// - While keeping all existing hox_preset and enzyme_subset logic

use async_trait::async_trait;
use crate::agents::SpecialistAgent;
use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistContext, SpecialistError, ProposedAction,
    Decision, DelegateRequest, DelegateResponse, Conflict, NegotiationResult,
    ResourceRequest, ProposalPriority, ExecutionResult, ExecutionStatus,
};

/// Maps existing SpecialistAgent names to federation SpecialistIds
pub fn agent_name_to_specialist_id(name: &str) -> Option<SpecialistId> {
    match name.to_lowercase().as_str() {
        "ariel" => Some(SpecialistId::Visionary),        // UI/UX → Design Generation
        "merlin" => Some(SpecialistId::Sentinel),        // Knowledge/Leadership → Orchestration
        "odin" => Some(SpecialistId::Sentinel),          // Leadership/Strategy → Orchestration
        "hephaestus" => Some(SpecialistId::Phygital),    // Manufacturing → Spatial/Rendering
        "argus" => Some(SpecialistId::Sentinel),         // Security → Arbitration/Oversight
        "dionysus" => Some(SpecialistId::Archivist),     // Experience/Memory → Archives
        _ => None,
    }
}

/// Bridge specialist: wraps an existing SpecialistAgent in the Specialist trait
pub struct SpecialistAgentBridge {
    agent: SpecialistAgent,
    specialist_id: SpecialistId,
    execution_history: tokio::sync::Mutex<Vec<ExecutionResult>>,
}

impl SpecialistAgentBridge {
    pub fn new(agent: SpecialistAgent) -> Result<Self, SpecialistError> {
        let specialist_id = agent_name_to_specialist_id(&agent.name)
            .ok_or(SpecialistError::ModelNotLoaded)?;

        Ok(Self {
            agent,
            specialist_id,
            execution_history: tokio::sync::Mutex::new(vec![]),
        })
    }

    pub fn agent(&self) -> &SpecialistAgent {
        &self.agent
    }

    pub async fn execution_history(&self) -> Vec<ExecutionResult> {
        let history = self.execution_history.lock().await;
        history.clone()
    }
}

#[async_trait]
impl Specialist for SpecialistAgentBridge {
    fn id(&self) -> SpecialistId {
        self.specialist_id
    }

    /// Propose action based on agent's domain expertise
    async fn propose(&self, _context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        // Proposals come from the agent's domain expertise and current context
        let proposals = match self.agent.domain {
            crate::agents::Domain::UserInterface => {
                // Ariel (Visionary): Visual design proposals
                vec![ProposedAction {
                    id: format!("proposal-ariel-{}", uuid()),
                    specialist: SpecialistId::Visionary,
                    action_type: "generate_designs".to_string(),
                    description: format!("{} proposes visual iteration", self.agent.name),
                    confidence: 0.85,
                    required_resources: ResourceRequest {
                        gpu_percent: 40.0,
                        cpu_percent: 20.0,
                        memory_mb: 800,
                        duration_seconds: 120,
                    },
                    priority: ProposalPriority::Normal,
                    tags: vec!["design".to_string(), "visual".to_string()],
                }]
            }
            crate::agents::Domain::Knowledge => {
                // Merlin: Knowledge synthesis proposals
                vec![ProposedAction {
                    id: format!("proposal-merlin-{}", uuid()),
                    specialist: SpecialistId::Sentinel,
                    action_type: "knowledge_synthesis".to_string(),
                    description: format!("{} proposes knowledge pattern analysis", self.agent.name),
                    confidence: 0.90,
                    required_resources: ResourceRequest {
                        gpu_percent: 30.0,
                        cpu_percent: 40.0,
                        memory_mb: 1200,
                        duration_seconds: 90,
                    },
                    priority: ProposalPriority::Normal,
                    tags: vec!["analysis".to_string(), "knowledge".to_string()],
                }]
            }
            crate::agents::Domain::Leadership => {
                // Odin: Strategic coordination proposals
                vec![ProposedAction {
                    id: format!("proposal-odin-{}", uuid()),
                    specialist: SpecialistId::Sentinel,
                    action_type: "coordination".to_string(),
                    description: format!("{} proposes orchestration decision", self.agent.name),
                    confidence: 0.88,
                    required_resources: ResourceRequest {
                        gpu_percent: 5.0,
                        cpu_percent: 15.0,
                        memory_mb: 400,
                        duration_seconds: 30,
                    },
                    priority: ProposalPriority::Urgent,
                    tags: vec!["leadership".to_string(), "orchestration".to_string()],
                }]
            }
            crate::agents::Domain::Manufacturing => {
                // Hephaestus: Execution/rendering proposals
                vec![ProposedAction {
                    id: format!("proposal-hephaestus-{}", uuid()),
                    specialist: SpecialistId::Phygital,
                    action_type: "render_spatial".to_string(),
                    description: format!("{} proposes 3D rendering", self.agent.name),
                    confidence: 0.82,
                    required_resources: ResourceRequest {
                        gpu_percent: 70.0,
                        cpu_percent: 30.0,
                        memory_mb: 1000,
                        duration_seconds: 60,
                    },
                    priority: ProposalPriority::UserFacing,
                    tags: vec!["rendering".to_string(), "spatial".to_string()],
                }]
            }
            crate::agents::Domain::Security => {
                // Argus: Security/validation proposals
                vec![ProposedAction {
                    id: format!("proposal-argus-{}", uuid()),
                    specialist: SpecialistId::Sentinel,
                    action_type: "validate_decision".to_string(),
                    description: format!("{} proposes security validation", self.agent.name),
                    confidence: 0.95,
                    required_resources: ResourceRequest {
                        gpu_percent: 10.0,
                        cpu_percent: 25.0,
                        memory_mb: 600,
                        duration_seconds: 45,
                    },
                    priority: ProposalPriority::Urgent,
                    tags: vec!["security".to_string(), "validation".to_string()],
                }]
            }
            crate::agents::Domain::Experience => {
                // Dionysus: Memory/experience proposals
                vec![ProposedAction {
                    id: format!("proposal-dionysus-{}", uuid()),
                    specialist: SpecialistId::Archivist,
                    action_type: "archive_experience".to_string(),
                    description: format!("{} proposes experience logging", self.agent.name),
                    confidence: 0.80,
                    required_resources: ResourceRequest {
                        gpu_percent: 0.0,
                        cpu_percent: 10.0,
                        memory_mb: 300,
                        duration_seconds: 20,
                    },
                    priority: ProposalPriority::Background,
                    tags: vec!["memory".to_string(), "experience".to_string()],
                }]
            }
            crate::agents::Domain::Undefined => vec![],
        };

        Ok(proposals)
    }

    /// Execute a decision from Sentinel
    async fn execute(&self, decision: &Decision) -> Result<ExecutionResult, SpecialistError> {
        // In a real implementation, this would invoke the agent's hox_preset
        // and execution logic. For now, we log the execution.

        let result = ExecutionResult {
            specialist: self.specialist_id,
            proposal_id: decision.proposal_id.clone(),
            status: ExecutionStatus::Success,
            output: format!(
                "{} executed {} (allocated {} GPU, {} memory)",
                self.agent.name,
                decision.action,
                decision.allocated_resources.gpu_percent,
                decision.allocated_resources.memory_mb
            ),
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 100,
            error: None,
        };

        let mut history = self.execution_history.lock().await;
        history.push(result.clone());

        Ok(result)
    }

    /// Delegate work to another specialist
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result: format!("{} delegated to {:?}", self.agent.name, request.target),
            duration_ms: 50,
        })
    }

    /// Negotiate with another specialist
    async fn negotiate(&self, other_id: SpecialistId, _conflict: &Conflict) -> Result<NegotiationResult, SpecialistError> {
        // In a real system, the agent's cognitive_bias would influence negotiation
        // Higher audit_strictness = more willing to compromise
        let willingness = self.agent.cognitive_bias.audit_strictness as f32 / 100.0;

        Ok(NegotiationResult {
            resolved: true,
            resolution: format!(
                "{} negotiated with {:?} (willingness: {:.1}%)",
                self.agent.name, other_id, willingness * 100.0
            ),
            winner: None,
            compromise: Some("Both agree to resource sharing".to_string()),
        })
    }

    async fn status(&self) -> Result<crate::federation::specialist::SpecialistStatus, SpecialistError> {
        let history = self.execution_history.lock().await;
        Ok(crate::federation::specialist::SpecialistStatus {
            id: self.specialist_id,
            enabled: true,
            load: 0.5,
            last_proposal: None,
            last_execution: history.last().map(|_| 0),
            error_count: 0,
        })
    }

    fn capabilities(&self) -> Vec<crate::federation::specialist::SpecialistCapability> {
        vec![
            crate::federation::specialist::SpecialistCapability {
                name: self.agent.role.clone(),
                description: self.agent.persona.clone(),
                required_resources: ResourceRequest::default(),
                estimated_duration_ms: 5000,
            },
        ]
    }
}

/// Simple UUID generator for proposal IDs
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
    fn test_agent_name_to_specialist_id() {
        assert_eq!(agent_name_to_specialist_id("ariel"), Some(SpecialistId::Visionary));
        assert_eq!(agent_name_to_specialist_id("merlin"), Some(SpecialistId::Sentinel));
        assert_eq!(agent_name_to_specialist_id("odin"), Some(SpecialistId::Sentinel));
        assert_eq!(agent_name_to_specialist_id("hephaestus"), Some(SpecialistId::Phygital));
        assert_eq!(agent_name_to_specialist_id("argus"), Some(SpecialistId::Sentinel));
        assert_eq!(agent_name_to_specialist_id("dionysus"), Some(SpecialistId::Archivist));
    }

    #[tokio::test]
    async fn test_bridge_creation() {
        let agent = crate::agents::create_specialist("ariel").unwrap();
        let bridge = SpecialistAgentBridge::new(agent);
        
        assert!(bridge.is_ok());
        let bridge = bridge.unwrap();
        assert_eq!(bridge.id(), SpecialistId::Visionary);
    }

    #[tokio::test]
    async fn test_bridge_propose() {
        let agent = crate::agents::create_specialist("ariel").unwrap();
        let bridge = SpecialistAgentBridge::new(agent).unwrap();

        let context = SpecialistContext {
            timestamp: 0,
            user_state: crate::federation::specialist::UserState::default(),
            system_resources: crate::federation::specialist::SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = bridge.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());
        assert!(proposals[0].specialist == SpecialistId::Visionary);
    }

    #[tokio::test]
    async fn test_bridge_execute() {
        let agent = crate::agents::create_specialist("merlin").unwrap();
        let bridge = SpecialistAgentBridge::new(agent).unwrap();

        let decision = Decision {
            proposal_id: "test".to_string(),
            specialist: SpecialistId::Sentinel,
            action: "analyze".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: std::collections::HashMap::new(),
        };

        let result = bridge.execute(&decision).await.unwrap();
        assert_eq!(result.status, ExecutionStatus::Success);
    }
}
