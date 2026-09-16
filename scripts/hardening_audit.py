#!/usr/bin/env python3
"""Machine-checkable baseline for the Aaroneous hardening audit program."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

LENSES = tuple(f"H{number:02d}" for number in range(1, 21))
RETIRED_ROOT_DOCS = {
    "TODO.md",
    "MASTER_ROADMAP.md",
    "RELEASE_NOTES.md",
    "STRATEGIC_VISION.md",
    "WHAT_EXISTS_TODAY.md",
    "TEXT_ENCODING.md",
}
ACTIVE_DOC_ROOTS = ("docs",)
MARKDOWN_LINK = re.compile(r"\[[^]]+\]\(([^)#]+)(?:#[^)]*)?\)")


def repository_root() -> Path:
    return Path(__file__).resolve().parent.parent


def active_markdown(root: Path) -> list[Path]:
    files = [
        path
        for directory in ACTIVE_DOC_ROOTS
        for path in (root / directory).rglob("*.md")
        if "archive" not in path.parts
    ]
    files.extend(
        root / name
        for name in ("README.md", "CONTRIBUTING.md", "SECURITY.md", "AGENTS.md", "CHANGELOG.md")
        if (root / name).is_file()
    )
    return sorted(files)


def broken_relative_links(root: Path) -> list[dict[str, str]]:
    broken = []
    for path in active_markdown(root):
        text = path.read_text(encoding="utf-8")
        for target in MARKDOWN_LINK.findall(text):
            if target.startswith(("http:", "https:", "mailto:", "file:")):
                continue
            if not (path.parent / target).exists():
                broken.append(
                    {
                        "file": path.relative_to(root).as_posix(),
                        "target": target,
                    }
                )
    return broken


def source_inventory(root: Path) -> dict[str, int]:
    files = [
        path
        for base in ("core", "crates", "dev")
        for path in (root / base).rglob("*.rs")
        if "target" not in path.parts
    ]
    text = "\n".join(path.read_text(encoding="utf-8") for path in files)
    return {
        "rust_files": len(files),
        "unsafe_blocks": len(re.findall(r"\bunsafe\s*\{", text)),
        "extern_c_symbols": len(re.findall(r'extern\s+"C"', text)),
        "unwrap_calls": len(re.findall(r"\.unwrap\(\)", text)),
        "expect_calls": len(re.findall(r"\.expect\(", text)),
        "ambient_environment_calls": len(
            re.findall(r"std::env::(?:var|var_os|set_var|remove_var|temp_dir|current_dir)", text)
        ),
        "canonicalize_calls": len(re.findall(r"\.canonicalize\(\)", text)),
        "todo_macros": len(re.findall(r"\btodo!\(", text)),
        "unimplemented_macros": len(re.findall(r"\bunimplemented!\(", text)),
    }


def command_available(command: str) -> bool:
    return subprocess.run(
        [command, "--version"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    ).returncode == 0


def audit(root: Path) -> dict[str, object]:
    matrix = (root / "docs/HARDENING_AUDIT_MATRIX.md").read_text(encoding="utf-8")
    ci = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    gate = (root / "scripts/agent_check.sh").read_text(encoding="utf-8")
    matrix_lenses = sorted(set(re.findall(r"\bH\d{2}\b", matrix)))
    retired_present = sorted(name for name in RETIRED_ROOT_DOCS if (root / name).exists())
    checks = {
        "matrix_covers_all_lenses": matrix_lenses == list(LENSES),
        "ci_runs_hardening_audit": "scripts/hardening_audit.py --check" in ci,
        "canonical_gate_runs_hardening_audit": "scripts/hardening_audit.py --check" in gate,
        "retired_root_documents_absent": not retired_present,
        "active_document_links_valid": not broken_relative_links(root),
    }
    return {
        "checks": checks,
        "errors": [
            name for name, passed in checks.items() if not passed
        ],
        "matrix_lenses": matrix_lenses,
        "retired_root_documents_present": retired_present,
        "broken_relative_links": broken_relative_links(root),
        "source_inventory": source_inventory(root),
        "tooling": {"cargo_audit_available": command_available("cargo-audit")},
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail when baseline policy checks fail")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    report = audit(repository_root())
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print("Hardening audit baseline")
        for name, passed in report["checks"].items():
            print(f"{'PASS' if passed else 'FAIL'} {name}")
        print("Inventory:", json.dumps(report["source_inventory"], sort_keys=True))
        print("cargo-audit available:", report["tooling"]["cargo_audit_available"])
    return 1 if args.check and report["errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())

