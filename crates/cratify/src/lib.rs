//! Cratify — Universal Sovereign Cratification CLI & Orchestration Bridge.
//!
//! Re-exports modular capabilities from `cratify_core`, `transpiler`, and `adaptation_engine`.

#![deny(unsafe_code)]

pub use adaptation_engine;
pub use cratify_core;
pub use transpiler;

// Convenience re-exports
pub use adaptation_engine::{CrateSpec, HarvestConfig, ParseStrategy, harvest, harvest_path};
pub use cratify_core::{UnifiedAuditReport, run_workspace_audit};
pub use transpiler::{
    create_crate_scaffold, create_crate_scaffold_at, scaffold_crate, scaffold_crate_at,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports_availability() {
        let report = UnifiedAuditReport::default();
        assert!(!report.has_failures());

        let config = HarvestConfig::default();
        assert_eq!(config.extensions, vec!["rs".to_string()]);
    }
}
