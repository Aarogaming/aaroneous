//! `cargo xtask status`: a read-only summary of facts that can be derived from
//! the workspace itself, so status documents can cite one command instead of
//! copying numbers into prose.
//!
//! Inputs are explicit (`--workspace-root`, `--git-revision`), matching
//! `preflight` and `workspace-metadata`: the command reads no environment and
//! uses no network. It does not run any check, so it reports what the gate
//! contains, never whether the gate passes.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::gate::{ALL_GATE_CHECKS, validate_target_rev, validate_workspace_root};
use crate::workspace_metadata::{CrateInfo, build_manifest};

/// Label used for packages that declare no `[package.metadata.cratify] profile`.
const NO_PROFILE: &str = "(none declared)";

/// Facts derived from the workspace at one revision.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StatusFacts {
    pub revision: String,
    pub workspace_version: String,
    pub package_count: usize,
    pub profile_counts: BTreeMap<String, usize>,
    pub packages_without_profile: Vec<String>,
    pub gate_checks_by_tier: BTreeMap<String, usize>,
    pub gate_check_total: usize,
    pub manifest_fingerprint: String,
}

/// Read `[workspace.package] version` from the root manifest.
fn read_workspace_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("Reading {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&text).with_context(|| format!("Parsing {}", path.display()))?;
    match value
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
    {
        Some(v) => Ok(v.to_string()),
        None => bail!(
            "{} has no [workspace.package] version; the version source of truth is missing",
            path.display()
        ),
    }
}

/// Pure reduction from already-collected inputs to the reported facts.
pub fn facts_from(
    revision: &str,
    workspace_version: &str,
    crates: &BTreeMap<String, CrateInfo>,
    manifest_fingerprint: &str,
) -> StatusFacts {
    let mut profile_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut packages_without_profile = Vec::new();
    for (name, info) in crates {
        match info.cratify_profile.as_deref() {
            Some(profile) => *profile_counts.entry(profile.to_string()).or_insert(0) += 1,
            None => {
                *profile_counts.entry(NO_PROFILE.to_string()).or_insert(0) += 1;
                packages_without_profile.push(name.clone());
            }
        }
    }

    let mut gate_checks_by_tier: BTreeMap<String, usize> = BTreeMap::new();
    for check in ALL_GATE_CHECKS {
        *gate_checks_by_tier
            .entry(check.tier.as_str().to_string())
            .or_insert(0) += 1;
    }

    StatusFacts {
        revision: revision.to_string(),
        workspace_version: workspace_version.to_string(),
        package_count: crates.len(),
        profile_counts,
        packages_without_profile,
        gate_checks_by_tier,
        gate_check_total: ALL_GATE_CHECKS.len(),
        manifest_fingerprint: manifest_fingerprint.to_string(),
    }
}

/// Human-readable rendering. Ends with what is deliberately not collected.
pub fn render(facts: &StatusFacts) -> String {
    let mut out = String::new();
    out.push_str("Workspace status (derived from the checkout; no network, no checks run)\n");
    out.push_str(&format!("  revision:          {}\n", facts.revision));
    out.push_str(&format!(
        "  workspace version: {} (Cargo.toml [workspace.package], the version source of truth)\n",
        facts.workspace_version
    ));
    out.push_str(&format!("  packages:          {}\n", facts.package_count));
    for (profile, count) in &facts.profile_counts {
        out.push_str(&format!("    profile {profile}: {count}\n"));
    }
    if !facts.packages_without_profile.is_empty() {
        out.push_str(&format!(
            "  packages with no declared profile: {}\n",
            facts.packages_without_profile.join(", ")
        ));
    }
    out.push_str(&format!(
        "  gate checks:       {} (see AGENTS.md section 6 for the numbered list)\n",
        facts.gate_check_total
    ));
    for (tier, count) in &facts.gate_checks_by_tier {
        out.push_str(&format!("    {tier}: {count}\n"));
    }
    out.push_str(&format!(
        "  manifest fingerprint: {}\n",
        facts.manifest_fingerprint
    ));
    out.push_str(
        "Not collected here: open pull requests, CI results, test outcomes, whether the gate passes.\n\
         Those need the network or a run; use `gh` or `cargo xtask gate`.\n",
    );
    out
}

/// Entry point for `cargo xtask status`.
pub fn run(args: &[String]) -> Result<()> {
    let mut root: Option<PathBuf> = None;
    let mut revision: Option<String> = None;
    let mut json = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--workspace-root" => {
                i += 1;
                match args.get(i) {
                    Some(v) => root = Some(PathBuf::from(v)),
                    None => bail!("--workspace-root requires an argument"),
                }
            }
            "--git-revision" => {
                i += 1;
                match args.get(i) {
                    Some(v) => revision = Some(v.clone()),
                    None => bail!("--git-revision requires an argument"),
                }
            }
            "--json" => json = true,
            other => bail!(
                "Unknown argument to status: '{other}'. Usage: cargo xtask status --workspace-root <PATH> --git-revision <REV> [--json]"
            ),
        }
        i += 1;
    }

    let root = root.ok_or_else(|| {
        anyhow::anyhow!("Missing required --workspace-root argument (zero ambient authority)")
    })?;
    let revision = revision.ok_or_else(|| {
        anyhow::anyhow!("Missing required --git-revision argument (zero ambient authority)")
    })?;

    let root = validate_workspace_root(&root)?;
    let revision = validate_target_rev(&root, &revision)?;
    let version = read_workspace_version(&root)?;
    let manifest = build_manifest(&root, &revision)?;
    let facts = facts_from(&revision, &version, &manifest.crates, &manifest.fingerprint);

    if json {
        println!("{}", serde_json::to_string_pretty(&facts)?);
    } else {
        print!("{}", render(&facts));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crate_info(name: &str, profile: Option<&str>) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            path: format!("crates/{name}"),
            cratify_profile: profile.map(str::to_string),
            ring: None,
            profile: Vec::new(),
            description: String::new(),
            dependencies: Vec::new(),
        }
    }

    fn sample_crates() -> BTreeMap<String, CrateInfo> {
        let mut m = BTreeMap::new();
        for (name, profile) in [
            ("a", Some("kernel")),
            ("b", Some("kernel")),
            ("c", Some("control")),
            ("d", None),
        ] {
            m.insert(name.to_string(), crate_info(name, profile));
        }
        m
    }

    #[test]
    fn facts_count_packages_and_profiles_from_the_manifest() {
        let facts = facts_from("abc", "9.9.9", &sample_crates(), "fp");
        assert_eq!(facts.package_count, 4);
        assert_eq!(facts.profile_counts.get("kernel"), Some(&2));
        assert_eq!(facts.profile_counts.get("control"), Some(&1));
        assert_eq!(facts.profile_counts.get(NO_PROFILE), Some(&1));
        assert_eq!(facts.packages_without_profile, vec!["d".to_string()]);
        assert_eq!(facts.workspace_version, "9.9.9");
    }

    #[test]
    fn gate_counts_come_from_the_gate_table_and_add_up() {
        let facts = facts_from("abc", "1.0.0", &BTreeMap::new(), "fp");
        assert_eq!(facts.gate_check_total, ALL_GATE_CHECKS.len());
        let tier_sum: usize = facts.gate_checks_by_tier.values().sum();
        assert_eq!(tier_sum, facts.gate_check_total);
        assert!(facts.gate_check_total > 0);
    }

    #[test]
    fn facts_are_deterministic_for_equal_inputs() {
        let a = facts_from("abc", "1.0.0", &sample_crates(), "fp");
        let b = facts_from("abc", "1.0.0", &sample_crates(), "fp");
        assert_eq!(a, b);
        assert_eq!(render(&a), render(&b));
    }

    #[test]
    fn render_names_the_version_source_and_what_is_not_collected() {
        let text = render(&facts_from("abc", "9.9.9", &sample_crates(), "fp"));
        assert!(text.contains("9.9.9"));
        assert!(text.contains("source of truth"));
        assert!(text.contains("profile kernel: 2"));
        assert!(text.contains("packages with no declared profile: d"));
        assert!(text.contains("Not collected here"));
    }

    #[test]
    fn workspace_version_is_read_from_workspace_package() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.2.3+x\"\n",
        )?;
        assert_eq!(read_workspace_version(dir.path())?, "1.2.3+x");
        Ok(())
    }

    #[test]
    fn missing_workspace_version_is_an_error_not_a_default() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n")?;
        assert!(read_workspace_version(dir.path()).is_err());
        Ok(())
    }

    #[test]
    fn run_requires_explicit_root_and_revision() {
        assert!(run(&[]).is_err());
        assert!(run(&["--workspace-root".to_string()]).is_err());
        assert!(run(&["--bogus".to_string()]).is_err());
        let only_root = vec!["--workspace-root".to_string(), ".".to_string()];
        assert!(run(&only_root).is_err());
    }
}
