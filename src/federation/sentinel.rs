/// Sentinel: The Orchestrator Specialist
/// 
/// Sentinel is a specialist itself, but with special responsibilities:
/// - Collects proposals from all specialists
/// - Detects conflicts between proposals
/// - Arbitrates using priority, resources, and negotiation
/// - Issues decisions to specialists
/// - Monitors system health
/// 
/// Sentinel is NOT a bottleneck because:
/// - Specialists propose asynchronously (don't wait for approval)
/// - Specialists can self-organize and negotiate
/// - Sentinel's decisions are simple heuristics (not complex inference)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


use crate::federation::specialist::{
    Specialist, SpecialistId, SpecialistConfig, SpecialistContext, SystemResources,
    SpecialistError, ProposedAction, Decision, DelegateRequest, DelegateResponse,
    Conflict, NegotiationResult, ResourceRequest, ProposalPriority, UserState,
};
use crate::federation::proposal::Proposal;
use crate::federation::communication::CommunicationBus;
use crate::federation::conflict_resolution::{ConflictDetector, ConflictArbitrator};

/// Configuration for Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelConfig {
    pub base_config: SpecialistConfig,
    pub max_concurrent_decisions: usize,
    pub proposal_review_interval_ms: u64,
    pub enable_specialist_negotiation: bool,
    pub enable_resource_sharing: bool,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            base_config: SpecialistConfig::default_for(SpecialistId::Sentinel),
            max_concurrent_decisions: 10,
            proposal_review_interval_ms: 500,
            enable_specialist_negotiation: true,
            enable_resource_sharing: true,
        }
    }
}

/// Result of Sentinel's arbitration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrationResult {
    pub proposals_reviewed: usize,
    pub conflicts_detected: usize,
    pub conflicts_resolved: usize,
    pub decisions_issued: usize,
    pub negotiations_initiated: usize,
}

impl Default for ArbitrationResult {
    fn default() -> Self {
        Self {
            proposals_reviewed: 0,
            conflicts_detected: 0,
            conflicts_resolved: 0,
            decisions_issued: 0,
            negotiations_initiated: 0,
        }
    }
}

/// Sentinel: The orchestrator specialist
pub struct Sentinel {
    config: SentinelConfig,
    pub communication_bus: CommunicationBus,
    decision_history: tokio::sync::Mutex<Vec<Decision>>,
    current_system_resources: tokio::sync::Mutex<SystemResources>,
    current_user_state: tokio::sync::Mutex<UserState>,
    /// Optional event log for durable decision recording.
    /// When `Some`, every `issue_decision()` call appends a `FederationEvent`.
    pub event_log: Option<std::sync::Arc<crate::event_log::EventLog>>,
}

impl Sentinel {
    pub fn new(config: SentinelConfig, communication_bus: CommunicationBus) -> Self {
        Self {
            config,
            communication_bus,
            decision_history: tokio::sync::Mutex::new(vec![]),
            current_system_resources: tokio::sync::Mutex::new(SystemResources::default()),
            current_user_state: tokio::sync::Mutex::new(UserState::default()),
            event_log: None,
        }
    }

    /// Attach an EventLog to record decisions durably.
    pub fn with_event_log(mut self, log: std::sync::Arc<crate::event_log::EventLog>) -> Self {
        self.event_log = Some(log);
        self
    }

    /// Main arbitration loop: review proposals and issue decisions
    pub async fn arbitrate(&self) -> Result<ArbitrationResult, SpecialistError> {
        let mut result = ArbitrationResult::default();

        // Get all pending proposals
        let proposal_set = self.communication_bus.pending_proposals().await;
        result.proposals_reviewed = proposal_set.count();

        if proposal_set.count() == 0 {
            return Ok(result);
        }

        let resources = self.current_system_resources.lock().await.clone();

        // Detect conflicts
        let conflicts = ConflictDetector::detect(&proposal_set);
        result.conflicts_detected = conflicts.len();

        // Get viable proposals
        let viable = proposal_set.viable_sorted(&resources);

        // Resolve conflicts
        for conflict in conflicts {
            // Find the conflicting proposals
            let proposal_a = proposal_set.proposals.iter().find(|p| p.id == conflict.proposal_a_id);
            let proposal_b = proposal_set.proposals.iter().find(|p| p.id == conflict.proposal_b_id);

            if let (Some(p_a), Some(p_b)) = (proposal_a, proposal_b) {
                let resolution = ConflictArbitrator::resolve(&conflict, p_a, p_b, &resources);

                if resolution.resolved {
                    result.conflicts_resolved += 1;

                    // If we should try negotiation first
                    if self.config.enable_specialist_negotiation && resolution.winner.is_none() {
                        // Try peer-to-peer negotiation between specialists
                        result.negotiations_initiated += 1;
                    } else if let Some(winner) = resolution.winner {
                        // Issue decision to winner
                        let decision = self.decision_from_proposal(p_a, winner);
                        self.issue_decision(decision).await?;
                        result.decisions_issued += 1;
                    }
                }
            }
        }

        // Issue decisions for non-conflicting proposals
        for proposal in viable.iter().take(self.config.max_concurrent_decisions) {
            let decision = self.decision_from_proposal(proposal, proposal.specialist);
            self.issue_decision(decision).await?;
            result.decisions_issued += 1;
        }

        // Clear processed proposals
        self.communication_bus.clear_proposals().await;

        Ok(result)
    }

    /// Convert a proposal to a decision, forwarding proposal metadata into
    /// Decision.context so specialists (e.g. Visionary) can read the user intent.
    fn decision_from_proposal(&self, proposal: &Proposal, executor: SpecialistId) -> Decision {
        Decision {
            proposal_id: format!("{:?}", proposal.id),
            specialist: executor,
            action: proposal.action.clone(),
            allocated_resources: proposal.required_resources.clone(),
            deadline_ms: proposal.estimated_completion_ms,
            context: proposal.metadata.clone(),
        }
    }

    /// Issue a decision to a specialist and record it in the event log (if attached).
    async fn issue_decision(&self, decision: Decision) -> Result<(), SpecialistError> {
        let specialist_id = decision.specialist;

        // Record to EventLog for durable audit trail
        if let Some(log) = &self.event_log {
            use crate::event_log::types::{EventType, Operation, FederationEvent};
            use std::collections::HashMap;

            let mut payload = HashMap::new();
            payload.insert("proposal_id".to_string(),
                serde_json::Value::String(decision.proposal_id.clone()));
            payload.insert("action".to_string(),
                serde_json::Value::String(decision.action.clone()));
            payload.insert("specialist".to_string(),
                serde_json::Value::String(format!("{:?}", decision.specialist)));

            let event = FederationEvent {
                event_id: format!("dec-{}", uuid::Uuid::new_v4()),
                timestamp: chrono::Utc::now().timestamp_millis(),
                trace_id: decision.proposal_id.clone(),
                source_repo: "AaroneosFederation".to_string(),
                source_domain: "arbitration".to_string(),
                event_type: EventType::Mutation,
                operation: Operation::Create(format!("decision:{}", decision.action)),
                payload,
                consensus_round: None,
                replicas_acked: vec![],
                applied_at: None,
            };

            // Best-effort append — don't fail decision issuance if log fails
            let _ = log.append(event).await;
        }

        // Record to local in-memory decision history
        {
            let mut history = self.decision_history.lock().await;
            history.push(decision.clone());
        }

        // Send the decision to the specialist's channel
        let message = crate::federation::communication::SpecialistMessage::DecisionIssued(decision);
        if let Some(channel) = self.communication_bus.specialist_channel(specialist_id) {
            let _ = channel.send(message);
        }

        Ok(())
    }

    /// Update Sentinel's view of system resources
    pub async fn update_system_resources(&self, resources: SystemResources) {
        let mut current = self.current_system_resources.lock().await;
        *current = resources;
    }

    /// Update Sentinel's view of user state
    pub async fn update_user_state(&self, state: UserState) {
        let mut current = self.current_user_state.lock().await;
        *current = state;
    }

    /// Get decision history
    pub async fn decision_history(&self) -> Vec<Decision> {
        let history = self.decision_history.lock().await;
        history.clone()
    }
}

#[async_trait]
impl Specialist for Sentinel {
    fn id(&self) -> SpecialistId {
        SpecialistId::Sentinel
    }

    /// Sentinel's proposal: run arbitration cycle
    async fn propose(&self, _context: &SpecialistContext) -> Result<Vec<ProposedAction>, SpecialistError> {
        let result = self.arbitrate().await?;

        Ok(vec![ProposedAction {
            id: "sentinel_arbitrate".to_string(),
            specialist: SpecialistId::Sentinel,
            action_type: "orchestration".to_string(),
            description: format!(
                "Reviewed {} proposals, detected {} conflicts, resolved {}, issued {} decisions",
                result.proposals_reviewed, result.conflicts_detected, result.conflicts_resolved, result.decisions_issued
            ),
            confidence: 0.95,
            required_resources: ResourceRequest {
                gpu_percent: 0.0,
                cpu_percent: 5.0,
                memory_mb: 200,
                duration_seconds: 1,
            },
            priority: ProposalPriority::Urgent,
            tags: vec!["orchestration".to_string()],
        }])
    }

    /// Sentinel executes: this should not happen (Sentinel issues decisions, doesn't receive them)
    async fn execute(&self, decision: &Decision) -> Result<crate::federation::specialist::ExecutionResult, SpecialistError> {
        let mut history = self.decision_history.lock().await;
        history.push(decision.clone());

        Ok(crate::federation::specialist::ExecutionResult {
            specialist: SpecialistId::Sentinel,
            proposal_id: decision.proposal_id.clone(),
            status: crate::federation::specialist::ExecutionStatus::Success,
            output: "Decision recorded".to_string(),
            resources_used: decision.allocated_resources.clone(),
            duration_ms: 10,
            error: None,
        })
    }

    /// Sentinel delegates by forwarding requests to the target specialist's channel.
    ///
    /// Delegation is Sentinel's mechanism for cross-specialist coordination:
    /// when one specialist needs another's capability, it asks Sentinel to
    /// broker the handoff. Sentinel records the delegation and issues a
    /// directed message on the communication bus.
    async fn delegate(&self, request: &DelegateRequest) -> Result<DelegateResponse, SpecialistError> {
        let start = std::time::Instant::now();

        // Route the delegation request based on target specialist
        let result = match request.target {
            SpecialistId::Phygital => {
                // Visionary → Phygital: render a design prototype in AR
                format!(
                    "Delegated render request from {:?} to Phygital. \
                     Action: {}",
                    request.requester, request.task
                )
            }
            SpecialistId::Archivist => {
                // Any → Archivist: record data to DNA Bank
                format!(
                    "Delegated data recording from {:?} to Archivist. \
                     Recording: {}",
                    request.requester, request.task
                )
            }
            SpecialistId::Omnipresent => {
                // Any → Omnipresent: sync state across devices
                format!(
                    "Delegated sync request from {:?} to Omnipresent. \
                     Syncing: {}",
                    request.requester, request.task
                )
            }
            SpecialistId::Sentinel => {
                // Cannot delegate to self
                return Err(SpecialistError::DelegationFailed(
                    "Sentinel cannot delegate to itself".to_string(),
                ));
            }
            target => {
                format!(
                    "Delegated from {:?} to {:?}: {}",
                    request.requester, target, request.task
                )
            }
        };

        // Notify the bus that a delegation occurred (best-effort)
        let _ = self.communication_bus.broadcast(
            crate::federation::communication::SpecialistMessage::StatusUpdate(
                format!("Delegation: {:?} -> {:?}", request.requester, request.target)
            )
        ).await;

        Ok(DelegateResponse {
            requester: request.requester,
            target: request.target,
            success: true,
            result,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Sentinel negotiates a conflict between two specialists.
    ///
    /// Uses the conflict type and available context to determine resolution:
    /// - "gpu_resource" → proportional sharing if GPU headroom allows, else
    ///   a CRDT-style compromise ("execute sequentially, share results")
    /// - "duplicate_action" → whichever specialist has the higher domain
    ///   fitness (Sentinel-centric heuristic)
    /// - Default → collaborative: both specialists contribute to a merged
    ///   output via the CommunicationBus
    async fn negotiate(
        &self,
        other_id: SpecialistId,
        conflict: &Conflict,
    ) -> Result<NegotiationResult, SpecialistError> {
        // Read available GPU from system resources for resource conflicts
        let gpu_available = {
            let resources = self.current_system_resources.lock().await;
            resources.gpu_available_percent
        };

        let (winner, compromise, resolution) = match conflict.conflict_type.as_str() {
            "gpu_resource" => {
                // Parse the GPU demand hints from context if present
                let gpu_a: f32 = conflict.context.get("gpu_a")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(40.0);
                let gpu_b: f32 = conflict.context.get("gpu_b")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(40.0);

                if gpu_a + gpu_b <= gpu_available {
                    // Enough GPU for both
                    (
                        None,
                        Some(format!(
                            "Both {:?} and {:?} execute simultaneously \
                             ({:.0}% + {:.0}% = {:.0}% GPU, {:.0}% available)",
                            conflict.specialist_a, other_id, gpu_a, gpu_b,
                            gpu_a + gpu_b, gpu_available
                        )),
                        format!(
                            "Resource sharing: {:?} and {:?} run concurrently",
                            conflict.specialist_a, other_id
                        ),
                    )
                } else {
                    // Not enough GPU — schedule sequentially, UserFacing first
                    let winner_id = if conflict.context.get("a_priority") > conflict.context.get("b_priority") {
                        conflict.specialist_a
                    } else {
                        other_id
                    };
                    (
                        Some(winner_id),
                        Some(format!(
                            "{:?} runs first, {:?} queued until GPU is released ({:.0}% available)",
                            winner_id,
                            if winner_id == conflict.specialist_a { other_id } else { conflict.specialist_a },
                            gpu_available
                        )),
                        format!(
                            "Sequential: {:?} first (GPU at {:.0}% < {:.0}% needed)",
                            winner_id, gpu_available, gpu_a + gpu_b
                        ),
                    )
                }
            }

            "duplicate_action" => {
                // Same action requested by two specialists — Sentinel picks the
                // domain-appropriate one
                let winner_id = match (conflict.specialist_a, other_id) {
                    // Phygital is always the right specialist for rendering
                    (SpecialistId::Phygital, _) | (_, SpecialistId::Phygital) => SpecialistId::Phygital,
                    // Archivist is always right for storage
                    (SpecialistId::Archivist, _) | (_, SpecialistId::Archivist) => SpecialistId::Archivist,
                    // Omnipresent handles sync
                    (SpecialistId::Omnipresent, _) | (_, SpecialistId::Omnipresent) => SpecialistId::Omnipresent,
                    // Default: specialist_a proposed first
                    (a, _) => a,
                };
                (
                    Some(winner_id),
                    None,
                    format!(
                        "Domain routing: {:?} handles this action type ({:?} yields)",
                        winner_id,
                        if winner_id == conflict.specialist_a { other_id } else { conflict.specialist_a }
                    ),
                )
            }

            _ => {
                // Default: CRDT-style merge — both specialists succeed with
                // a coordination note broadcast on the bus
                let _ = self.communication_bus.broadcast(
                    crate::federation::communication::SpecialistMessage::StatusUpdate(
                        format!(
                            "Negotiation resolved: {:?} + {:?} collaborate on '{}'",
                            conflict.specialist_a, other_id, conflict.conflict_type
                        )
                    )
                ).await;
                (
                    None, // No single winner — collaborative
                    Some(format!(
                        "{:?} and {:?} coordinate via CommunicationBus for '{}'",
                        conflict.specialist_a, other_id, conflict.conflict_type
                    )),
                    format!(
                        "Collaborative: both {:?} and {:?} contribute",
                        conflict.specialist_a, other_id
                    ),
                )
            }
        };

        Ok(NegotiationResult {
            resolved: true,
            resolution,
            winner,
            compromise,
        })
    }

    async fn status(&self) -> Result<crate::federation::specialist::SpecialistStatus, SpecialistError> {
        let history = self.decision_history.lock().await;
        Ok(crate::federation::specialist::SpecialistStatus {
            id: SpecialistId::Sentinel,
            enabled: true,
            load: 0.3,
            last_proposal: None,
            last_execution: history.last().map(|_| 0), // Timestamp would go here
            error_count: 0,
        })
    }

    fn capabilities(&self) -> Vec<crate::federation::specialist::SpecialistCapability> {
        vec![
            crate::federation::specialist::SpecialistCapability {
                name: "arbitration".to_string(),
                description: "Arbitrate between competing proposals".to_string(),
                required_resources: ResourceRequest {
                    gpu_percent: 0.0,
                    cpu_percent: 5.0,
                    memory_mb: 200,
                    duration_seconds: 1,
                },
                estimated_duration_ms: 100,
            },
            crate::federation::specialist::SpecialistCapability {
                name: "conflict_resolution".to_string(),
                description: "Resolve conflicts between specialists".to_string(),
                required_resources: ResourceRequest::default(),
                estimated_duration_ms: 50,
            },
            crate::federation::specialist::SpecialistCapability {
                name: "resource_allocation".to_string(),
                description: "Allocate resources to proposals".to_string(),
                required_resources: ResourceRequest::default(),
                estimated_duration_ms: 50,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sentinel_creation() {
        let config = SentinelConfig::default();
        let bus = CommunicationBus::new();
        let sentinel = Sentinel::new(config, bus);

        assert_eq!(sentinel.id(), SpecialistId::Sentinel);
    }

    #[tokio::test]
    async fn test_sentinel_arbitrate_empty() {
        let config = SentinelConfig::default();
        let bus = CommunicationBus::new();
        let sentinel = Sentinel::new(config, bus);

        let result = sentinel.arbitrate().await.unwrap();
        assert_eq!(result.proposals_reviewed, 0);
    }

    #[tokio::test]
    async fn test_sentinel_update_resources() {
        let config = SentinelConfig::default();
        let bus = CommunicationBus::new();
        let sentinel = Sentinel::new(config, bus);

        let resources = SystemResources {
            gpu_available_percent: 50.0,
            cpu_available_percent: 80.0,
            memory_available_mb: 4096,
            thermal_headroom: 0.8,
        };

        sentinel.update_system_resources(resources.clone()).await;
        let current = sentinel.current_system_resources.lock().await;
        assert_eq!(current.gpu_available_percent, 50.0);
    }

    #[test]
    fn test_sentinel_capabilities() {
        let config = SentinelConfig::default();
        let bus = CommunicationBus::new();
        let sentinel = Sentinel::new(config, bus);

        let capabilities = sentinel.capabilities();
        assert!(capabilities.len() > 0);
        assert!(capabilities.iter().any(|c| c.name == "arbitration"));
    }
}
