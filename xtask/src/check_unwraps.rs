//! xtask/src/check_unwraps.rs
//! Ratchet enforcement for AGENTS.md section 5's ban on `.unwrap()`,
//! `.expect(...)`, `panic!(...)`, and `assert!(...)` on values derived from
//! I/O, config, or model output.
//!
//! Mirrors `check_deps.rs`'s pattern (see `xtask/dep_direction_baseline.txt`
//! on `chore/dependency-hygiene`): walk the workspace, count current
//! occurrences per package, and compare against a checked-in baseline file
//! (`xtask/unwrap_baseline.txt`). The gate fails only when a package's
//! current count exceeds its baseline - the count is free to go down (fixed
//! occurrences) or stay flat, but a new occurrence with no exemption fails
//! the gate immediately rather than silently inflating the baseline.
//!
//! The actual scan (parsing, `#[cfg(test)]`/`#[test]` exclusion, the
//! `// INFALLIBLE:` escape hatch) lives in
//! `ast_auditor::scan_unwrap_panic_hits` / `ast_auditor::UnwrapPanicVisitor`,
//! reused here rather than re-implemented; this module is only responsible
//! for workspace/package discovery, file-shape exclusions (`tests/`,
//! `examples/`, `benches/`, `build.rs`, bootstrap entrypoints), and the
//! baseline diff.
//!
//! Usage: `cargo xtask check-unwraps`

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Top-level workspace directories this check scans, matching the scope the
/// task and AGENTS.md section 6 gate 6 both use (`core/`, `crates/`,
/// `dev/`).
const SCAN_ROOTS: &[&str] = &["core", "crates", "dev"];

struct WorkspacePackage {
    name: String,
    root_dir: PathBuf,
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

/// Load every workspace member package whose manifest lives under one of
/// `SCAN_ROOTS`, via `cargo metadata --no-deps` (authoritative package
/// names and manifest paths, robust to renames).
fn load_scanned_packages(workspace_root: &Path) -> Result<Vec<WorkspacePackage>> {
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
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("failed to parse `cargo metadata` JSON")?;
    let packages = metadata
        .get("packages")
        .and_then(|p| p.as_array())
        .context("`cargo metadata` output has no 'packages' array")?;

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
            .context("package with no manifest_path")?;
        let manifest_path = PathBuf::from(manifest_path);
        let Ok(rel) = manifest_path.strip_prefix(workspace_root) else {
            continue;
        };
        let Some(first_component) = rel.components().next() else {
            continue;
        };
        let first = first_component.as_os_str().to_string_lossy();
        if !SCAN_ROOTS.contains(&first.as_ref()) {
            continue;
        }
        let root_dir = manifest_path
            .parent()
            .context("manifest_path has no parent directory")?
            .to_path_buf();
        result.push(WorkspacePackage { name, root_dir });
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// True when `path` (which must be under `pkg_root`) should be excluded
/// from the scan, per AGENTS.md section 5's own exemption list (bootstrap
/// entrypoints, tests, `build.rs`) plus the task's file-shape exclusions
/// (`tests/`, `examples/`, `benches/`, `*_test.rs`, `*_tests.rs`).
fn is_excluded(path: &Path, pkg_root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(pkg_root) else {
        return false;
    };
    let components: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components
        .iter()
        .any(|c| matches!(c.as_str(), "tests" | "examples" | "benches" | "target"))
    {
        return true;
    }

    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };

    if file_name == "build.rs" {
        return true;
    }
    if file_name.ends_with("_test.rs") || file_name.ends_with("_tests.rs") {
        return true;
    }
    // Bootstrap entrypoints per AGENTS.md section 1's own floor exemption
    // (`src/main.rs`, `src/bin/*`).
    if file_name == "main.rs" && components.len() >= 2 && components[components.len() - 2] == "src"
    {
        return true;
    }
    if components
        .windows(2)
        .any(|w| w[0] == "src" && w[1] == "bin")
    {
        return true;
    }

    false
}

/// Collect every `.rs` file under `pkg_root` that belongs to `pkg_root`'s
/// own package - i.e. stop descending into a nested directory that is
/// itself a separate workspace member's manifest root (e.g.
/// `crates/runtime_monitor/runtime_monitor_bench` nested under
/// `crates/runtime_monitor`), so a file is never attributed to two
/// packages at once.
fn collect_source_files(pkg_root: &Path, other_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(pkg_root).into_iter().filter_entry(|entry| {
        if entry.depth() == 0 {
            return true;
        }
        if entry.file_type().is_dir() {
            if matches!(
                entry.file_name().to_str(),
                Some("target" | ".git" | "node_modules")
            ) {
                return false;
            }
            if other_roots.iter().any(|r| r == entry.path()) {
                return false;
            }
        }
        true
    }) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if is_excluded(path, pkg_root) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files
}

/// Count current unwrap/expect/panic/assert occurrences per package.
pub fn count_current(workspace_root: &Path) -> Result<BTreeMap<String, u64>> {
    let packages = load_scanned_packages(workspace_root)?;
    let all_roots: Vec<PathBuf> = packages.iter().map(|p| p.root_dir.clone()).collect();

    let mut counts = BTreeMap::new();
    for pkg in &packages {
        let other_roots: Vec<PathBuf> = all_roots
            .iter()
            .filter(|r| *r != &pkg.root_dir)
            .cloned()
            .collect();
        let files = collect_source_files(&pkg.root_dir, &other_roots);
        let mut total: u64 = 0;
        for file in files {
            match ast_auditor::scan_unwrap_panic_hits(&file) {
                Ok(hits) => total += hits.len() as u64,
                Err(error) => bail!("failed to scan {}: {error}", file.display()),
            }
        }
        counts.insert(pkg.name.clone(), total);
    }
    Ok(counts)
}

fn baseline_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("xtask").join("unwrap_baseline.txt")
}

pub fn load_baseline(path: &Path) -> Result<BTreeMap<String, u64>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .with_context(|| format!("{}: malformed baseline line '{line}'", path.display()))?;
        let count: u64 = parts
            .next()
            .with_context(|| format!("{}: malformed baseline line '{line}'", path.display()))?
            .parse()
            .with_context(|| format!("{}: non-numeric count in line '{line}'", path.display()))?;
        map.insert(name.to_string(), count);
    }
    Ok(map)
}

/// Compare current per-package counts against the baseline. Returns the
/// packages whose current count exceeds their baseline - the only
/// failures. A package whose count dropped below baseline, or one no
/// longer present, is not a failure: unlike `check_deps.rs`'s exact-set
/// edge diff, this ratchet only needs to shrink or hold, not track
/// perfectly (the baseline is expected to be hand-edited down over time as
/// unwraps are actually fixed).
pub fn find_regressions<'a>(
    current: &'a BTreeMap<String, u64>,
    baseline: &BTreeMap<String, u64>,
) -> Vec<(&'a str, u64, u64)> {
    let mut regressions = Vec::new();
    for (name, &count) in current {
        let allowed = baseline.get(name).copied().unwrap_or(0);
        if count > allowed {
            regressions.push((name.as_str(), count, allowed));
        }
    }
    regressions
}

pub fn format_baseline(counts: &BTreeMap<String, u64>) -> String {
    let mut out = String::new();
    out.push_str(
        "# Ratchet baseline for `cargo xtask check-unwraps` (AGENTS.md section 5;\n\
         # CRATIFY_SPEC.md's Ratchet pattern). Each line is `package_name count`: the\n\
         # number of `.unwrap()`, `.expect(...)`, `panic!(...)`, and `assert!(...)`\n\
         # occurrences found in that package's production code. Excluded: tests/,\n\
         # examples/, benches/, build.rs, bootstrap entrypoints (src/main.rs,\n\
         # src/bin/*), *_test.rs, *_tests.rs, #[cfg(test)]/#[test] items, debug_assert!,\n\
         # and any call whose preceding line is `// INFALLIBLE: <reason>` - see\n\
         # crates/ast_auditor/src/rules/unwrap_panic.rs.\n\
         #\n\
         # This list may only shrink or stay flat: the gate fails if a package's\n\
         # current count exceeds the number recorded here. When you fix unwraps,\n\
         # lower the number - do not leave it stale.\n",
    );
    for (name, count) in counts {
        out.push_str(&format!("{name} {count}\n"));
    }
    out
}

/// `cargo xtask check-unwraps [--dump-baseline]`. `--dump-baseline` prints
/// the current per-package counts in baseline-file format (to regenerate
/// `xtask/unwrap_baseline.txt` by hand after intentionally raising or
/// lowering it) instead of running the ratchet check.
pub fn run(args: &[String]) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let workspace_root = find_workspace_root(&current_dir)?;

    let current = count_current(&workspace_root)?;

    if args.iter().any(|a| a == "--dump-baseline") {
        print!("{}", format_baseline(&current));
        return Ok(());
    }

    let baseline = load_baseline(&baseline_path(&workspace_root))?;
    let regressions = find_regressions(&current, &baseline);

    let total: u64 = current.values().sum();
    println!(
        "check-unwraps: {} package(s) scanned, {} total unwrap/expect/panic/assert occurrence(s).",
        current.len(),
        total
    );

    if regressions.is_empty() {
        println!("check-unwraps: OK - no package exceeds its baseline.");
        return Ok(());
    }

    eprintln!("check-unwraps: package(s) exceeding their ratchet baseline:");
    for (name, count, allowed) in &regressions {
        eprintln!("  {name}: {count} > baseline {allowed}");
    }
    bail!(
        "check-unwraps: {} package(s) regressed past their unwrap/expect/panic/assert baseline. \
         Fix the new occurrence(s), or if genuinely infallible, mark the line above the call \
         `// INFALLIBLE: <reason>` per AGENTS.md section 5.",
        regressions.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn regression_when_current_exceeds_baseline() {
        let current = map(&[("a", 5), ("b", 2)]);
        let baseline = map(&[("a", 3), ("b", 2)]);
        let regressions = find_regressions(&current, &baseline);
        assert_eq!(regressions, vec![("a", 5, 3)]);
    }

    #[test]
    fn no_regression_when_current_drops_below_baseline() {
        let current = map(&[("a", 1)]);
        let baseline = map(&[("a", 3)]);
        assert!(find_regressions(&current, &baseline).is_empty());
    }

    #[test]
    fn no_regression_when_current_equals_baseline() {
        let current = map(&[("a", 3)]);
        let baseline = map(&[("a", 3)]);
        assert!(find_regressions(&current, &baseline).is_empty());
    }

    #[test]
    fn new_package_with_zero_baseline_regresses_on_any_count() {
        let current = map(&[("new_pkg", 1)]);
        let baseline = BTreeMap::new();
        assert_eq!(
            find_regressions(&current, &baseline),
            vec![("new_pkg", 1, 0)]
        );
    }

    #[test]
    fn baseline_round_trips_through_format_and_parse() {
        let counts = map(&[("a", 3), ("b", 0)]);
        let formatted = format_baseline(&counts);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.txt");
        fs::write(&path, &formatted).unwrap();
        let parsed = load_baseline(&path).unwrap();
        assert_eq!(parsed, counts);
    }

    #[test]
    fn baseline_ignores_comments_and_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("baseline.txt");
        fs::write(&path, "# comment\n\na 2\n\n# another\nb 5\n").unwrap();
        let parsed = load_baseline(&path).unwrap();
        assert_eq!(parsed, map(&[("a", 2), ("b", 5)]));
    }

    #[test]
    fn missing_baseline_file_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.txt");
        assert!(load_baseline(&path).unwrap().is_empty());
    }

    #[test]
    fn excludes_test_dirs_examples_benches_and_bootstrap_entrypoints() {
        let root = Path::new("crates/foo");
        assert!(is_excluded(Path::new("crates/foo/tests/it.rs"), root));
        assert!(is_excluded(Path::new("crates/foo/examples/demo.rs"), root));
        assert!(is_excluded(Path::new("crates/foo/benches/b.rs"), root));
        assert!(is_excluded(Path::new("crates/foo/build.rs"), root));
        assert!(is_excluded(Path::new("crates/foo/src/main.rs"), root));
        assert!(is_excluded(Path::new("crates/foo/src/bin/tool.rs"), root));
        assert!(is_excluded(
            Path::new("crates/foo/src/widget_test.rs"),
            root
        ));
        assert!(is_excluded(
            Path::new("crates/foo/src/widget_tests.rs"),
            root
        ));
        assert!(!is_excluded(Path::new("crates/foo/src/lib.rs"), root));
        assert!(!is_excluded(Path::new("crates/foo/src/widget.rs"), root));
    }
}
