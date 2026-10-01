//! Native dependency boundary audit (`cargo xtask check-native`).
//!
//! Enforces the checked-in `native-policy.toml` allowlist against the
//! resolved Cargo dependency graph, per target. See that file's header
//! comment for the full schema, and the C68 native-boundary audit (referenced
//! from `docs/CRATIFY_SPEC.md`) for the policy this implements.
//!
//! Four checks run per target:
//!
//! 1. Every native-indicator package (`links`, or a direct dependency on a
//!    known build-tool crate) must have a matching allowlist entry for that
//!    target/feature-set, or the gate fails (undeclared provenance).
//! 2. Every allowlist entry must carry a non-empty `justification` and a
//!    non-empty `owning_adapter` list, or the gate fails (schema violation).
//! 3. For each allowed native package, the nearest workspace-member crate(s)
//!    that reach it in the resolved graph must all be in that entry's
//!    `owning_adapter` list, unless they are a `tooling_profile_crates` crate
//!    (which gets latitude for legitimate build-time tool usage).
//! 4. Targets are read from `native-policy.toml`'s `targets` list rather than
//!    a single hardcoded platform.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

// ---------------------------------------------------------------------------
// `cargo metadata --format-version 1` shapes (only the fields we use).
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Resolve,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    links: Option<String>,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    #[serde(default)]
    deps: Vec<NodeDep>,
    #[serde(default)]
    features: Vec<String>,
}

#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
}

// ---------------------------------------------------------------------------
// `native-policy.toml` shapes.
// ---------------------------------------------------------------------------

fn default_targets() -> Vec<String> {
    vec!["x86_64-pc-windows-msvc".to_string()]
}

#[derive(Deserialize)]
struct RawPolicy {
    #[serde(default = "default_targets")]
    targets: Vec<String>,
    #[serde(default)]
    tooling_profile_crates: Vec<String>,
    #[serde(default)]
    allowed: Vec<RawAllowedPackage>,
}

/// Deliberately permissive at the serde layer (`justification` and
/// `owning_adapter` are `Option`) so that a missing required field produces
/// our own clear, per-entry diagnostic in [`validate_entries`] instead of an
/// opaque serde/toml parse error.
#[derive(Deserialize)]
struct RawAllowedPackage {
    name: String,
    version: String,
    justification: Option<String>,
    owning_adapter: Option<Vec<String>>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    features: Option<Vec<String>>,
}

struct AllowedPackage {
    name: String,
    version: String,
    #[allow(dead_code)]
    justification: String,
    owning_adapter: Vec<String>,
    target: Option<String>,
    features: Option<Vec<String>>,
}

/// Validates that every policy entry carries the required, non-empty
/// `justification` and `owning_adapter` fields, returning all violations at
/// once (rather than failing on the first one) so a reviewer can fix every
/// entry in one pass.
fn validate_entries(raw: Vec<RawAllowedPackage>) -> Result<Vec<AllowedPackage>> {
    let mut errors = Vec::new();
    let mut entries = Vec::new();

    for (i, pkg) in raw.into_iter().enumerate() {
        let justification = pkg.justification.filter(|j| !j.trim().is_empty());
        let owning_adapter = pkg
            .owning_adapter
            .filter(|o| !o.is_empty() && o.iter().any(|s| !s.trim().is_empty()));

        if justification.is_none() {
            errors.push(format!(
                "native-policy.toml allowed[{i}] ({} {}) is missing a required, non-empty `justification`",
                pkg.name, pkg.version
            ));
        }
        if owning_adapter.is_none() {
            errors.push(format!(
                "native-policy.toml allowed[{i}] ({} {}) is missing a required, non-empty `owning_adapter` list",
                pkg.name, pkg.version
            ));
        }

        if let (Some(justification), Some(owning_adapter)) = (justification, owning_adapter) {
            entries.push(AllowedPackage {
                name: pkg.name,
                version: pkg.version,
                justification,
                owning_adapter,
                target: pkg.target,
                features: pkg.features,
            });
        }
    }

    if !errors.is_empty() {
        eprintln!(
            "Found {} native-policy.toml schema violation(s):",
            errors.len()
        );
        for e in &errors {
            eprintln!("  - {e}");
        }
        bail!("Native Dependency Boundary Audit failed: policy schema is incomplete.");
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

/// Fixed indicators of native (C/C++/assembly/build-tool) provenance: a
/// package either declares a Cargo `links` key, or directly depends on one
/// of these well-known native build-tool crates.
const NATIVE_INDICATORS: &[&str] = &[
    "cc",
    "bindgen",
    "libloading",
    "cmake",
    "clang-sys",
    "pkg-config",
    "vcpkg",
];

pub fn run() -> Result<()> {
    println!("=== Native Dependency Boundary Audit ===");

    let policy_str = std::fs::read_to_string("native-policy.toml")
        .context("Failed to read native-policy.toml")?;
    let raw_policy: RawPolicy =
        toml::from_str(&policy_str).context("Failed to parse native-policy.toml")?;

    let targets = if raw_policy.targets.is_empty() {
        default_targets()
    } else {
        raw_policy.targets
    };
    let tooling_profile_crates: HashSet<String> =
        raw_policy.tooling_profile_crates.into_iter().collect();
    let allowed = validate_entries(raw_policy.allowed)?;

    let mut all_violations = Vec::new();

    for target in &targets {
        println!("--- auditing target: {target} ---");
        let violations = audit_target(target, &allowed, &tooling_profile_crates)
            .with_context(|| format!("failed to audit target {target}"))?;
        for v in violations {
            all_violations.push(format!("[{target}] {v}"));
        }
    }

    if !all_violations.is_empty() {
        eprintln!(
            "Found {} native boundary violation(s):",
            all_violations.len()
        );
        for v in &all_violations {
            eprintln!("  - {v}");
        }
        bail!("Native Dependency Boundary Audit failed.");
    }

    println!(
        "Native Dependency Boundary Audit passed ({} target(s), {} allowlist entries).",
        targets.len(),
        allowed.len()
    );
    Ok(())
}

fn audit_target(
    target: &str,
    allowed: &[AllowedPackage],
    tooling_profile_crates: &HashSet<String>,
) -> Result<Vec<String>> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--filter-platform",
            target,
        ])
        .output()
        .with_context(|| format!("Failed to run cargo metadata for target {target}"))?;

    if !output.status.success() {
        bail!(
            "cargo metadata failed for target {target}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let meta: Metadata = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse cargo metadata JSON for target {target}"))?;

    let packages_by_id: HashMap<&str, &Package> =
        meta.packages.iter().map(|p| (p.id.as_str(), p)).collect();
    let workspace_members: HashSet<&str> =
        meta.workspace_members.iter().map(String::as_str).collect();
    let nodes_by_id: HashMap<&str, &Node> = meta
        .resolve
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    // Reverse adjacency: dependency id -> set of ids that directly depend on it.
    let mut reverse_deps: HashMap<&str, HashSet<&str>> = HashMap::new();
    for node in &meta.resolve.nodes {
        for dep in &node.deps {
            reverse_deps
                .entry(dep.pkg.as_str())
                .or_default()
                .insert(node.id.as_str());
        }
    }

    let mut violations = Vec::new();

    for pkg in &meta.packages {
        let mut reasons = Vec::new();
        if let Some(links) = &pkg.links {
            reasons.push(format!("links=\"{links}\""));
        }
        for dep in &pkg.dependencies {
            if NATIVE_INDICATORS.contains(&dep.name.as_str()) {
                reasons.push(format!("depends on {}", dep.name));
            }
        }
        if reasons.is_empty() {
            continue;
        }

        let mut active_features: Vec<String> = nodes_by_id
            .get(pkg.id.as_str())
            .map(|n| n.features.clone())
            .unwrap_or_default();
        active_features.sort();

        let matching_entry = allowed.iter().find(|entry| {
            entry.name == pkg.name
                && entry.version == pkg.version
                && entry.target.as_deref().is_none_or(|t| t == target)
                && entry.features.as_ref().is_none_or(|f| {
                    let mut expected = f.clone();
                    expected.sort();
                    expected == active_features
                })
        });

        let Some(entry) = matching_entry else {
            let feature_note = if active_features.is_empty() {
                String::new()
            } else {
                format!(" (active features: {active_features:?})")
            };
            violations.push(format!(
                "Package {} v{} has undeclared native provenance: {}{feature_note} — no matching \
                 native-policy.toml entry for target {target}",
                pkg.name,
                pkg.version,
                reasons.join(", ")
            ));
            continue;
        };

        // Point 3: only the declared owning_adapter(s) — or a tooling-profile
        // crate exercising its build-time latitude — may be the nearest
        // workspace-member reacher of this native package.
        let owners =
            nearest_workspace_owners(&pkg.id, &reverse_deps, &workspace_members, &packages_by_id);
        for owner in owners {
            if entry.owning_adapter.iter().any(|a| a == owner) {
                continue;
            }
            if tooling_profile_crates.contains(owner) {
                continue;
            }
            violations.push(format!(
                "Package {} v{} is directly reached by workspace crate `{owner}`, which is not in \
                 its declared owning_adapter list {:?}",
                pkg.name, pkg.version, entry.owning_adapter
            ));
        }
    }

    Ok(violations)
}

/// Walks the resolved dependency graph backwards (who-depends-on-me) from
/// `start`, breadth-first, and returns the name(s) of the nearest
/// workspace-member crate(s) reachable on any path — i.e. the workspace
/// crate(s) that made the deliberate Cargo.toml choice pulling this native
/// package in, even when the actual `-sys`/`links` crate sits one or more
/// wrapper crates below it (e.g. hypervisor -> rusqlite -> libsqlite3-sys).
///
/// A path stops expanding once it reaches a workspace member (we only care
/// about the nearest one on that path), but the search still explores every
/// path at the same depth before stopping, so ties are all reported.
fn nearest_workspace_owners<'a>(
    start: &'a str,
    reverse_deps: &HashMap<&'a str, HashSet<&'a str>>,
    workspace_members: &HashSet<&'a str>,
    packages_by_id: &HashMap<&'a str, &'a Package>,
) -> Vec<&'a str> {
    let mut visited: HashSet<&str> = HashSet::from([start]);
    let mut queue: VecDeque<(&str, u32)> = VecDeque::from([(start, 0)]);
    let mut found = Vec::new();
    let mut found_depth: Option<u32> = None;

    while let Some((current, depth)) = queue.pop_front() {
        if found_depth.is_some_and(|fd| depth > fd) {
            break;
        }

        if workspace_members.contains(current) {
            if let Some(pkg) = packages_by_id.get(current) {
                found.push(pkg.name.as_str());
            }
            found_depth = Some(depth);
            continue;
        }

        if let Some(parents) = reverse_deps.get(current) {
            for &parent in parents {
                if visited.insert(parent) {
                    queue.push_back((parent, depth + 1));
                }
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(id: &str, name: &str) -> Package {
        Package {
            id: id.to_string(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            links: None,
            dependencies: Vec::new(),
        }
    }

    /// `validate_entries` must reject an entry missing `justification` and
    /// one missing `owning_adapter`, reporting BOTH in a single pass rather
    /// than stopping at the first failure (Required enforcement point 2).
    #[test]
    fn validate_entries_reports_all_missing_required_fields_at_once() {
        let raw = vec![
            RawAllowedPackage {
                name: "no-justification".to_string(),
                version: "1.0.0".to_string(),
                justification: None,
                owning_adapter: Some(vec!["hypervisor".to_string()]),
                target: None,
                features: None,
            },
            RawAllowedPackage {
                name: "no-owning-adapter".to_string(),
                version: "1.0.0".to_string(),
                justification: Some("fine".to_string()),
                owning_adapter: Some(vec![]),
                target: None,
                features: None,
            },
            RawAllowedPackage {
                name: "well-formed".to_string(),
                version: "1.0.0".to_string(),
                justification: Some("fine".to_string()),
                owning_adapter: Some(vec!["hypervisor".to_string()]),
                target: None,
                features: None,
            },
        ];

        match validate_entries(raw) {
            Ok(_) => panic!("missing required fields must fail the gate"),
            Err(err) => {
                let message = format!("{err:#}");
                assert!(message.contains("policy schema is incomplete"));
            }
        }
    }

    /// A fully-populated entry passes through unchanged.
    #[test]
    fn validate_entries_accepts_well_formed_entry() {
        let raw = vec![RawAllowedPackage {
            name: "ring".to_string(),
            version: "0.17.14".to_string(),
            justification: Some("ECDSA signing".to_string()),
            owning_adapter: Some(vec!["ast_auditor".to_string()]),
            target: None,
            features: None,
        }];

        let entries = validate_entries(raw).expect("well-formed entry must pass");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "ring");
        assert_eq!(entries[0].owning_adapter, vec!["ast_auditor"]);
    }

    /// Synthetic graph: `native-sys` <- `wrapper` <- `hypervisor` (workspace
    /// member), with `unrelated-tool` (external, not a workspace member) as
    /// a red herring depending directly on `native-sys` too. The nearest
    /// workspace owner must be `hypervisor`, found by walking up through the
    /// external `wrapper` crate — not `unrelated-tool`, which has no
    /// workspace-member ancestor at all.
    #[test]
    fn nearest_workspace_owners_walks_up_through_external_wrapper_crates() {
        let packages = [
            pkg("native-sys#1", "native-sys"),
            pkg("hypervisor#1", "hypervisor"),
        ];
        let packages_by_id: HashMap<&str, &Package> =
            packages.iter().map(|p| (p.id.as_str(), p)).collect();

        let workspace_members: HashSet<&str> = HashSet::from(["hypervisor#1"]);

        // reverse_deps: native-sys <- wrapper <- hypervisor
        //               native-sys <- unrelated-tool (no workspace ancestor)
        let mut reverse_deps: HashMap<&str, HashSet<&str>> = HashMap::new();
        reverse_deps.insert(
            "native-sys#1",
            HashSet::from(["wrapper#1", "unrelated-tool#1"]),
        );
        reverse_deps.insert("wrapper#1", HashSet::from(["hypervisor#1"]));

        let owners = nearest_workspace_owners(
            "native-sys#1",
            &reverse_deps,
            &workspace_members,
            &packages_by_id,
        );

        assert_eq!(owners, vec!["hypervisor"]);
    }

    /// When two workspace crates reach the native package at the same
    /// minimal depth, both must be reported (ties are not arbitrarily
    /// dropped — see e.g. `aws-lc-rs`'s multi-owner entry in
    /// native-policy.toml).
    #[test]
    fn nearest_workspace_owners_reports_ties_at_equal_depth() {
        let packages = [
            pkg("native-sys#1", "native-sys"),
            pkg("crate-a#1", "crate-a"),
            pkg("crate-b#1", "crate-b"),
        ];
        let packages_by_id: HashMap<&str, &Package> =
            packages.iter().map(|p| (p.id.as_str(), p)).collect();
        let workspace_members: HashSet<&str> = HashSet::from(["crate-a#1", "crate-b#1"]);

        let mut reverse_deps: HashMap<&str, HashSet<&str>> = HashMap::new();
        reverse_deps.insert("native-sys#1", HashSet::from(["crate-a#1", "crate-b#1"]));

        let mut owners = nearest_workspace_owners(
            "native-sys#1",
            &reverse_deps,
            &workspace_members,
            &packages_by_id,
        );
        owners.sort_unstable();

        assert_eq!(owners, vec!["crate-a", "crate-b"]);
    }

    /// A native package with no dependents at all (i.e. it IS the
    /// workspace-member crate reaching directly for its own native use, like
    /// `rfc0006_host`/`sdk` in native-policy.toml) reports itself.
    #[test]
    fn nearest_workspace_owners_self_owning_crate() {
        let packages = [pkg("sdk#1", "sdk")];
        let packages_by_id: HashMap<&str, &Package> =
            packages.iter().map(|p| (p.id.as_str(), p)).collect();
        let workspace_members: HashSet<&str> = HashSet::from(["sdk#1"]);
        let reverse_deps: HashMap<&str, HashSet<&str>> = HashMap::new();

        let owners =
            nearest_workspace_owners("sdk#1", &reverse_deps, &workspace_members, &packages_by_id);
        assert_eq!(owners, vec!["sdk"]);
    }
}
