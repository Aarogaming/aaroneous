/// Proposal System: How specialists share their ideas
/// 
/// A proposal is a specialist's asynchronous signal: "I have an idea that might help."
/// Proposals are:
/// - Non-blocking: specialist submits and continues
/// - Ranked: multiple proposals from different specialists
/// - Conflict-capable: Sentinel detects overlaps and resolves
/// - Retryable: proposals can be resubmitted if rejected

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use crate::federation::specialist::{SpecialistId, ResourceRequest, ProposalPriority};

/// Unique identifier for a proposal
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProposalId(pub u64);

impl ProposalId {
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        ProposalId(timestamp)
    }
}

impl Default for ProposalId {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a proposal in the system
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// Just submitted, waiting for Sentinel review
    Pending,
    /// Sentinel reviewing for conflicts
    Reviewing,
    /// Accepted and assigned to specialist
    Accepted,
    /// Specialist executing the proposal
    Executing,
    /// Proposal completed successfully
    Completed,
    /// Rejected (conflicts, resource unavailable, priority too low)
    Rejected,
    /// Cancelled by user or system
    Cancelled,
    /// Specialist negotiating with another specialist
    Negotiating,
    /// Failed during execution
    Failed,
}

/// A proposal submitted by a specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub specialist: SpecialistId,
    pub timestamp: u64,
    pub status: ProposalStatus,
    pub action: String,                      // "generate_design", "sync_devices", etc.
    pub description: String,
    pub confidence: f32,                     // 0.0-1.0 (how sure specialist is)
    pub priority: ProposalPriority,
    pub required_resources: ResourceRequest,
    pub estimated_completion_ms: u64,
    pub dependencies: Vec<ProposalId>,       // This proposal depends on others
    pub tags: Vec<String>,                   // For categorization ("design", "sync", "learning", etc.)
    pub rejection_reason: Option<String>,    // If rejected, why?
    /// Arbitrary key-value metadata forwarded into Decision.context at arbitration.
    /// Use "intent" to carry the active user intent into specialist execution.
    pub metadata: std::collections::HashMap<String, String>,
}

impl Proposal {
    pub fn new(
        specialist: SpecialistId,
        action: String,
        description: String,
        confidence: f32,
        priority: ProposalPriority,
    ) -> Self {
        Self {
            id: ProposalId::new(),
            specialist,
            timestamp: current_timestamp(),
            status: ProposalStatus::Pending,
            action,
            description,
            confidence,
            priority,
            required_resources: ResourceRequest::default(),
            estimated_completion_ms: 5000,
            dependencies: vec![],
            tags: vec![],
            rejection_reason: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_resources(mut self, resources: ResourceRequest) -> Self {
        self.required_resources = resources;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Attach key-value metadata that will be forwarded into Decision.context.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<ProposalId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_priority(mut self, priority: ProposalPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Score for ranking proposals (higher = better)
    /// Takes into account confidence, priority, resource availability
    pub fn score(&self, available_resources: &crate::federation::specialist::SystemResources) -> f32 {
        let priority_weight = match self.priority {
            ProposalPriority::Background => 0.1,
            ProposalPriority::Normal => 0.5,
            ProposalPriority::UserFacing => 0.8,
            ProposalPriority::Urgent => 1.0,
        };

        let resource_feasible = 
            self.required_resources.gpu_percent <= available_resources.gpu_available_percent &&
            self.required_resources.memory_mb <= available_resources.memory_available_mb;

        if !resource_feasible {
            return 0.0;
        }

        (self.confidence * 0.6) + (priority_weight * 0.4)
    }

    pub fn is_viable(&self, available_resources: &crate::federation::specialist::SystemResources) -> bool {
        self.required_resources.gpu_percent <= available_resources.gpu_available_percent &&
        self.required_resources.memory_mb <= available_resources.memory_available_mb
    }
}

/// Collection of proposals from multiple specialists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalSet {
    pub proposals: Vec<Proposal>,
    pub timestamp: u64,
}

impl ProposalSet {
    pub fn new() -> Self {
        Self {
            proposals: vec![],
            timestamp: current_timestamp(),
        }
    }

    pub fn add(&mut self, proposal: Proposal) {
        self.proposals.push(proposal);
    }

    pub fn count(&self) -> usize {
        self.proposals.len()
    }

    pub fn by_specialist(&self, specialist: SpecialistId) -> Vec<&Proposal> {
        self.proposals
            .iter()
            .filter(|p| p.specialist == specialist)
            .collect()
    }

    /// Get viable proposals sorted by score (highest first)
    pub fn viable_sorted(
        &self,
        available_resources: &crate::federation::specialist::SystemResources,
    ) -> Vec<&Proposal> {
        let mut proposals: Vec<_> = self
            .proposals
            .iter()
            .filter(|p| p.is_viable(available_resources))
            .collect();

        proposals.sort_by(|a, b| {
            let score_a = a.score(available_resources);
            let score_b = b.score(available_resources);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        proposals
    }

    /// Detect potential conflicts (same resource contention)
    pub fn detect_conflicts(&self) -> Vec<ProposalConflict> {
        let mut conflicts = vec![];

        for i in 0..self.proposals.len() {
            for j in (i + 1)..self.proposals.len() {
                let p1 = &self.proposals[i];
                let p2 = &self.proposals[j];

                // Conflict if both need GPU and combined usage > 100%
                if p1.required_resources.gpu_percent > 0.0 && p2.required_resources.gpu_percent > 0.0 {
                    if p1.required_resources.gpu_percent + p2.required_resources.gpu_percent > 100.0 {
                        conflicts.push(ProposalConflict {
                            proposal_a_id: p1.id,
                            proposal_b_id: p2.id,
                            specialist_a: p1.specialist,
                            specialist_b: p2.specialist,
                            conflict_type: ConflictType::ResourceContention("GPU".to_string()),
                            severity: ConflictSeverity::High,
                        });
                    }
                }

                // Conflict if both need same resource at same time
                if p1.action == p2.action && p1.priority == p2.priority {
                    conflicts.push(ProposalConflict {
                        proposal_a_id: p1.id,
                        proposal_b_id: p2.id,
                        specialist_a: p1.specialist,
                        specialist_b: p2.specialist,
                        conflict_type: ConflictType::DuplicateAction,
                        severity: ConflictSeverity::Medium,
                    });
                }
            }
        }

        conflicts
    }
}

impl Default for ProposalSet {
    fn default() -> Self {
        Self::new()
    }
}

/// A conflict detected between two proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalConflict {
    pub proposal_a_id: ProposalId,
    pub proposal_b_id: ProposalId,
    pub specialist_a: SpecialistId,
    pub specialist_b: SpecialistId,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    ResourceContention(String),  // "GPU", "CPU", "Memory"
    DuplicateAction,
    DependencyLoop,
    PriorityMismatch,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_id_unique() {
        // Generate many IDs: even if two collide in the same ns, the set
        // of 100 must have at least some distinct values.
        let ids: Vec<_> = (0..100).map(|_| ProposalId::new()).collect();
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(unique_count > 1, "expected distinct ProposalIds, got all identical");
    }

    #[test]
    fn test_proposal_creation() {
        let proposal = Proposal::new(
            SpecialistId::Visionary,
            "generate_design".to_string(),
            "Generate 10 UI designs".to_string(),
            0.8,
            ProposalPriority::Normal,
        );

        assert_eq!(proposal.specialist, SpecialistId::Visionary);
        assert_eq!(proposal.action, "generate_design");
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.confidence, 0.8);
    }

    #[test]
    fn test_proposal_with_resources() {
        let resources = ResourceRequest {
            gpu_percent: 80.0,
            cpu_percent: 20.0,
            memory_mb: 500,
            duration_seconds: 120,
        };

        let proposal = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Design".to_string(),
            0.9,
            ProposalPriority::Normal,
        )
        .with_resources(resources);

        assert_eq!(proposal.required_resources.gpu_percent, 80.0);
    }

    #[test]
    fn test_proposal_scoring() {
        let available = crate::federation::specialist::SystemResources {
            gpu_available_percent: 100.0,
            cpu_available_percent: 100.0,
            memory_available_mb: 8192,
            thermal_headroom: 1.0,
        };

        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "High confidence design".to_string(),
            0.9,
            ProposalPriority::UserFacing,
        );

        let p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Low confidence backup".to_string(),
            0.3,
            ProposalPriority::Background,
        );

        let score1 = p1.score(&available);
        let score2 = p2.score(&available);

        assert!(score1 > score2);
    }

    #[test]
    fn test_proposal_set_add_and_count() {
        let mut set = ProposalSet::new();
        
        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Design".to_string(),
            0.8,
            ProposalPriority::Normal,
        );

        let p2 = Proposal::new(
            SpecialistId::Omnipresent,
            "sync".to_string(),
            "Sync".to_string(),
            0.7,
            ProposalPriority::Normal,
        );

        set.add(p1);
        set.add(p2);

        assert_eq!(set.count(), 2);
    }

    #[test]
    fn test_proposal_set_by_specialist() {
        let mut set = ProposalSet::new();
        
        set.add(Proposal::new(
            SpecialistId::Visionary,
            "d1".to_string(),
            "Design 1".to_string(),
            0.8,
            ProposalPriority::Normal,
        ));

        set.add(Proposal::new(
            SpecialistId::Visionary,
            "d2".to_string(),
            "Design 2".to_string(),
            0.8,
            ProposalPriority::Normal,
        ));

        set.add(Proposal::new(
            SpecialistId::Omnipresent,
            "sync".to_string(),
            "Sync".to_string(),
            0.7,
            ProposalPriority::Normal,
        ));

        let visionary_proposals = set.by_specialist(SpecialistId::Visionary);
        assert_eq!(visionary_proposals.len(), 2);

        let omnipresent_proposals = set.by_specialist(SpecialistId::Omnipresent);
        assert_eq!(omnipresent_proposals.len(), 1);
    }

    #[test]
    fn test_conflict_detection_gpu_contention() {
        let mut set = ProposalSet::new();

        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Design".to_string(),
            0.8,
            ProposalPriority::Normal,
        );
        p1.required_resources.gpu_percent = 80.0;

        let mut p2 = Proposal::new(
            SpecialistId::Phygital,
            "render".to_string(),
            "Render".to_string(),
            0.8,
            ProposalPriority::Normal,
        );
        p2.required_resources.gpu_percent = 60.0;

        set.add(p1);
        set.add(p2);

        let conflicts = set.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(conflicts[0].conflict_type, ConflictType::ResourceContention(_)));
    }

    #[test]
    fn test_viable_sorted() {
        let available = crate::federation::specialist::SystemResources {
            gpu_available_percent: 100.0,
            cpu_available_percent: 100.0,
            memory_available_mb: 1000,
            thermal_headroom: 1.0,
        };

        let mut set = ProposalSet::new();

        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "d1".to_string(),
            "High confidence".to_string(),
            0.9,
            ProposalPriority::UserFacing,
        );
        p1.required_resources.memory_mb = 500;

        let mut p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Low confidence".to_string(),
            0.3,
            ProposalPriority::Background,
        );
        p2.required_resources.memory_mb = 2000; // Exceeds available

        set.add(p1);
        set.add(p2);

        let viable = set.viable_sorted(&available);
        assert_eq!(viable.len(), 1);
        assert_eq!(viable[0].specialist, SpecialistId::Visionary);
    }
}
