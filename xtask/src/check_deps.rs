//! xtask/src/check_deps.rs
//! Implements CRATIFY_SPEC.md section 2.3 / 7.1 item 9: the profile
//! dependency-direction check.
//!
//! A crate may depend (normal or build dependency, not dev) only on crates
//! whose Cratify profile is the same or stricter, in the order
//! `kernel` > `control` > `presentation` == `tooling`. This module:
//!
//! 1. Determines every workspace crate's profile, preferring
//!    `[package.metadata.cratify] profile = "..."` in its own `Cargo.toml`
//!    and falling back to the `### 2.2 Profile Assignment` table in
//!    `docs/CRATIFY_SPEC.md`.
//! 2. Builds the workspace-internal dependency edge set from
//!    `cargo metadata --no-deps` (normal + build dependencies only).
//! 3. Flags any edge that crosses from a stricter profile to a looser one.
//! 4. Compares the violation set against a checked-in ratchet baseline
//!    (`xtask/dep_direction_baseline.txt`): a new edge fails the gate, and a
//!    baseline edge that no longer exists also fails the gate (the baseline
//!    must shrink, not just drift).
//!
//! Usage: `cargo xtask check-deps`

use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The four Cratify compliance profiles, ordered from strictest (0) to
/// loosest. `presentation` and `tooling` are equally strict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Profile {
    Kernel,
    Control,
    PresentationOrTooling,
}

impl Profile {
    fn parse(s: &str) -> Option<Profile> {
        match s {
            "kernel" => Some(Profile::Kernel),
            "control" => Some(Profile::Control),
            "presentation" | "tooling" => Some(Profile::PresentationOrTooling),
            _ => None,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Profile::Kernel => 0,
            Profile::Control => 1,
            Profile::PresentationOrTooling => 2,
        }
    }
}

/// A directed workspace-internal dependency edge, `from -> to`, using crate
/// (package) names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

impl Edge {
    fn format(&self) -> String {
        format!("{} -> {}", self.from, self.to)
    }

    fn parse(line: &str) -> Option<Edge> {
        let (from, to) = line.split_once("->")?;
        Some(Edge {
            from: from.trim().to_string(),
            to: to.trim().to_string(),
        })
    }
}

/// Parse the `### 2.2 Profile Assignment` table out of CRATIFY_SPEC.md's
/// text, returning a map of raw table token (crate name, or a workspace
/// path like `core/hypervisor`) to its declared profile.
///
/// The table looks like:
/// ```text
/// ### 2.2 Profile Assignment
///
/// | Profile | Crates |
/// |---|---|
/// | `kernel` | `core/hypervisor`, `ipc_bus`, ... |
/// | `control` | `orchestrator`, ... |
/// ```
pub fn parse_profile_table(spec_text: &str) -> Result<BTreeMap<String, Profile>> {
    let heading_pos = spec_text
        .find("### 2.2 Profile Assignment")
        .context("docs/CRATIFY_SPEC.md has no '### 2.2 Profile Assignment' heading")?;
    let after_heading = &spec_text[heading_pos..];

    let mut map = BTreeMap::new();
    let mut rows_seen = 0usize;
    for line in after_heading.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            // Stop once we've seen at least one data row and hit a non-table line.
            if rows_seen > 0 {
                break;
            }
            continue;
        }
        // Header separator row, e.g. "|---|---|".
        if trimmed.chars().all(|c| matches!(c, '|' | '-' | ' ' | ':')) {
            continue;
        }
        let cells: Vec<&str> = trimmed.trim_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let profile_cell = cells[0].trim();
        if !profile_cell.starts_with('`') {
            // The header row ("| Profile | Crates |") — skip.
            continue;
        }
        let profile_name = profile_cell.trim_matches('`');
        let Some(profile) = Profile::parse(profile_name) else {
            continue;
        };
        rows_seen += 1;

        let crates_cell = cells[1];
        for token in crates_cell.split(',') {
            let name = token.trim().trim_matches('`').trim();
            if name.is_empty() {
                continue;
            }
            map.insert(name.to_string(), profile);
        }
    }

    if map.is_empty() {
        bail!("Parsed zero rows from the 2.2 Profile Assignment table in docs/CRATIFY_SPEC.md");
    }
    Ok(map)
}

/// Resolve a workspace root Cargo.toml-relative path token (as it appears in
/// the profile table, e.g. `core/hypervisor` or `sdk/rust`) to the actual
/// crate (package) name declared in that directory's `Cargo.toml`. A token
/// with no `/` is already a crate name and is returned unchanged.
fn resolve_table_token_to_crate_name(workspace_root: &Path, token: &str) -> Result<String> {
    if !token.contains('/') {
        return Ok(token.to_string());
    }
    let manifest = workspace_root.join(token).join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .with_context(|| format!("reading {} for profile-table entry '{token}'", manifest.display()))?;
    let parsed: toml::Value = toml::from_str(&text)?;
    let name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .with_context(|| format!("{} has no [package].name", manifest.display()))?;
    Ok(name.to_string())
}

/// Build the crate-name -> profile map, resolving path-shaped table tokens
/// (e.g. `sdk/rust` -> package name `sdk`) against the real crate manifests.
pub fn build_profile_map_from_spec(
    workspace_root: &Path,
    spec_text: &str,
) -> Result<BTreeMap<String, Profile>> {
    let raw = parse_profile_table(spec_text)?;
    let mut resolved = BTreeMap::new();
    for (token, profile) in raw {
        let crate_name = resolve_table_token_to_crate_name(workspace_root, &token)?;
        resolved.insert(crate_name, profile);
    }
    Ok(resolved)
}

/// Read `[package.metadata.cratify] profile = "..."` directly from a
/// package's Cargo.toml, if present. Returns `None` (not an error) when the
/// key is absent, so callers can fall back to the spec table.
fn read_declared_profile(manifest_path: &str) -> Result<Option<Profile>> {
    let text = fs::read_to_string(manifest_path)
        .with_context(|| format!("reading {manifest_path}"))?;
    let parsed: toml::Value = toml::from_str(&text)?;
    let Some(name) = parsed
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("cratify"))
        .and_then(|c| c.get("profile"))
        .and_then(|p| p.as_str())
    else {
        return Ok(None);
    };
    match Profile::parse(name) {
        Some(p) => Ok(Some(p)),
        None => bail!("{manifest_path}: unknown cratify profile '{name}'"),
    }
}

/// One workspace member, as reported by `cargo metadata --no-deps`.
struct WorkspacePackage {
    name: String,
    manifest_path: String,
    /// Names of workspace-internal normal/build dependencies (dev
    /// dependencies excluded).
    deps: Vec<String>,
}

/// Run `cargo metadata --no-deps --format-version=1` and extract the
/// workspace members and their workspace-internal normal/build
/// dependencies.
fn load_workspace_packages(workspace_root: &Path) -> Result<Vec<WorkspacePackage>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(workspace_root)
        .output()
        .context("failed to spawn `cargo metadata`")?;
    if !output.status.success() {
        bail!(
            "`cargo metadata --no-deps` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse `cargo metadata` JSON")?;
    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .context("`cargo metadata` output has no 'packages' array")?;

    let workspace_names: BTreeSet<String> = packages
        .iter()
        .filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    let mut result = Vec::new();
    for pkg in packages {
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .context("package with no name")?
            .to_string();
        let manifest_path = pkg
            .get("manifest_path")
            .and_then(|n| n.as_str())
            .context("package with no manifest_path")?
            .to_string();
        let mut deps = Vec::new();
        if let Some(dep_list) = pkg.get("dependencies").and_then(|d| d.as_array()) {
            for dep in dep_list {
                let kind = dep.get("kind").and_then(|k| k.as_str());
                // `kind` is `null` (missing) for normal deps, "build" for
                // build deps, "dev" for dev deps. We want normal + build.
                if kind == Some("dev") {
                    continue;
                }
                let Some(dep_name) = dep.get("name").and_then(|n| n.as_str()) else {
                    continue;
                };
                // `cargo metadata`'s dependency `name` is always the real
                // package name, even when the manifest renames it (e.g.
                // `specialists = { package = "capabilities", ... }`), so
                // aliasing never hides an edge here.
                if workspace_names.contains(dep_name) && dep_name != name {
                    deps.push(dep_name.to_string());
                }
            }
        }
        deps.sort();
        deps.dedup();
        result.push(WorkspacePackage {
            name,
            manifest_path,
            deps,
        });
    }
    Ok(result)
}

/// Compare the currently-computed violation set against the checked-in
/// baseline. Returns `(new_edges, resolved_edges)`; the gate should fail if
/// either is non-empty.
pub fn diff_against_baseline(
    current: &BTreeSet<Edge>,
    baseline: &BTreeSet<Edge>,
) -> (Vec<Edge>, Vec<Edge>) {
    let new_edges: Vec<Edge> = current.difference(baseline).cloned().collect();
    let resolved_edges: Vec<Edge> = baseline.difference(current).cloned().collect();
    (new_edges, resolved_edges)
}

fn load_baseline(path: &Path) -> Result<BTreeSet<Edge>> {
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let edge = Edge::parse(line)
            .with_context(|| format!("{}: malformed edge line '{line}'", path.display()))?;
        set.insert(edge);
    }
    Ok(set)
}

fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let toml_path = current.join("Cargo.toml");
        if toml_path.exists() {
            let content = fs::read_to_string(&toml_path)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        if !current.pop() {
            bail!("Could not find workspace root with [workspace] section");
        }
    }
}

/// Entry point for `cargo xtask check-deps`.
pub fn run() -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let workspace_root = find_workspace_root(&current_dir)?;

    let spec_path = workspace_root.join("docs").join("CRATIFY_SPEC.md");
    let spec_text = fs::read_to_string(&spec_path)
        .with_context(|| format!("reading {}", spec_path.display()))?;
    let spec_profiles = build_profile_map_from_spec(&workspace_root, &spec_text)?;

    let packages = load_workspace_packages(&workspace_root)?;

    // Resolve each workspace package's profile: declared metadata wins,
    // else the spec table, else it's unclassified (a hard failure).
    let mut profiles: BTreeMap<String, Profile> = BTreeMap::new();
    let mut unclassified = Vec::new();
    for pkg in &packages {
        let declared = read_declared_profile(&pkg.manifest_path)?;
        let profile = declared.or_else(|| spec_profiles.get(&pkg.name).copied());
        match profile {
            Some(p) => {
                profiles.insert(pkg.name.clone(), p);
            }
            None => unclassified.push(pkg.name.clone()),
        }
    }

    if !unclassified.is_empty() {
        bail!(
            "check-deps: {} workspace crate(s) have no Cratify profile (neither \
             [package.metadata.cratify].profile nor a row in docs/CRATIFY_SPEC.md's \
             2.2 table): {}",
            unclassified.len(),
            unclassified.join(", ")
        );
    }

    let mut current_violations: BTreeSet<Edge> = BTreeSet::new();
    for pkg in &packages {
        let from_profile = profiles[&pkg.name];
        for dep_name in &pkg.deps {
            let to_profile = profiles[dep_name];
            if to_profile.rank() > from_profile.rank() {
                current_violations.insert(Edge {
                    from: pkg.name.clone(),
                    to: dep_name.clone(),
                });
            }
        }
    }

    let baseline_path = workspace_root.join("xtask").join("dep_direction_baseline.txt");
    let baseline = load_baseline(&baseline_path)?;

    let (new_edges, resolved_edges) = diff_against_baseline(&current_violations, &baseline);

    if new_edges.is_empty() && resolved_edges.is_empty() {
        println!(
            "check-deps: OK — {} known profile-direction violation(s), unchanged from baseline.",
            current_violations.len()
        );
        return Ok(());
    }

    if !new_edges.is_empty() {
        eprintln!("check-deps: new profile-direction violation(s) not in the baseline:");
        for e in &new_edges {
            eprintln!("  + {}", e.format());
        }
    }
    if !resolved_edges.is_empty() {
        eprintln!(
            "check-deps: baseline edge(s) no longer present in the dependency graph — the \
             ratchet only shrinks, so remove these from {}:",
            baseline_path.display()
        );
        for e in &resolved_edges {
            eprintln!("  - {}", e.format());
        }
    }
    bail!(
        "check-deps: profile dependency-direction check failed ({} new, {} to remove from baseline)",
        new_edges.len(),
        resolved_edges.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SPEC: &str = r#"
# Doc

### 2.1 Profile Rules

| Rule | `kernel` | `control` |
|---|---|---|
| Something | yes | yes |

### 2.2 Profile Assignment

| Profile | Crates |
|---|---|
| `kernel` | `core/hypervisor`, `ipc_bus`, `wire` |
| `control` | `orchestrator`, `governance`, `sdk/rust` |
| `presentation` | `api`, `studio_hud` |
| `tooling` | `ast_auditor`, `xtask` |

`core/hypervisor` has two roles.

### 2.3 Profile Dependency Direction

Text that must not be parsed as table rows.
"#;

    #[test]
    fn parses_profile_table_rows_and_stops_at_next_section() {
        let map = parse_profile_table(SAMPLE_SPEC).unwrap();
        assert_eq!(map.get("core/hypervisor"), Some(&Profile::Kernel));
        assert_eq!(map.get("ipc_bus"), Some(&Profile::Kernel));
        assert_eq!(map.get("wire"), Some(&Profile::Kernel));
        assert_eq!(map.get("orchestrator"), Some(&Profile::Control));
        assert_eq!(map.get("sdk/rust"), Some(&Profile::Control));
        assert_eq!(map.get("api"), Some(&Profile::PresentationOrTooling));
        assert_eq!(map.get("ast_auditor"), Some(&Profile::PresentationOrTooling));
        // The 2.1 table's "Rule"/"kernel"/"control" header cells must not
        // leak in as bogus crate names.
        assert!(!map.contains_key("Rule"));
        assert!(!map.contains_key("Something"));
    }

    #[test]
    fn missing_section_is_an_error() {
        let err = parse_profile_table("# no such section here").unwrap_err();
        assert!(err.to_string().contains("2.2 Profile Assignment"));
    }

    #[test]
    fn profile_rank_orders_kernel_strictest() {
        assert!(Profile::Kernel.rank() < Profile::Control.rank());
        assert!(Profile::Control.rank() < Profile::PresentationOrTooling.rank());
    }

    fn edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn diff_reports_new_and_resolved_edges() {
        let current: BTreeSet<Edge> = [edge("hypervisor", "paths"), edge("hypervisor", "omni")]
            .into_iter()
            .collect();
        let baseline: BTreeSet<Edge> = [edge("hypervisor", "paths"), edge("ipc_bus", "paths")]
            .into_iter()
            .collect();
        let (new_edges, resolved_edges) = diff_against_baseline(&current, &baseline);
        assert_eq!(new_edges, vec![edge("hypervisor", "omni")]);
        assert_eq!(resolved_edges, vec![edge("ipc_bus", "paths")]);
    }

    #[test]
    fn diff_is_empty_when_sets_match() {
        let set: BTreeSet<Edge> = [edge("a", "b")].into_iter().collect();
        let (new_edges, resolved_edges) = diff_against_baseline(&set, &set);
        assert!(new_edges.is_empty());
        assert!(resolved_edges.is_empty());
    }

    #[test]
    fn edge_parse_and_format_round_trip() {
        let e = Edge::parse("hypervisor -> paths").unwrap();
        assert_eq!(e.from, "hypervisor");
        assert_eq!(e.to, "paths");
        assert_eq!(e.format(), "hypervisor -> paths");
    }

    #[test]
    fn edge_parse_rejects_malformed_line() {
        assert!(Edge::parse("not an edge").is_none());
    }
}
