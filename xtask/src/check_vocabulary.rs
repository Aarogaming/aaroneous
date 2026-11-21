//! xtask/src/check_vocabulary.rs
//! Ratchet enforcement for the vocabulary rule: use the standardized scientific
//! or engineering term where one exists, and keep an analogical name only where
//! none does (owner rule 2026-11-19; term list approved 2026-11-21).
//!
//! Mirrors `check_unwraps.rs`: walk the workspace, count flagged identifier
//! occurrences per package, and compare against a checked-in baseline
//! (`xtask/vocabulary_baseline.txt`). The failure is a count that rises above its
//! baseline; the count may fall or hold. Package discovery, file-shape exclusions
//! and baseline parsing/diffing are shared with `check_unwraps`; the scan itself
//! is `ast_auditor::scan_vocabulary_hits`, and the reviewed term list lives in
//! `xtask/vocabulary_terms.txt` (every entry has a recorded reason, allowed terms
//! included).
//!
//! This command renames nothing. It is not yet part of `cargo xtask gate`;
//! wiring it in changes the mandatory verification protocol and is a separate,
//! owner-approved step.
//!
//! Usage: `cargo xtask check-vocabulary [--dump-baseline]`

use anyhow::{Context, Result, bail};
use ast_auditor::TermList;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::check_unwraps::{
    collect_source_files, find_regressions, find_workspace_root, load_baseline,
    load_scanned_packages,
};

fn terms_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("xtask").join("vocabulary_terms.txt")
}

fn baseline_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("xtask").join("vocabulary_baseline.txt")
}

/// Load and parse the reviewed term list. A missing or malformed list is an
/// error, never an empty list (an empty list would silently pass everything).
pub fn load_terms(path: &Path) -> Result<TermList> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let list = TermList::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    if list.flag.is_empty() {
        bail!("{}: the term list flags nothing", path.display());
    }
    Ok(list)
}

/// Count flagged identifier occurrences per package.
pub fn count_current(workspace_root: &Path, terms: &TermList) -> Result<BTreeMap<String, u64>> {
    let packages = load_scanned_packages(workspace_root)?;
    let all_roots: Vec<PathBuf> = packages.iter().map(|p| p.root_dir.clone()).collect();

    let mut counts = BTreeMap::new();
    for pkg in &packages {
        let other_roots: Vec<PathBuf> = all_roots
            .iter()
            .filter(|r| *r != &pkg.root_dir)
            .cloned()
            .collect();
        let mut total: u64 = 0;
        for file in collect_source_files(&pkg.root_dir, &other_roots) {
            match ast_auditor::scan_vocabulary_hits(&file, terms) {
                Ok(hits) => total += hits.len() as u64,
                Err(error) => bail!("failed to scan {}: {error}", file.display()),
            }
        }
        counts.insert(pkg.name.clone(), total);
    }
    Ok(counts)
}

pub fn format_baseline(counts: &BTreeMap<String, u64>) -> String {
    let mut out = String::new();
    out.push_str(
        "# Ratchet baseline for `cargo xtask check-vocabulary` (CRATIFY_SPEC.md Ratchet\n\
         # pattern; owner rule 2026-11-19). Each line is `package_name count`: the number\n\
         # of identifier occurrences in that package's production code whose words match a\n\
         # `flag` entry in `xtask/vocabulary_terms.txt`. Excluded: tests/, examples/,\n\
         # benches/, build.rs, bootstrap entrypoints, *_test.rs, *_tests.rs, and\n\
         # #[cfg(test)]/#[test] items. Comments, strings and macro bodies are not scanned.\n\
         #\n\
         # This list may only shrink or stay flat. When you rename an identifier to its\n\
         # standard term, lower the number - do not leave it stale. Regenerate with\n\
         # `cargo xtask check-vocabulary --dump-baseline`.\n",
    );
    for (name, count) in counts {
        out.push_str(&format!("{name} {count}\n"));
    }
    out
}

/// `cargo xtask check-vocabulary [--dump-baseline]`.
pub fn run(args: &[String]) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let workspace_root = find_workspace_root(&current_dir)?;
    run_with_root(&workspace_root, args)
}

/// Run the vocabulary ratchet against an explicit workspace root.
pub fn run_with_root(workspace_root: &Path, args: &[String]) -> Result<()> {
    let terms = load_terms(&terms_path(workspace_root))?;
    let current = count_current(workspace_root, &terms)?;

    if args.iter().any(|a| a == "--dump-baseline") {
        print!("{}", format_baseline(&current));
        return Ok(());
    }

    let baseline = load_baseline(&baseline_path(workspace_root))?;
    let regressions = find_regressions(&current, &baseline);

    let total: u64 = current.values().sum();
    eprintln!(
        "check-vocabulary: {} package(s) scanned, {} flagged identifier occurrence(s).",
        current.len(),
        total
    );

    if regressions.is_empty() {
        eprintln!("check-vocabulary: OK - no package exceeds its baseline.");
        return Ok(());
    }

    eprintln!("check-vocabulary: package(s) exceeding their ratchet baseline:");
    for (name, count, allowed) in &regressions {
        eprintln!("  {name}: {count} > baseline {allowed}");
    }
    bail!(
        "check-vocabulary: {} package(s) regressed past their vocabulary baseline. Use the \
         standard engineering term for the new name, or record why none exists by proposing a \
         term-list change for owner review.",
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
    fn baseline_round_trips_through_format_and_parse() -> Result<()> {
        let counts = map(&[("a", 3), ("b", 0)]);
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("baseline.txt");
        fs::write(&path, format_baseline(&counts))?;
        assert_eq!(load_baseline(&path)?, counts);
        Ok(())
    }

    #[test]
    fn missing_term_list_is_an_error_not_an_empty_list() -> Result<()> {
        let dir = tempfile::tempdir()?;
        assert!(load_terms(&dir.path().join("none.txt")).is_err());
        Ok(())
    }

    #[test]
    fn term_list_that_flags_nothing_is_an_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("terms.txt");
        fs::write(&path, "# nothing flagged\nallow|prefix|mutat|standard\n")?;
        assert!(load_terms(&path).is_err());
        Ok(())
    }

    #[test]
    fn malformed_term_list_is_an_error() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("terms.txt");
        fs::write(&path, "flag|prefix|synap\n")?;
        assert!(load_terms(&path).is_err());
        Ok(())
    }

    #[test]
    fn regression_is_a_rise_above_baseline_only() {
        let baseline = map(&[("a", 3), ("b", 2)]);
        assert_eq!(
            find_regressions(&map(&[("a", 4), ("b", 1)]), &baseline),
            vec![("a", 4, 3)]
        );
        assert!(find_regressions(&map(&[("a", 3), ("b", 2)]), &baseline).is_empty());
    }

    #[test]
    fn a_new_package_regresses_on_any_flagged_name() {
        assert_eq!(
            find_regressions(&map(&[("new_pkg", 1)]), &BTreeMap::new()),
            vec![("new_pkg", 1, 0)]
        );
    }
}
