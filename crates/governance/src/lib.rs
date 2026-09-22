pub mod constraint_inspector;
pub mod interference_checker;
pub mod jit_audit;
pub mod lattice_verifier;
#[cfg(feature = "adaptive-governance")]
pub mod load_governor;
pub mod metrics_exporter;
pub mod resource_governor;
pub mod rollback_journal;
pub mod smt_action_interlock;
pub mod system_limits;
#[cfg(feature = "adaptive-governance")]
pub mod throughput_governor;

pub use constraint_inspector::{ConstraintInspectionReport, ConstraintInspector};
pub use interference_checker::{InterferenceChecker, MAX_HARDWARE_REGISTER, NonInterferenceReport};
pub use lattice_verifier::{LatticeVerifier, VerificationReport};
#[cfg(feature = "adaptive-governance")]
pub use load_governor::{
    GovernanceAction, LoadForecast, LoadGovernorConfig, PredictiveLoadGovernor,
};
pub use metrics_exporter::{
    InMemoryMetricsSink, MetricObservation, UniversalMetricsExporter, UniversalMetricsSink,
};
pub use resource_governor::{DynamicEquilibriumState, FeedbackRegulator, ResourceGovernor};
pub use rollback_journal::{GenerationSnapshot, GenerationalJournal};
pub use smt_action_interlock::{GovernanceError, InterlockAuditCertificate, SmtActionInterlock};
pub use system_limits::{
    ExecutionBias, SpecialistBudget, SpecialistHealth, SystemHealthGovernor, SystemHealthReport,
    ThrottleState,
};
#[cfg(feature = "adaptive-governance")]
pub use throughput_governor::{
    AdaptiveAction, AdaptiveForecast, AdaptiveGovernor, AdaptiveGovernorConfig,
};
