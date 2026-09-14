//! Cratify — Universal Sovereign Cratification CLI & Orchestration Bridge.
//!
//! Re-exports modular capabilities from `ast_auditor`, `transpiler`, and `adaptation_engine`.

#![deny(unsafe_code)]

pub use ast_auditor;
pub use transpiler;
pub use adaptation_engine;

// Convenience re-exports
pub use ast_auditor::{run_workspace_audit, UnifiedAuditReport};
pub use transpiler::{create_crate_scaffold, create_crate_scaffold_at, scaffold_crate, scaffold_crate_at};
pub use adaptation_engine::{harvest, harvest_path, CrateSpec, HarvestConfig, ParseStrategy};

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
