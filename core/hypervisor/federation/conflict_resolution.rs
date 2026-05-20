/// Conflict Resolution: How Sentinel arbitrates between proposals
/// 
/// Sentinel's core responsibility: when two proposals can't both run,
/// decide which one takes priority. Uses scoring, negotiation, and
/// resource allocation strategies.

use serde::{Deserialize, Serialize};
use crate::federation::specialist::{SpecialistId, SystemResources, ProposalPriority};
use crate::federation::proposal::{Proposal, ProposalConflict, ConflictType, ConflictSeverity};

/// Result of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub resolved: bool,
    pub winner: Option<SpecialistId>,
    pub loser: Option<SpecialistId>,
    pub strategy: ResolutionStrategy,
    pub reasoning: String,
    pub compromises: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    /// Higher priority wins
    PriorityBased,
    /// Higher confidence wins
    ConfidenceBased,
    /// Negotiate a compromise
    NegotiationBased,
    /// Split resources between both
    ResourceSharing,
    /// Sequential execution (queue one)
    Sequential,
}

/// Detects conflicts in proposal sets
pub struct ConflictDetector;

impl ConflictDetector {
    /// Find all conflicts in a proposal set
    pub fn detect(proposals: &crate::federation::proposal::ProposalSet) -> Vec<ProposalConflict> {
        proposals.detect_conflicts()
    }

    /// Classify conflict severity
    pub fn severity(conflict: &ProposalConflict) -> ConflictSeverity {
        conflict.severity
    }

    /// Check if two proposals conflict on GPU usage
    pub fn gpu_conflict(p1: &Proposal, p2: &Proposal) -> bool {
        if p1.required_resources.gpu_percent > 0.0 && p2.required_resources.gpu_percent > 0.0 {
            p1.required_resources.gpu_percent + p2.required_resources.gpu_percent > 100.0
        } else {
            false
        }
    }

    /// Check if two proposals conflict on memory usage
    pub fn memory_conflict(p1: &Proposal, p2: &Proposal) -> bool {
        p1.required_resources.memory_mb + p2.required_resources.memory_mb > 16384 // Assume 16GB max
    }

    /// Check if two proposals are duplicates
    pub fn duplicate_conflict(p1: &Proposal, p2: &Proposal) -> bool {
        p1.action == p2.action && p1.specialist != p2.specialist
    }
}

/// Allocates resources among competing proposals
pub struct ResourceAllocation;

impl ResourceAllocation {
    /// Determine how to split GPU between two proposals
    pub fn gpu_allocation(
        p1: &Proposal,
        p2: &Proposal,
        available: f32,
    ) -> (f32, f32) {
        let needed = p1.required_resources.gpu_percent + p2.required_resources.gpu_percent;
        
        if needed <= available {
            // Both can run fully
            (p1.required_resources.gpu_percent, p2.required_resources.gpu_percent)
        } else {
            // Allocate proportionally by priority and confidence
            let total_score = Self::proposal_score(p1) + Self::proposal_score(p2);
            let p1_score = Self::proposal_score(p1);
            let p2_score = Self::proposal_score(p2);
            
            (
                (p1_score / total_score) * available,
                (p2_score / total_score) * available,
            )
        }
    }

    fn proposal_score(p: &Proposal) -> f32 {
        let priority_weight = match p.priority {
            ProposalPriority::Background => 0.1,
            ProposalPriority::Normal => 0.5,
            ProposalPriority::UserFacing => 0.8,
            ProposalPriority::Urgent => 1.0,
        };
        (p.confidence * 0.6) + (priority_weight * 0.4)
    }

    /// Determine memory allocation
    pub fn memory_allocation(
        p1: &Proposal,
        p2: &Proposal,
        available: u32,
    ) -> (u32, u32) {
        let needed = p1.required_resources.memory_mb + p2.required_resources.memory_mb;
        
        if needed <= available {
            (p1.required_resources.memory_mb, p2.required_resources.memory_mb)
        } else {
            // Allocate proportionally
            let total = needed;
            let p1_ratio = p1.required_resources.memory_mb as f32 / total as f32;
            let p2_ratio = p2.required_resources.memory_mb as f32 / total as f32;
            
            (
                (p1_ratio * available as f32) as u32,
                (p2_ratio * available as f32) as u32,
            )
        }
    }
}

/// Sentinel's arbitration logic
pub struct ConflictArbitrator;

impl ConflictArbitrator {
    /// Resolve a conflict between two proposals
    pub fn resolve(
        conflict: &ProposalConflict,
        p1: &Proposal,
        p2: &Proposal,
        available: &SystemResources,
    ) -> ConflictResolution {
        match conflict.conflict_type {
            ConflictType::ResourceContention(ref resource) => {
                Self::resolve_resource_contention(resource, p1, p2, available)
            }
            ConflictType::DuplicateAction => {
                Self::resolve_duplicate_action(p1, p2)
            }
            ConflictType::DependencyLoop => {
                Self::resolve_dependency_loop(p1, p2)
            }
            ConflictType::PriorityMismatch => {
                Self::resolve_priority_mismatch(p1, p2)
            }
        }
    }

    fn resolve_resource_contention(
        resource: &str,
        p1: &Proposal,
        p2: &Proposal,
        available: &SystemResources,
    ) -> ConflictResolution {
        match resource {
            "GPU" => {
                let p1_score = Self::proposal_score(p1);
                let p2_score = Self::proposal_score(p2);

                if p1_score > p2_score {
                    ConflictResolution {
                        conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                        resolved: true,
                        winner: Some(p1.specialist),
                        loser: Some(p2.specialist),
                        strategy: ResolutionStrategy::PriorityBased,
                        reasoning: format!(
                            "{:?} has higher score ({:.2}) than {:?} ({:.2})",
                            p1.specialist, p1_score, p2.specialist, p2_score
                        ),
                        compromises: vec![],
                    }
                } else if p2_score > p1_score {
                    ConflictResolution {
                        conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                        resolved: true,
                        winner: Some(p2.specialist),
                        loser: Some(p1.specialist),
                        strategy: ResolutionStrategy::PriorityBased,
                        reasoning: format!(
                            "{:?} has higher score ({:.2}) than {:?} ({:.2})",
                            p2.specialist, p2_score, p1.specialist, p1_score
                        ),
                        compromises: vec![],
                    }
                } else {
                    // Try resource sharing
                    let (gpu1, gpu2) = ResourceAllocation::gpu_allocation(
                        p1,
                        p2,
                        available.gpu_available_percent,
                    );

                    ConflictResolution {
                        conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                        resolved: true,
                        winner: None,
                        loser: None,
                        strategy: ResolutionStrategy::ResourceSharing,
                        reasoning: format!(
                            "Both specialists share GPU: {:?} gets {:.1}%, {:?} gets {:.1}%",
                            p1.specialist, gpu1, p2.specialist, gpu2
                        ),
                        compromises: vec![
                            format!("{:?} gets {:.1}% GPU", p1.specialist, gpu1),
                            format!("{:?} gets {:.1}% GPU", p2.specialist, gpu2),
                        ],
                    }
                }
            }
            _ => ConflictResolution {
                conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                resolved: false,
                winner: None,
                loser: None,
                strategy: ResolutionStrategy::NegotiationBased,
                reasoning: format!("Unknown resource type: {}", resource),
                compromises: vec![],
            },
        }
    }

    fn resolve_duplicate_action(p1: &Proposal, p2: &Proposal) -> ConflictResolution {
        let p1_score = Self::proposal_score(p1);
        let p2_score = Self::proposal_score(p2);

        if p1_score >= p2_score {
            ConflictResolution {
                conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                resolved: true,
                winner: Some(p1.specialist),
                loser: Some(p2.specialist),
                strategy: ResolutionStrategy::ConfidenceBased,
                reasoning: format!(
                    "{:?} has equal or higher confidence for '{}' action",
                    p1.specialist, p1.action
                ),
                compromises: vec![format!(
                    "Queue {:?}'s proposal if {:?}'s completes",
                    p2.specialist, p1.specialist
                )],
            }
        } else {
            ConflictResolution {
                conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                resolved: true,
                winner: Some(p2.specialist),
                loser: Some(p1.specialist),
                strategy: ResolutionStrategy::ConfidenceBased,
                reasoning: format!(
                    "{:?} has higher confidence for '{}' action",
                    p2.specialist, p2.action
                ),
                compromises: vec![format!(
                    "Queue {:?}'s proposal if {:?}'s completes",
                    p1.specialist, p2.specialist
                )],
            }
        }
    }

    fn resolve_dependency_loop(p1: &Proposal, p2: &Proposal) -> ConflictResolution {
        ConflictResolution {
            conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
            resolved: false,
            winner: None,
            loser: None,
            strategy: ResolutionStrategy::NegotiationBased,
            reasoning: "Circular dependency detected. Specialists must renegotiate.".to_string(),
            compromises: vec!["Break dependency loop through human intervention or specialist negotiation".to_string()],
        }
    }

    fn resolve_priority_mismatch(p1: &Proposal, p2: &Proposal) -> ConflictResolution {
        if p1.priority >= p2.priority {
            ConflictResolution {
                conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                resolved: true,
                winner: Some(p1.specialist),
                loser: Some(p2.specialist),
                strategy: ResolutionStrategy::PriorityBased,
                reasoning: format!(
                    "{:?} has priority {:?}, {:?} has priority {:?}",
                    p1.specialist, p1.priority, p2.specialist, p2.priority
                ),
                compromises: vec![],
            }
        } else {
            ConflictResolution {
                conflict_id: format!("{:?}-{:?}", p1.id, p2.id),
                resolved: true,
                winner: Some(p2.specialist),
                loser: Some(p1.specialist),
                strategy: ResolutionStrategy::PriorityBased,
                reasoning: format!(
                    "{:?} has priority {:?}, {:?} has priority {:?}",
                    p2.specialist, p2.priority, p1.specialist, p1.priority
                ),
                compromises: vec![],
            }
        }
    }

    fn proposal_score(p: &Proposal) -> f32 {
        let priority_weight = match p.priority {
            ProposalPriority::Background => 0.1,
            ProposalPriority::Normal => 0.5,
            ProposalPriority::UserFacing => 0.8,
            ProposalPriority::Urgent => 1.0,
        };
        (p.confidence * 0.6) + (priority_weight * 0.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::specialist::ProposalPriority;

    #[test]
    fn test_gpu_conflict_detection() {
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

        assert!(ConflictDetector::gpu_conflict(&p1, &p2));
    }

    #[test]
    fn test_gpu_allocation() {
        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Design".to_string(),
            0.8,
            ProposalPriority::UserFacing,
        );
        p1.required_resources.gpu_percent = 40.0;

        let mut p2 = Proposal::new(
            SpecialistId::Phygital,
            "render".to_string(),
            "Render".to_string(),
            0.6,
            ProposalPriority::Normal,
        );
        p2.required_resources.gpu_percent = 40.0;

        let (gpu1, gpu2) = ResourceAllocation::gpu_allocation(&p1, &p2, 100.0);
        
        // Both proposals should get allocated GPU proportionally
        assert!(gpu1 > 0.0);
        assert!(gpu2 > 0.0);
        assert!(gpu1 + gpu2 <= 100.1); // Allow small floating point error
    }

    #[test]
    fn test_duplicate_action_resolution() {
        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "backup".to_string(),
            "Backup".to_string(),
            0.9,
            ProposalPriority::Normal,
        );

        let mut p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Backup".to_string(),
            0.7,
            ProposalPriority::Normal,
        );

        let conflict = ProposalConflict {
            proposal_a_id: p1.id,
            proposal_b_id: p2.id,
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Archivist,
            conflict_type: ConflictType::DuplicateAction,
            severity: ConflictSeverity::Medium,
        };

        let resolution = ConflictArbitrator::resolve(
            &conflict,
            &p1,
            &p2,
            &SystemResources::default(),
        );

        assert!(resolution.resolved);
        assert_eq!(resolution.winner, Some(SpecialistId::Visionary));
    }

    #[test]
    fn test_priority_resolution() {
        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "action".to_string(),
            "Action".to_string(),
            0.5,
            ProposalPriority::Urgent,
        );

        let p2 = Proposal::new(
            SpecialistId::Archivist,
            "other".to_string(),
            "Other".to_string(),
            0.9,
            ProposalPriority::Background,
        );

        let conflict = ProposalConflict {
            proposal_a_id: p1.id,
            proposal_b_id: p2.id,
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Archivist,
            conflict_type: ConflictType::PriorityMismatch,
            severity: ConflictSeverity::Medium,
        };

        let resolution = ConflictArbitrator::resolve(
            &conflict,
            &p1,
            &p2,
            &SystemResources::default(),
        );

        assert!(resolution.resolved);
        assert_eq!(resolution.winner, Some(SpecialistId::Visionary));
    }
}
