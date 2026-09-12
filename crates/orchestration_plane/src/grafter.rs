//! Domain Module Grafter
//!
//! Places harvested and transformed modules into appropriate workspace domain crates
//! (e.g., `crates/compute/src/`, `crates/ipc_bus/src/`) and updates the crate's `src/lib.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::domain_classifier::{Domain, IngestionReport};

/// Grafting outcome report.
#[derive(Debug, Clone)]
pub struct GraftReport {
    pub target_domain: Domain,
    pub module_name: String,
    pub destination_file: PathBuf,
    pub updated_lib_rs: PathBuf,
}

/// Grafts a module into a domain crate based on an `IngestionReport` and remediated source code.
pub fn graft_module(
    report: &IngestionReport,
    source_code: &str,
    workspace_root: &Path,
) -> Result<GraftReport> {
    // 1. Determine target crate path from target domain
    let relative_crate_path = match &report.target_domain {
        Domain::Compute => "crates/compute",
        Domain::Hypervisor => "core/hypervisor",
        Domain::IpcBus => "crates/ipc_bus",
        Domain::Orchestrator => "crates/orchestrator",
        Domain::Novel(_name) => {
            // Scaffold or place into a dedicated staging/novel directory
            "crates/orchestration_plane"
        }
    };

    let target_crate_path = workspace_root.join(relative_crate_path);
    graft_module_to_crate(report, source_code, &target_crate_path)
}

/// Grafts a module directly into a designated target crate directory.
pub fn graft_module_to_crate(
    report: &IngestionReport,
    source_code: &str,
    target_crate_path: &Path,
) -> Result<GraftReport> {
    let src_dir = target_crate_path.join("src");
    if !src_dir.exists() {
        fs::create_dir_all(&src_dir)
            .with_context(|| format!("Failed to create directory {:?}", src_dir))?;
    }

    // Sanitize module file name
    let base_name = Path::new(&report.target)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("harvested_module");
    let sanitized_mod_name = base_name.to_lowercase().replace(['-', ' '], "_");

    let destination_file = src_dir.join(format!("{}.rs", sanitized_mod_name));
    fs::write(&destination_file, source_code)
        .with_context(|| format!("Failed to write grafted module to {:?}", destination_file))?;

    // Update target crate's lib.rs
    let lib_rs = src_dir.join("lib.rs");
    if lib_rs.exists() {
        let existing = fs::read_to_string(&lib_rs)
            .with_context(|| format!("Failed to read {:?}", lib_rs))?;

        let mod_decl = format!("pub mod {};", sanitized_mod_name);
        if !existing.contains(&mod_decl) {
            let updated = format!("{}\n{}\n", mod_decl, existing.trim_start());
            fs::write(&lib_rs, updated)
                .with_context(|| format!("Failed to update {:?}", lib_rs))?;
        }
    } else {
        let content = format!("pub mod {};\n", sanitized_mod_name);
        fs::write(&lib_rs, content)
            .with_context(|| format!("Failed to initialize {:?}", lib_rs))?;
    }

    Ok(GraftReport {
        target_domain: report.target_domain.clone(),
        module_name: sanitized_mod_name,
        destination_file,
        updated_lib_rs: lib_rs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::collections::HashMap;

    #[test]
    fn test_graft_module_to_crate() {
        let temp = tempdir().unwrap();
        let target_crate = temp.path().join("compute");
        fs::create_dir_all(target_crate.join("src")).unwrap();
        fs::write(target_crate.join("src/lib.rs"), "// Compute crate lib\n").unwrap();

        let report = IngestionReport {
            target: "fast_matrix.rs".to_string(),
            target_domain: Domain::Compute,
            affinity_scores: HashMap::new(),
            public_items: vec![],
            detected_dependencies: std::collections::HashSet::new(),
            ambient_risks: vec![],
            required_transformations: vec![],
        };

        let source = "pub fn mul() {}";
        let graft_result = graft_module_to_crate(&report, source, &target_crate).unwrap();

        assert_eq!(graft_result.module_name, "fast_matrix");
        assert!(graft_result.destination_file.exists());
        assert_eq!(
            fs::read_to_string(&graft_result.destination_file).unwrap(),
            source
        );

        let lib_content = fs::read_to_string(&graft_result.updated_lib_rs).unwrap();
        assert!(lib_content.contains("pub mod fast_matrix;"));
    }
}
