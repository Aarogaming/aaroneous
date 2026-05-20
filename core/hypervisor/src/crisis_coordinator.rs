// Aaroneous Crisis Coordinator
// Emergency response orchestration across the specialist hive
// Assembles crisis teams, allocates resources, coordinates real-time response

use crate::capability_dashboard::CapabilityDashboard;
use crate::skill_system::SoulRank;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Crisis severity level
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CrisisSeverity {
    Routine = 1,      // difficulty 1.0
    Moderate = 2,     // difficulty 2.0-2.5
    High = 3,         // difficulty 3.0-3.5
    Critical = 4,     // difficulty 4.0-4.5
    Catastrophic = 5, // difficulty 5.0
}

impl CrisisSeverity {
    pub fn difficulty_multiplier(&self) -> f64 {
        match self {
            CrisisSeverity::Routine => 1.0,
            CrisisSeverity::Moderate => 2.25,
            CrisisSeverity::High => 3.25,
            CrisisSeverity::Critical => 4.25,
            CrisisSeverity::Catastrophic => 5.0,
        }
    }

    pub fn min_team_size(&self) -> usize {
        match self {
            CrisisSeverity::Routine => 1,
            CrisisSeverity::Moderate => 2,
            CrisisSeverity::High => 3,
            CrisisSeverity::Critical => 4,
            CrisisSeverity::Catastrophic => 5,
        }
    }

    pub fn min_rank_required(&self) -> SoulRank {
        match self {
            CrisisSeverity::Routine => SoulRank::Rank1NovellyDigested,
            CrisisSeverity::Moderate => SoulRank::Rank2IntegratedSpecialist,
            CrisisSeverity::High => SoulRank::Rank3Journeyman,
            CrisisSeverity::Critical => SoulRank::Rank4Master,
            CrisisSeverity::Catastrophic => SoulRank::Rank4Master,
        }
    }
}

/// Crisis incident
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisIncident {
    pub incident_id: String,
    pub name: String,
    pub description: String,
    pub severity: CrisisSeverity,
    pub affected_areas: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub status: CrisisStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CrisisStatus {
    Detected,
    Escalated,
    Responding,
    Contained,
    Resolved,
    Archived,
}

impl CrisisIncident {
    pub fn new(name: String, description: String, severity: CrisisSeverity) -> Self {
        Self {
            incident_id: format!("crisis_{}", uuid::Uuid::new_v4()),
            name,
            description,
            severity,
            affected_areas: Vec::new(),
            created_at: Utc::now(),
            status: CrisisStatus::Detected,
        }
    }

    pub fn duration(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
}

/// Crisis response team
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisResponseTeam {
    pub team_id: String,
    pub incident_id: String,
    pub lead_specialist_id: String,
    pub team_members: Vec<String>,
    pub specialist_roles: HashMap<String, String>, // specialist_id -> role
    pub assembled_at: DateTime<Utc>,
    pub status: TeamStatus,
    pub xp_multiplier: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TeamStatus {
    Assembling,
    Ready,
    Responding,
    Coordinating,
    Standby,
    Complete,
}

impl CrisisResponseTeam {
    pub fn new(
        incident_id: String,
        lead_specialist_id: String,
        team_members: Vec<String>,
        severity: CrisisSeverity,
    ) -> Self {
        let base_multiplier = severity.difficulty_multiplier() / 2.5; // Normalize
        Self {
            team_id: format!("team_{}", uuid::Uuid::new_v4()),
            incident_id,
            lead_specialist_id,
            team_members,
            specialist_roles: HashMap::new(),
            assembled_at: Utc::now(),
            status: TeamStatus::Assembling,
            xp_multiplier: base_multiplier,
        }
    }

    pub fn assign_role(&mut self, specialist_id: String, role: String) {
        self.specialist_roles.insert(specialist_id, role);
    }

    pub fn mark_ready(&mut self) {
        self.status = TeamStatus::Ready;
    }

    pub fn mark_responding(&mut self) {
        self.status = TeamStatus::Responding;
    }

    pub fn mark_complete(&mut self) {
        self.status = TeamStatus::Complete;
    }
}

/// Crisis resolution metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisMetrics {
    pub incident_id: String,
    pub total_specialists_engaged: usize,
    pub total_xp_awarded: u32,
    pub average_xp_per_specialist: u32,
    pub breakthrough_count: u32,
    pub awakening_count: u32,
    pub resolution_time_minutes: u32,
    pub success_rate: f64,
}

impl CrisisMetrics {
    pub fn new(incident_id: String) -> Self {
        Self {
            incident_id,
            total_specialists_engaged: 0,
            total_xp_awarded: 0,
            average_xp_per_specialist: 0,
            breakthrough_count: 0,
            awakening_count: 0,
            resolution_time_minutes: 0,
            success_rate: 0.0,
        }
    }
}

/// Main crisis coordinator
pub struct CrisisCoordinator {
    active_incidents: HashMap<String, CrisisIncident>,
    active_teams: HashMap<String, CrisisResponseTeam>,
    incident_history: Vec<CrisisIncident>,
    team_history: Vec<CrisisResponseTeam>,
    crisis_metrics: HashMap<String, CrisisMetrics>,
}

impl CrisisCoordinator {
    pub fn new() -> Self {
        Self {
            active_incidents: HashMap::new(),
            active_teams: HashMap::new(),
            incident_history: Vec::new(),
            team_history: Vec::new(),
            crisis_metrics: HashMap::new(),
        }
    }

    /// Detect and register a crisis
    pub fn detect_crisis(
        &mut self,
        name: String,
        description: String,
        severity: CrisisSeverity,
    ) -> String {
        let incident = CrisisIncident::new(name, description, severity);
        let incident_id = incident.incident_id.clone();
        self.active_incidents.insert(incident_id.clone(), incident);
        incident_id
    }

    /// Assemble response team from dashboard
    pub fn assemble_team(
        &mut self,
        incident_id: String,
        dashboard: &CapabilityDashboard,
    ) -> Result<String, String> {
        let incident = self
            .active_incidents
            .get(&incident_id)
            .ok_or("Incident not found")?;

        let team_size = incident.severity.min_team_size();
        let min_rank = incident.severity.min_rank_required();

        // Find suitable specialists
        let mut candidates: Vec<_> = dashboard
            .get_all_statuses()
            .into_iter()
            .filter(|s| s.current_rank as u8 >= min_rank as u8)
            .collect();

        if candidates.is_empty() {
            return Err("No specialists meet minimum rank requirement".to_string());
        }

        // Sort by success rate and rank
        candidates.sort_by(|a, b| {
            b.average_success_rate
                .partial_cmp(&a.average_success_rate)
                .unwrap()
                .then_with(|| (b.current_rank as u8).cmp(&(a.current_rank as u8)))
        });

        // Select team
        let lead_specialist = candidates[0].specialist_id.clone();
        let team_members: Vec<String> = candidates
            .into_iter()
            .take(team_size)
            .map(|s| s.specialist_id.clone())
            .collect();

        let mut team = CrisisResponseTeam::new(incident_id.clone(), lead_specialist, team_members, incident.severity);
        team.mark_ready();

        let team_id = team.team_id.clone();
        self.active_teams.insert(team_id.clone(), team);

        Ok(team_id)
    }

    /// Get crisis incident
    pub fn get_incident(&self, incident_id: &str) -> Option<&CrisisIncident> {
        self.active_incidents.get(incident_id)
    }

    /// Get crisis response team
    pub fn get_team(&self, team_id: &str) -> Option<&CrisisResponseTeam> {
        self.active_teams.get(team_id)
    }

    /// Get mutable team for updates
    pub fn get_team_mut(&mut self, team_id: &str) -> Option<&mut CrisisResponseTeam> {
        self.active_teams.get_mut(team_id)
    }

    /// Get teams for an incident
    pub fn get_incident_teams(&self, incident_id: &str) -> Vec<&CrisisResponseTeam> {
        self.active_teams
            .values()
            .filter(|t| t.incident_id == incident_id)
            .collect()
    }

    /// Mark incident as resolved
    pub fn resolve_incident(
        &mut self,
        incident_id: String,
        success: bool,
    ) -> Result<CrisisMetrics, String> {
        let mut incident = self
            .active_incidents
            .remove(&incident_id)
            .ok_or("Incident not found")?;

        incident.status = CrisisStatus::Resolved;

        // Get associated teams
        let mut metrics = CrisisMetrics::new(incident_id.clone());
        metrics.success_rate = if success { 1.0 } else { 0.5 };
        metrics.resolution_time_minutes = (incident.duration().num_minutes()) as u32;

        // Archive teams
        let teams: Vec<_> = self
            .active_teams
            .values()
            .filter(|t| t.incident_id == incident_id)
            .cloned()
            .collect();

        for mut team in teams {
            team.mark_complete();
            metrics.total_specialists_engaged += team.team_members.len();
            self.team_history.push(team);
        }

        // Remove teams from active
        self.active_teams
            .retain(|_, t| t.incident_id != incident_id);

        self.incident_history.push(incident);
        self.crisis_metrics.insert(incident_id, metrics.clone());

        Ok(metrics)
    }

    /// Get active crises count
    pub fn active_crisis_count(&self) -> usize {
        self.active_incidents
            .values()
            .filter(|i| i.status != CrisisStatus::Resolved && i.status != CrisisStatus::Archived)
            .count()
    }

    /// Get critical crises
    pub fn get_critical_crises(&self) -> Vec<&CrisisIncident> {
        self.active_incidents
            .values()
            .filter(|i| i.severity as u8 >= CrisisSeverity::Critical as u8)
            .collect()
    }

    /// Get incident history
    pub fn get_history(&self, limit: usize) -> Vec<&CrisisIncident> {
        self.incident_history
            .iter()
            .rev()
            .take(limit)
            .collect()
    }

    /// Get crisis statistics
    pub fn get_statistics(&self) -> CrisisStatistics {
        let total_incidents = self.incident_history.len() + self.active_incidents.len();
        let resolved = self.incident_history.len();
        let critical = self.active_incidents.values()
            .filter(|i| i.severity == CrisisSeverity::Catastrophic)
            .count();

        let avg_resolution_time = if resolved > 0 {
            self.incident_history
                .iter()
                .map(|i| (i.duration().num_minutes()) as u32)
                .sum::<u32>()
                / resolved as u32
        } else {
            0
        };

        CrisisStatistics {
            total_incidents,
            resolved_incidents: resolved,
            active_incidents: self.active_incidents.len(),
            critical_active: critical,
            average_resolution_minutes: avg_resolution_time,
            total_specialists_engaged: self
                .team_history
                .iter()
                .flat_map(|t| t.team_members.iter())
                .collect::<std::collections::HashSet<_>>()
                .len(),
        }
    }
}

/// Crisis statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrisisStatistics {
    pub total_incidents: usize,
    pub resolved_incidents: usize,
    pub active_incidents: usize,
    pub critical_active: usize,
    pub average_resolution_minutes: u32,
    pub total_specialists_engaged: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crisis_severity_multiplier() {
        assert_eq!(CrisisSeverity::Routine.difficulty_multiplier(), 1.0);
        assert_eq!(CrisisSeverity::Catastrophic.difficulty_multiplier(), 5.0);
        assert!(CrisisSeverity::High.difficulty_multiplier() > CrisisSeverity::Moderate.difficulty_multiplier());
    }

    #[test]
    fn test_crisis_incident_creation() {
        let incident = CrisisIncident::new(
            "Database Crash".to_string(),
            "Primary DB failed".to_string(),
            CrisisSeverity::Critical,
        );

        assert_eq!(incident.severity, CrisisSeverity::Critical);
        assert_eq!(incident.status, CrisisStatus::Detected);
    }

    #[test]
    fn test_crisis_response_team() {
        let team = CrisisResponseTeam::new(
            "crisis_1".to_string(),
            "specialist_1".to_string(),
            vec!["specialist_1".to_string(), "specialist_2".to_string()],
            CrisisSeverity::High,
        );

        assert_eq!(team.team_members.len(), 2);
        assert_eq!(team.status, TeamStatus::Assembling);
        assert!(team.xp_multiplier > 1.0);
    }

    #[test]
    fn test_crisis_coordinator() {
        let mut coordinator = CrisisCoordinator::new();

        let incident_id = coordinator.detect_crisis(
            "API Failure".to_string(),
            "All endpoints down".to_string(),
            CrisisSeverity::Catastrophic,
        );

        assert!(!incident_id.is_empty());
        assert_eq!(coordinator.active_crisis_count(), 1);
    }

    #[test]
    fn test_crisis_resolution() {
        let mut coordinator = CrisisCoordinator::new();

        let incident_id = coordinator.detect_crisis(
            "Service Degradation".to_string(),
            "Slow response times".to_string(),
            CrisisSeverity::Moderate,
        );

        let result = coordinator.resolve_incident(incident_id, true);
        assert!(result.is_ok());
        assert_eq!(coordinator.active_crisis_count(), 0);
    }

    #[test]
    fn test_crisis_statistics() {
        let mut coordinator = CrisisCoordinator::new();

        coordinator.detect_crisis(
            "Incident 1".to_string(),
            "Description".to_string(),
            CrisisSeverity::High,
        );

        let stats = coordinator.get_statistics();
        assert_eq!(stats.total_incidents, 1);
        assert_eq!(stats.active_incidents, 1);
    }
}
