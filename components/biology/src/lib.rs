pub mod biology;
pub mod metabolic_governor;
pub mod thermodynamic_governor;
pub use biology::{SystemBiology, SpecialistMetabolism, ThrottleState, SystemHealthReport, SpecialistHealth};
pub use metabolic_governor::{PredictiveMetabolicGovernor, MetabolicGovernorConfig, MetabolicForecast, GovernanceAction};
pub use thermodynamic_governor::{ThermodynamicGovernor, ThermodynamicGovernorConfig, ThermodynamicForecast, ThermodynamicAction};
