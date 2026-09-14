use ast_auditor::{UnifiedAuditReport, audit_source_file, run_workspace_audit};

fn audit(code: &str) -> anyhow::Result<UnifiedAuditReport> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("input.rs");
    std::fs::write(&file, code)?;
    let mut report = UnifiedAuditReport::default();
    audit_source_file(&file, &mut report).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(report)
}

#[test]
fn actual_hot_path_violations_are_reported() -> anyhow::Result<()> {
    for body in [
        "let x = std::vec::Vec::<u8>::new();",
        "let x = String::from(\"allocation\");",
        "let x = vec![1u8];",
        "value.unwrap();",
        "let x = std::sync::Mutex::new(0);",
        "lock.lock();",
    ] {
        let code = format!("#[doc = \"hot_path\"] fn tick() {{ {body} }}");
        let report = audit(&code)?;
        assert_eq!(report.hot_functions_scanned, 1);
        assert!(!report.hot_path_violations.is_empty(), "missed {body}");
        assert!(report.hot_path_violations.iter().all(|v| v.line == 1));
    }
    Ok(())
}

#[test]
fn executable_stubs_and_manual_layout_impls_are_rejected() -> anyhow::Result<()> {
    let stub = ["fn incomplete() { to", "do!(); }"].concat();
    let layout = ["unsafe ", "impl ", "Pod for Bad {}"].concat();
    assert_eq!(
        audit(&stub)?.soundness_violations[0].rule,
        "executable-stub"
    );
    assert_eq!(
        audit(&layout)?.soundness_violations[0].rule,
        "manual-pod-implementation"
    );
    Ok(())
}

#[test]
fn comments_and_literals_are_not_executable() -> anyhow::Result<()> {
    let code = [
        "// to",
        "do!();\nfn example() { let text = \"unimplemented",
        "!()\"; }",
    ]
    .concat();
    assert!(!audit(&code)?.has_failures());
    let report = audit(
        "#[doc = \"hot_path\"] fn reduce(input: &[u8]) -> u8 { input.first().copied().unwrap_or(0) }",
    )?;
    assert_eq!(report.hot_functions_scanned, 1);
    assert!(!report.has_failures());
    Ok(())
}

#[test]
fn malformed_missing_and_empty_targets_fail_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    assert!(run_workspace_audit([dir.path()]).is_err());
    assert!(run_workspace_audit([dir.path().join("missing")]).is_err());
    let source = dir.path().join("bad.rs");
    std::fs::write(&source, "fn broken(")?;
    assert!(run_workspace_audit([&source]).is_err());
    assert!(audit_source_file(&source, &mut UnifiedAuditReport::default()).is_err());
    let manifest = dir.path().join("Cargo.toml");
    std::fs::write(&manifest, "[broken")?;
    assert!(run_workspace_audit([&manifest]).is_err());
    Ok(())
}

#[test]
fn generated_output_is_excluded_only_during_traversal() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("lib.rs"), "pub fn valid() {}")?;
    let generated = dir.path().join("target");
    std::fs::create_dir(&generated)?;
    let broken = generated.join("generated.rs");
    std::fs::write(&broken, "fn broken(")?;
    assert!(run_workspace_audit([dir.path()]).is_ok());
    assert!(run_workspace_audit([&broken]).is_err());
    Ok(())
}

#[test]
fn ambient_access_is_actually_checked() -> anyhow::Result<()> {
    assert!(
        !audit("fn ambient() { let x = std::env::var(\"EXAMPLE\"); }")?
            .ambient_violations
            .is_empty()
    );
    Ok(())
}
