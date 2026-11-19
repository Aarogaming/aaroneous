//! xtask/src/check_chrono.rs
//! Ratchet enforcement for leaving `chrono` (RUSTSEC-2026-0543).
//!
//! `chrono` is unmaintained with no successor, and `Utc::now()` /
//! `Local::now()` are ambient clock reads that AGENTS.md section 2 already
//! bans in library code. `docs/CHRONO_DECISION_2026-11-18.md` plans a lane to
//! leave chrono; this check keeps that lane from going backwards while it is
//! in progress.
//!
//! Two numbers are tracked per workspace package, in the same shape as
//! `check_unwraps.rs`:
//!
//! - `calls`: `Utc::now` / `Local::now` paths in production code, counted by
//!   `ast_auditor::scan_ambient_clock_hits` (test code, test files, bootstrap
//!   entrypoints and `build.rs` excluded, exactly as in `check-unwraps`).
//! - `dep`: 1 when the package's manifest declares `chrono` as a dependency
//!   (normal, dev or build), else 0.
//!
//! The gate fails when either number exceeds the package's baseline in
//! `xtask/chrono_baseline.txt`. A package missing from the baseline has an
//! implicit baseline of 0 for both, so a new crate cannot adopt chrono
//! silently. Numbers may fall freely; when one does, the check prints a hint
//! to lower the baseline but does not fail, so the baseline is lowered in the
//! same PR that removed the use. The file may only shrink or hold.
//!
//! The check is read-only and uses no clock, environment variable, or network.
//!
//! Usage: `cargo xtask check-chrono [--dump-baseline]`

use crate::check_unwraps::{collect_source_files, find_workspace_root, load_scanned_packages};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The crate name that is being left.
const CHRONO: &str = "chrono";

/// Per-package measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// `Utc::now` / `Local::now` production call sites.
    pub calls: u64,
    /// 1 if the manifest declares a `chrono` dependency, else 0.
    pub dep: u64,
}

impl Counts {
    fn is_zero(&self) -> bool {
        self.calls == 0 && self.dep == 0
    }
}

/// One metric of one package that exceeds its baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regression {
    pub package: String,
    pub metric: &'static str,
    pub current: u64,
    pub allowed: u64,
}

/// Measure every scanned workspace package.
pub fn count_current(workspace_root: &Path) -> Result<BTreeMap<String, Counts>> {
    let packages = load_scanned_packages(workspace_root)?;
    let all_roots: Vec<PathBuf> = packages.iter().map(|p| p.root_dir.clone()).collect();

    let mut counts = BTreeMap::new();
    for pkg in &packages {
        let other_roots: Vec<PathBuf> = all_roots
            .iter()
            .filter(|r| *r != &pkg.root_dir)
            .cloned()
            .collect();
        let mut calls: u64 = 0;
        for file in collect_source_files(&pkg.root_dir, &other_roots) {
            match ast_auditor::scan_ambient_clock_hits(&file) {
                Ok(hits) => calls += hits.len() as u64,
                Err(error) => bail!("failed to scan {}: {error}", file.display()),
            }
        }
        let dep = u64::from(pkg.dependencies.iter().any(|d| d == CHRONO));
        counts.insert(pkg.name.clone(), Counts { calls, dep });
    }
    Ok(counts)
}

fn baseline_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("xtask").join("chrono_baseline.txt")
}

/// Parse a baseline file: `package calls dep` per line, `#` comments and
/// blank lines ignored. A missing file is an empty baseline.
pub fn load_baseline(path: &Path) -> Result<BTreeMap<String, Counts>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse_baseline(&text).with_context(|| format!("parsing {}", path.display()))
}

fn parse_baseline(text: &str) -> Result<BTreeMap<String, Counts>> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [name, calls, dep] = parts.as_slice() else {
            bail!("malformed baseline line '{line}' (expected `package calls dep`)");
        };
        let calls: u64 = calls
            .parse()
            .with_context(|| format!("non-numeric calls in line '{line}'"))?;
        let dep: u64 = dep
            .parse()
            .with_context(|| format!("non-numeric dep in line '{line}'"))?;
        if dep > 1 {
            bail!("dep must be 0 or 1 in line '{line}'");
        }
        map.insert((*name).to_string(), Counts { calls, dep });
    }
    Ok(map)
}

/// Metrics that grew past the baseline. These are the only failures.
pub fn find_regressions(
    current: &BTreeMap<String, Counts>,
    baseline: &BTreeMap<String, Counts>,
) -> Vec<Regression> {
    let mut regressions = Vec::new();
    for (name, now) in current {
        let allowed = baseline.get(name).copied().unwrap_or_default();
        if now.calls > allowed.calls {
            regressions.push(Regression {
                package: name.clone(),
                metric: "Utc::now/Local::now call sites",
                current: now.calls,
                allowed: allowed.calls,
            });
        }
        if now.dep > allowed.dep {
            regressions.push(Regression {
                package: name.clone(),
                metric: "direct chrono dependency",
                current: now.dep,
                allowed: allowed.dep,
            });
        }
    }
    regressions
}

/// Packages whose baseline is now higher than reality, so the baseline can
/// be lowered. Packages absent from the workspace are included (the package
/// was removed or renamed). Returned as `(package, baseline, current)`.
pub fn find_stale(
    current: &BTreeMap<String, Counts>,
    baseline: &BTreeMap<String, Counts>,
) -> Vec<(String, Counts, Counts)> {
    let mut stale = Vec::new();
    for (name, allowed) in baseline {
        let now = current.get(name).copied().unwrap_or_default();
        if now.calls < allowed.calls || now.dep < allowed.dep {
            stale.push((name.clone(), *allowed, now));
        }
    }
    stale
}

/// Baseline-file text for `current`: only packages with something to
/// ratchet are listed, since an absent package already means zero.
pub fn format_baseline(current: &BTreeMap<String, Counts>) -> String {
    let mut out = String::new();
    out.push_str(
        "# Ratchet baseline for `cargo xtask check-chrono` (AGENTS.md section 2, \"No\n\
         # Ambient Clock\"; docs/CHRONO_DECISION_2026-11-18.md; RUSTSEC-2026-0543).\n\
         # Each line is `package calls dep`:\n\
         #   calls = `Utc::now` / `Local::now` paths in the package's production code\n\
         #           (tests/, examples/, benches/, build.rs, bootstrap entrypoints,\n\
         #           #[cfg(test)] and #[test] items are excluded; macro bodies that are\n\
         #           not comma-separated expressions are not seen).\n\
         #   dep   = 1 if the package's manifest declares `chrono`, else 0.\n\
         # A package that is not listed has calls = 0 and dep = 0, so a new crate\n\
         # cannot adopt chrono without this file growing, which review will see.\n\
         #\n\
         # This list may only shrink or stay flat: the gate fails if a package's\n\
         # current number exceeds the one recorded here. When you remove a use,\n\
         # lower the number (delete the line at 0 0). Do not leave it stale.\n\
         # Exit: when `chrono` is gone from Cargo.lock this file is empty and the\n\
         # check can be deleted together with the deny.toml/audit.toml entries.\n",
    );
    for (name, counts) in current {
        if counts.is_zero() {
            continue;
        }
        out.push_str(&format!("{name} {} {}\n", counts.calls, counts.dep));
    }
    out
}

/// `cargo xtask check-chrono [--dump-baseline]`. `--dump-baseline` prints the
/// current numbers in baseline-file format instead of checking.
pub fn run(args: &[String]) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let workspace_root = find_workspace_root(&current_dir)?;
    run_with_root(&workspace_root, args)
}

/// Run the chrono ratchet against an explicit workspace root.
pub fn run_with_root(workspace_root: &Path, args: &[String]) -> Result<()> {
    let current = count_current(workspace_root)?;

    if args.iter().any(|a| a == "--dump-baseline") {
        print!("{}", format_baseline(&current));
        return Ok(());
    }

    let baseline = load_baseline(&baseline_path(workspace_root))?;
    let regressions = find_regressions(&current, &baseline);

    let total_calls: u64 = current.values().map(|c| c.calls).sum();
    let total_deps: u64 = current.values().map(|c| c.dep).sum();
    eprintln!(
        "check-chrono: {} package(s) scanned; {total_deps} with a direct chrono dependency, \
         {total_calls} Utc::now/Local::now call site(s).",
        current.len()
    );

    for (name, allowed, now) in find_stale(&current, &baseline) {
        eprintln!(
            "check-chrono: hint: {name} is below its baseline (calls {} -> {}, dep {} -> {}); \
             lower xtask/chrono_baseline.txt.",
            allowed.calls, now.calls, allowed.dep, now.dep
        );
    }

    if regressions.is_empty() {
        eprintln!("check-chrono: OK - no package exceeds its baseline.");
        return Ok(());
    }

    eprintln!("check-chrono: package(s) exceeding their ratchet baseline:");
    for r in &regressions {
        eprintln!(
            "  {}: {} {} > baseline {}",
            r.package, r.metric, r.current, r.allowed
        );
    }
    bail!(
        "check-chrono: {} metric(s) grew past the chrono baseline. chrono is unmaintained \
         (RUSTSEC-2026-0543) and the workspace is leaving it: take the time from an injected \
         clock instead of Utc::now()/Local::now(), and do not add a chrono dependency. See \
         docs/CHRONO_DECISION_2026-11-18.md.",
        regressions.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(calls: u64, dep: u64) -> Counts {
        Counts { calls, dep }
    }

    fn map(pairs: &[(&str, Counts)]) -> BTreeMap<String, Counts> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn regression_when_calls_exceed_baseline() {
        let current = map(&[("a", c(5, 1))]);
        let baseline = map(&[("a", c(3, 1))]);
        let regressions = find_regressions(&current, &baseline);
        assert_eq!(regressions.len(), 1);
        assert_eq!(regressions[0].package, "a");
        assert_eq!((regressions[0].current, regressions[0].allowed), (5, 3));
    }

    #[test]
    fn regression_when_dependency_is_added() {
        let current = map(&[("a", c(0, 1))]);
        let baseline = map(&[("a", c(0, 0))]);
        let regressions = find_regressions(&current, &baseline);
        assert_eq!(regressions.len(), 1);
        assert_eq!(regressions[0].metric, "direct chrono dependency");
    }

    #[test]
    fn new_package_is_held_to_zero() {
        let current = map(&[("new_pkg", c(1, 1))]);
        let regressions = find_regressions(&current, &BTreeMap::new());
        assert_eq!(regressions.len(), 2);
    }

    #[test]
    fn no_regression_when_equal_or_lower() {
        let baseline = map(&[("a", c(3, 1))]);
        assert!(find_regressions(&map(&[("a", c(3, 1))]), &baseline).is_empty());
        assert!(find_regressions(&map(&[("a", c(1, 0))]), &baseline).is_empty());
        assert!(find_regressions(&map(&[("a", c(0, 0))]), &baseline).is_empty());
    }

    #[test]
    fn stale_baseline_is_reported_but_is_not_a_regression() {
        let current = map(&[("a", c(1, 1)), ("b", c(0, 0))]);
        let baseline = map(&[("a", c(3, 1)), ("b", c(2, 1)), ("gone", c(4, 1))]);
        assert!(find_regressions(&current, &baseline).is_empty());
        let stale: Vec<String> = find_stale(&current, &baseline)
            .into_iter()
            .map(|(name, _, _)| name)
            .collect();
        assert_eq!(stale, vec!["a", "b", "gone"]);
    }

    #[test]
    fn baseline_round_trips_and_omits_zero_packages() {
        let current = map(&[("a", c(3, 1)), ("b", c(0, 0)), ("d", c(0, 1))]);
        let text = format_baseline(&current);
        assert!(!text.lines().any(|l| l.starts_with("b ")));
        let parsed = parse_baseline(&text).expect("formatted baseline must parse");
        assert_eq!(parsed, map(&[("a", c(3, 1)), ("d", c(0, 1))]));
    }

    #[test]
    fn baseline_rejects_malformed_lines() {
        assert!(parse_baseline("a 1\n").is_err());
        assert!(parse_baseline("a x 1\n").is_err());
        assert!(parse_baseline("a 1 2\n").is_err());
        assert!(parse_baseline("a 1 1 extra\n").is_err());
    }

    #[test]
    fn baseline_ignores_comments_and_blank_lines() {
        let parsed = parse_baseline("# note\n\na 2 1\n\n# more\nb 5 0\n").expect("valid baseline");
        assert_eq!(parsed, map(&[("a", c(2, 1)), ("b", c(5, 0))]));
    }

    #[test]
    fn missing_baseline_file_is_empty_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("absent.txt");
        assert!(load_baseline(&path).expect("missing is ok").is_empty());
    }
}
