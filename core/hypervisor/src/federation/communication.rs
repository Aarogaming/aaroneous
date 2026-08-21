use crate::federation::proposal::{Proposal, ProposalSet};
use crate::federation::specialist::{Conflict, Decision, DelegateRequest, SpecialistId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialistMessage {
    ProposalSubmitted(Proposal),
    DecisionIssued(Decision),
    DelegationRequest(DelegateRequest),
    DelegationResponse { success: bool, result: String },
    ConflictNotification(Conflict),
    StatusUpdate(String),
    Error(String),
}

#[derive(Clone)]
pub struct MessageChannel {
    tx: mpsc::UnboundedSender<SpecialistMessage>,
    rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SpecialistMessage>>>,
}

impl Default for MessageChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageChannel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Arc::new(tokio::sync::Mutex::new(rx)),
        }
    }
    pub fn send(&self, message: SpecialistMessage) {
        let _ = self.tx.send(message);
    }
    pub async fn receive(&self) -> Option<SpecialistMessage> {
        let mut rx = self.rx.lock().await;
        rx.recv().await
    }
    pub async fn try_receive(&self) -> Option<SpecialistMessage> {
        self.rx.lock().await.try_recv().ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ProposalError {
    #[error("Duplicate proposal ID: {0}")]
    DuplicateId(u64),
    #[error("Confidence out of range [0,1]: {0}")]
    InvalidConfidence(f32),
    #[error("Action cannot be empty")]
    EmptyAction,
    #[error("Resource GPU percent out of range [0,100]: {0}")]
    InvalidGpuPercent(f32),
    #[error("Resource CPU percent out of range [0,100]: {0}")]
    InvalidCpuPercent(f32),
    #[error("Max proposals ({max}) reached for specialist")]
    MaxProposalsReached { max: usize },
}

pub struct CommunicationBus {
    channels: std::collections::HashMap<SpecialistId, MessageChannel>,
    proposals: Vec<Proposal>,
    seen_ids: std::collections::HashSet<u64>,
    proposal_counts: std::collections::HashMap<SpecialistId, usize>,
    max_proposals_per_specialist: usize,
}

impl CommunicationBus {
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashMap::new(),
            proposals: vec![],
            seen_ids: std::collections::HashSet::new(),
            proposal_counts: std::collections::HashMap::new(),
            max_proposals_per_specialist: 10,
        }
    }

    pub fn with_max_proposals(mut self, max: usize) -> Self {
        self.max_proposals_per_specialist = max;
        self
    }

    pub fn register(&mut self, id: SpecialistId) -> MessageChannel {
        let ch = MessageChannel::new();
        self.channels.insert(id, ch.clone());
        ch
    }

    pub fn register_specialist(&mut self, id: SpecialistId) -> MessageChannel {
        self.register(id)
    }

    pub fn broadcast(&self, msg: SpecialistMessage) {
        for ch in self.channels.values() {
            ch.send(msg.clone());
        }
    }

    pub fn specialist_channel(&self, id: &SpecialistId) -> Option<MessageChannel> {
        self.channels.get(id).cloned()
    }

    pub fn specialist_count(&self) -> usize {
        self.channels.len()
    }

    pub fn submit_proposal(&mut self, proposal: Proposal) -> Result<(), ProposalError> {
        let id_val = proposal.id.0;
        if self.seen_ids.contains(&id_val) {
            return Err(ProposalError::DuplicateId(id_val));
        }
        if proposal.action.is_empty() {
            return Err(ProposalError::EmptyAction);
        }
        if !(0.0..=1.0).contains(&proposal.confidence) {
            return Err(ProposalError::InvalidConfidence(proposal.confidence));
        }
        if proposal.required_resources.gpu_percent < 0.0
            || proposal.required_resources.gpu_percent > 100.0
        {
            return Err(ProposalError::InvalidGpuPercent(
                proposal.required_resources.gpu_percent,
            ));
        }
        if proposal.required_resources.cpu_percent < 0.0
            || proposal.required_resources.cpu_percent > 100.0
        {
            return Err(ProposalError::InvalidCpuPercent(
                proposal.required_resources.cpu_percent,
            ));
        }
        let count = self
            .proposal_counts
            .get(&proposal.specialist)
            .copied()
            .unwrap_or(0);
        if count >= self.max_proposals_per_specialist {
            return Err(ProposalError::MaxProposalsReached {
                max: self.max_proposals_per_specialist,
            });
        }
        self.seen_ids.insert(id_val);
        *self.proposal_counts.entry(proposal.specialist).or_insert(0) += 1;
        self.proposals.push(proposal);
        Ok(())
    }

    pub async fn pending_proposals(&self) -> ProposalSet {
        let mut set = ProposalSet::new();
        for p in &self.proposals {
            set.add(p.clone());
        }
        set
    }

    pub async fn clear_proposals(&mut self) {
        self.proposals.clear();
        self.seen_ids.clear();
        self.proposal_counts.clear();
    }

    pub fn pending_count(&self) -> usize {
        self.proposals.len()
    }
}

impl Default for CommunicationBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::proposal::Proposal;
    use crate::federation::specialist::{ProposalPriority, ResourceRequest, SpecialistId};

    fn valid_proposal(specialist: SpecialistId) -> Proposal {
        Proposal {
            id: crate::federation::proposal::ProposalId::new(),
            specialist,
            timestamp: 0,
            status: crate::federation::proposal::ProposalStatus::Pending,
            action: "test_action".to_string(),
            description: "test".to_string(),
            confidence: 0.8,
            priority: ProposalPriority::Normal,
            required_resources: ResourceRequest {
                gpu_percent: 20.0,
                cpu_percent: 10.0,
                memory_mb: 100,
                duration_seconds: 60,
            },
            estimated_completion_ms: 1000,
            dependencies: vec![],
            tags: vec![],
            rejection_reason: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_submit_valid_proposal() {
        let mut bus = CommunicationBus::new();
        let p = valid_proposal(SpecialistId::Visionary);
        assert!(bus.submit_proposal(p).is_ok());
        assert_eq!(bus.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_reject_duplicate_id() {
        let mut bus = CommunicationBus::new();
        let mut p1 = valid_proposal(SpecialistId::Visionary);
        let id = p1.id;
        p1.action = "first".to_string();
        assert!(bus.submit_proposal(p1).is_ok());
        let mut p2 = valid_proposal(SpecialistId::Visionary);
        p2.id = id;
        p2.action = "second".to_string();
        let err = bus.submit_proposal(p2).unwrap_err();
        assert!(matches!(err, ProposalError::DuplicateId(_)));
    }

    #[tokio::test]
    async fn test_reject_empty_action() {
        let mut bus = CommunicationBus::new();
        let mut p = valid_proposal(SpecialistId::Visionary);
        p.action = "".to_string();
        let err = bus.submit_proposal(p).unwrap_err();
        assert!(matches!(err, ProposalError::EmptyAction));
    }

    #[tokio::test]
    async fn test_reject_invalid_confidence() {
        let mut bus = CommunicationBus::new();
        let mut p = valid_proposal(SpecialistId::Visionary);
        p.confidence = 1.5;
        let err = bus.submit_proposal(p).unwrap_err();
        assert!(matches!(err, ProposalError::InvalidConfidence(_)));
    }

    #[tokio::test]
    async fn test_reject_invalid_gpu() {
        let mut bus = CommunicationBus::new();
        let mut p = valid_proposal(SpecialistId::Visionary);
        p.required_resources.gpu_percent = 150.0;
        let err = bus.submit_proposal(p).unwrap_err();
        assert!(matches!(err, ProposalError::InvalidGpuPercent(_)));
    }

    #[tokio::test]
    async fn test_reject_max_proposals() {
        let mut bus = CommunicationBus::new().with_max_proposals(2);
        let p1 = valid_proposal(SpecialistId::Visionary);
        assert!(bus.submit_proposal(p1).is_ok());
        let p2 = valid_proposal(SpecialistId::Visionary);
        assert!(bus.submit_proposal(p2).is_ok());
        let p3 = valid_proposal(SpecialistId::Visionary);
        let err = bus.submit_proposal(p3).unwrap_err();
        assert!(matches!(err, ProposalError::MaxProposalsReached { .. }));
    }

    #[tokio::test]
    async fn test_clear_resets_state() {
        let mut bus = CommunicationBus::new();
        bus.submit_proposal(valid_proposal(SpecialistId::Visionary))
            .unwrap();
        bus.submit_proposal(valid_proposal(SpecialistId::Omnipresent))
            .unwrap();
        assert_eq!(bus.pending_count(), 2);
        bus.clear_proposals().await;
        assert_eq!(bus.pending_count(), 0);
    }
}
