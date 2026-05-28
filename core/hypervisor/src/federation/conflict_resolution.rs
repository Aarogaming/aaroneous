use crate::federation::specialist::{SpecialistId, Conflict, SystemResources};
use crate::federation::proposal::{Proposal, ProposalConflict, ConflictType, ConflictSeverity, ProposalSet};

pub struct ConflictDetector;

impl ConflictDetector {
    pub fn new() -> Self { Self }

    pub fn detect(&self, _conflicts: &[Conflict]) -> Vec<(Proposal, Proposal)> {
        vec![]
    }

    pub fn detect_from_set(&self, set: &ProposalSet) -> Vec<ProposalConflict> {
        let mut conflicts = vec![];

        for i in 0..set.proposals.len() {
            for j in (i + 1)..set.proposals.len() {
                let p1 = &set.proposals[i];
                let p2 = &set.proposals[j];

                if let Some(c) = Self::check_gpu_contention(p1, p2) {
                    conflicts.push(c);
                }

                if let Some(c) = Self::check_cpu_contention(p1, p2) {
                    conflicts.push(c);
                }

                if let Some(c) = Self::check_memory_contention(p1, p2) {
                    conflicts.push(c);
                }

                if let Some(c) = Self::check_duplicate_action(p1, p2) {
                    conflicts.push(c);
                }

                if let Some(c) = Self::check_dependency_loop(p1, p2, set) {
                    conflicts.push(c);
                }
            }
        }

        conflicts
    }

    fn check_gpu_contention(p1: &Proposal, p2: &Proposal) -> Option<ProposalConflict> {
        if p1.required_resources.gpu_percent > 0.0 && p2.required_resources.gpu_percent > 0.0
            && p1.required_resources.gpu_percent + p2.required_resources.gpu_percent > 100.0
        {
            Some(ProposalConflict {
                proposal_a_id: p1.id,
                proposal_b_id: p2.id,
                specialist_a: p1.specialist,
                specialist_b: p2.specialist,
                conflict_type: ConflictType::ResourceContention("GPU".to_string()),
                severity: ConflictSeverity::High,
            })
        } else {
            None
        }
    }

    fn check_cpu_contention(p1: &Proposal, p2: &Proposal) -> Option<ProposalConflict> {
        if p1.required_resources.cpu_percent > 0.0 && p2.required_resources.cpu_percent > 0.0
            && p1.required_resources.cpu_percent + p2.required_resources.cpu_percent > 100.0
        {
            Some(ProposalConflict {
                proposal_a_id: p1.id,
                proposal_b_id: p2.id,
                specialist_a: p1.specialist,
                specialist_b: p2.specialist,
                conflict_type: ConflictType::ResourceContention("CPU".to_string()),
                severity: ConflictSeverity::Medium,
            })
        } else {
            None
        }
    }

    fn check_memory_contention(p1: &Proposal, p2: &Proposal) -> Option<ProposalConflict> {
        if p1.required_resources.memory_mb > 500 && p2.required_resources.memory_mb > 500 {
            Some(ProposalConflict {
                proposal_a_id: p1.id,
                proposal_b_id: p2.id,
                specialist_a: p1.specialist,
                specialist_b: p2.specialist,
                conflict_type: ConflictType::ResourceContention("Memory".to_string()),
                severity: ConflictSeverity::High,
            })
        } else {
            None
        }
    }

    fn check_duplicate_action(p1: &Proposal, p2: &Proposal) -> Option<ProposalConflict> {
        if p1.action == p2.action && p1.priority == p2.priority {
            Some(ProposalConflict {
                proposal_a_id: p1.id,
                proposal_b_id: p2.id,
                specialist_a: p1.specialist,
                specialist_b: p2.specialist,
                conflict_type: ConflictType::DuplicateAction,
                severity: ConflictSeverity::Medium,
            })
        } else {
            None
        }
    }

    fn check_dependency_loop(p1: &Proposal, p2: &Proposal, set: &ProposalSet) -> Option<ProposalConflict> {
        let a_depends_on_b = p1.dependencies.contains(&p2.id);
        let b_depends_on_a = p2.dependencies.contains(&p1.id);

        if a_depends_on_b && b_depends_on_a {
            return Some(ProposalConflict {
                proposal_a_id: p1.id,
                proposal_b_id: p2.id,
                specialist_a: p1.specialist,
                specialist_b: p2.specialist,
                conflict_type: ConflictType::DependencyLoop,
                severity: ConflictSeverity::High,
            });
        }

        if a_depends_on_b {
            for transitive in &set.proposals {
                if transitive.id != p1.id && transitive.id != p2.id
                    && transitive.dependencies.contains(&p1.id)
                    && p2.dependencies.contains(&transitive.id)
                {
                    return Some(ProposalConflict {
                        proposal_a_id: p1.id,
                        proposal_b_id: p2.id,
                        specialist_a: p1.specialist,
                        specialist_b: p2.specialist,
                        conflict_type: ConflictType::DependencyLoop,
                        severity: ConflictSeverity::High,
                    });
                }
            }
        }

        None
    }

    pub fn duplicate_conflict(a: &Proposal, b: &Proposal) -> bool {
        a.action == b.action
    }
}

pub struct ConflictArbitrator;

impl ConflictArbitrator {
    pub fn new() -> Self { Self }

    pub fn arbitrate(&self, conflict: &Conflict) -> ConflictResolution {
        ConflictResolution {
            resolved: true,
            winner: Some(conflict.specialist_a),
            loser: Some(conflict.specialist_b),
            suggestion: None,
        }
    }

    pub fn resolve(_conflict: &ProposalConflict, a: &Proposal, b: &Proposal, _resources: &SystemResources) -> ConflictResolution {
        let (winner, loser) = if a.priority as u8 >= b.priority as u8 {
            (Some(a.specialist), Some(b.specialist))
        } else {
            (Some(b.specialist), Some(a.specialist))
        };
        ConflictResolution { resolved: true, winner, loser, suggestion: None }
    }

    pub fn resolve_with_resources(conflicts: &[ProposalConflict], proposals: &[Proposal], resources: &SystemResources) -> Vec<ConflictResolution> {
        let mut resolutions = vec![];

        for conflict in conflicts {
            let p_a = proposals.iter().find(|p| p.id == conflict.proposal_a_id);
            let p_b = proposals.iter().find(|p| p.id == conflict.proposal_b_id);

            let (p1, p2) = match (p_a, p_b) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };

            let resolution = match &conflict.conflict_type {
                ConflictType::ResourceContention(resource) => {
                    Self::resolve_resource_contention(p1, p2, resource, resources)
                }
                ConflictType::DuplicateAction => {
                    let r = Self::resolve(conflict, p1, p2, resources);
                    ConflictResolution {
                        resolved: r.resolved,
                        winner: r.winner,
                        loser: Some(conflict.specialist_b),
                        suggestion: None,
                    }
                }
                ConflictType::DependencyLoop => {
                    ConflictResolution {
                        resolved: true,
                        winner: Some(conflict.specialist_a),
                        loser: Some(conflict.specialist_b),
                        suggestion: Some("Break circular dependency: run proposal A first, re-evaluate B".to_string()),
                    }
                }
                ConflictType::PriorityMismatch => {
                    Self::resolve(conflict, p1, p2, resources)
                }
            };

            resolutions.push(resolution);
        }

        resolutions
    }

    fn resolve_resource_contention(p1: &Proposal, p2: &Proposal, resource: &str, resources: &SystemResources) -> ConflictResolution {
        let available = match resource {
            "GPU" => resources.gpu_available_percent,
            "CPU" => resources.cpu_available_percent,
            "Memory" => resources.memory_available_mb as f32,
            _ => 0.0,
        };

        let needed_p1 = match resource {
            "GPU" => p1.required_resources.gpu_percent,
            "CPU" => p1.required_resources.cpu_percent,
            "Memory" => p1.required_resources.memory_mb as f32,
            _ => 0.0,
        };

        let needed_p2 = match resource {
            "GPU" => p2.required_resources.gpu_percent,
            "CPU" => p2.required_resources.cpu_percent,
            "Memory" => p2.required_resources.memory_mb as f32,
            _ => 0.0,
        };

        if needed_p1 <= available {
            return ConflictResolution {
                resolved: true,
                winner: Some(p1.specialist),
                loser: Some(p2.specialist),
                suggestion: Some(format!("{:?} can proceed, {:?} deferred", p1.specialist, p2.specialist)),
            };
        }

        if needed_p2 <= available {
            return ConflictResolution {
                resolved: true,
                winner: Some(p2.specialist),
                loser: Some(p1.specialist),
                suggestion: Some(format!("{:?} can proceed, {:?} deferred", p2.specialist, p1.specialist)),
            };
        }

        if p1.priority as u8 >= p2.priority as u8 {
            ConflictResolution {
                resolved: true,
                winner: Some(p1.specialist),
                loser: Some(p2.specialist),
                suggestion: Some(format!("Priority-based: {:?} wins over {:?}", p1.specialist, p2.specialist)),
            }
        } else {
            ConflictResolution {
                resolved: true,
                winner: Some(p2.specialist),
                loser: Some(p1.specialist),
                suggestion: Some(format!("Priority-based: {:?} wins over {:?}", p2.specialist, p1.specialist)),
            }
        }
    }
}

pub struct ResourceAllocation {
    pub specialist_id: SpecialistId,
    pub tokens: f32,
}

pub struct ConflictResolution {
    pub resolved: bool,
    pub winner: Option<SpecialistId>,
    pub loser: Option<SpecialistId>,
    pub suggestion: Option<String>,
}

impl ConflictResolution {
    pub fn unresolved() -> Self {
        Self {
            resolved: false,
            winner: None,
            loser: None,
            suggestion: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::proposal::{Proposal, ProposalId};
    use crate::federation::specialist::{SpecialistId, SystemResources, ResourceRequest, ProposalPriority};

    fn make_proposal(specialist: SpecialistId, action: &str, priority: ProposalPriority, gpu: f32, cpu: f32, mem: u32) -> Proposal {
        Proposal {
            id: ProposalId::new(),
            specialist,
            timestamp: 0,
            action: action.to_string(),
            description: String::new(),
            confidence: 0.8,
            priority,
            required_resources: ResourceRequest {
                gpu_percent: gpu,
                cpu_percent: cpu,
                memory_mb: mem,
                duration_seconds: 60,
            },
            estimated_completion_ms: 1000,
            dependencies: vec![],
            tags: vec![],
            rejection_reason: None,
            metadata: std::collections::HashMap::new(),
            status: crate::federation::proposal::ProposalStatus::Pending,
        }
    }

    #[test]
    fn test_detect_gpu_contention() {
        let a = make_proposal(SpecialistId::Visionary, "render", ProposalPriority::Normal, 70.0, 10.0, 100);
        let b = make_proposal(SpecialistId::Phygital, "track", ProposalPriority::Normal, 60.0, 20.0, 200);

        let mut set = ProposalSet::new();
        set.add(a);
        set.add(b);

        let detector = ConflictDetector::new();
        let conflicts = detector.detect_from_set(&set);

        assert_eq!(conflicts.len(), 1);
        assert!(matches!(conflicts[0].conflict_type, ConflictType::ResourceContention(ref r) if r == "GPU"));
        assert_eq!(conflicts[0].severity, ConflictSeverity::High);
    }

    #[test]
    fn test_detect_cpu_contention() {
        let a = make_proposal(SpecialistId::Archivist, "digest", ProposalPriority::Normal, 0.0, 80.0, 100);
        let b = make_proposal(SpecialistId::Symbiotic, "poll", ProposalPriority::Normal, 0.0, 50.0, 50);

        let mut set = ProposalSet::new();
        set.add(a);
        set.add(b);

        let conflicts = ConflictDetector::new().detect_from_set(&set);

        assert_eq!(conflicts.len(), 1);
        assert!(matches!(conflicts[0].conflict_type, ConflictType::ResourceContention(ref r) if r == "CPU"));
        assert_eq!(conflicts[0].severity, ConflictSeverity::Medium);
    }

    #[test]
    fn test_detect_duplicate_action() {
        let a = make_proposal(SpecialistId::Visionary, "backup", ProposalPriority::Normal, 0.0, 10.0, 50);
        let b = make_proposal(SpecialistId::Archivist, "backup", ProposalPriority::Normal, 0.0, 10.0, 50);

        let mut set = ProposalSet::new();
        set.add(a);
        set.add(b);

        let conflicts = ConflictDetector::new().detect_from_set(&set);

        let dup = conflicts.iter().find(|c| matches!(c.conflict_type, ConflictType::DuplicateAction));
        assert!(dup.is_some(), "should detect duplicate action conflict");
    }

    #[test]
    fn test_detect_dependency_loop() {
        let mut a = make_proposal(SpecialistId::Visionary, "design", ProposalPriority::Normal, 0.0, 10.0, 50);
        let mut b = make_proposal(SpecialistId::Phygital, "build", ProposalPriority::Normal, 0.0, 10.0, 50);

        let a_id = a.id;
        let b_id = b.id;
        a.dependencies.push(b_id);
        b.dependencies.push(a_id);

        let mut set = ProposalSet::new();
        set.add(a);
        set.add(b);

        let conflicts = ConflictDetector::new().detect_from_set(&set);

        let loop_c = conflicts.iter().find(|c| matches!(c.conflict_type, ConflictType::DependencyLoop));
        assert!(loop_c.is_some(), "should detect dependency loop");
    }

    #[test]
    fn test_no_false_positive_independent_proposals() {
        let a = make_proposal(SpecialistId::Visionary, "design", ProposalPriority::Normal, 10.0, 10.0, 50);
        let b = make_proposal(SpecialistId::Archivist, "digest", ProposalPriority::Background, 10.0, 10.0, 50);

        let mut set = ProposalSet::new();
        set.add(a);
        set.add(b);

        let conflicts = ConflictDetector::new().detect_from_set(&set);

        assert!(conflicts.is_empty(), "low resource proposals should not conflict");
    }

    #[test]
    fn test_arbitrate_resolves_by_priority() {
        let conflict = Conflict {
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Archivist,
            conflict_type: "gpu_contention".to_string(),
            context: std::collections::HashMap::new(),
        };

        let arbitrator = ConflictArbitrator::new();
        let resolution = arbitrator.arbitrate(&conflict);

        assert!(resolution.resolved);
        assert_eq!(resolution.winner, Some(SpecialistId::Visionary));
    }

    #[test]
    fn test_resolve_with_resources_gpu_winner_fits() {
        let a = make_proposal(SpecialistId::Visionary, "render", ProposalPriority::Urgent, 50.0, 10.0, 100);
        let b = make_proposal(SpecialistId::Phygital, "track", ProposalPriority::Normal, 70.0, 10.0, 100);

        let conflict = ProposalConflict {
            proposal_a_id: a.id,
            proposal_b_id: b.id,
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Phygital,
            conflict_type: ConflictType::ResourceContention("GPU".to_string()),
            severity: ConflictSeverity::High,
        };

        let resources = SystemResources {
            gpu_available_percent: 50.0,
            ..Default::default()
        };

        let resolutions = ConflictArbitrator::resolve_with_resources(&[conflict], &[a, b], &resources);

        assert_eq!(resolutions.len(), 1);
        assert!(resolutions[0].resolved);
        assert_eq!(resolutions[0].winner, Some(SpecialistId::Visionary));
        assert!(resolutions[0].suggestion.is_some());
    }
}
