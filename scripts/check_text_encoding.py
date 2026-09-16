#!/usr/bin/env python3
"""Read-only UTF-8/LF validation of Git-tracked files (Python 3.10+)."""

from __future__ import annotations

import argparse
from collections import Counter
import json
import os
from pathlib import Path
import stat
import subprocess
import sys


class ScanError(Exception):
    """An incomplete scan must never be reported as a clean inventory."""


def git(root: Path, *args: str, data: bytes | None = None) -> bytes:
    result = subprocess.run(
        ["git", "-C", str(root), *args], input=data, capture_output=True,
        check=False,
    )
    if result.returncode:
        raise ScanError(result.stderr.decode("utf-8", "replace").strip())
    return result.stdout


def tracked(root: Path) -> dict[str, tuple[str, str]]:
    entries = {}
    for item in git(root, "ls-files", "--stage", "-z").split(b"\0"):
        if not item:
            continue
        metadata, name = item.split(b"\t", 1)
        mode, oid, stage = metadata.decode("ascii").split()
        path = os.fsdecode(name)
        if stage != "0":
            raise ScanError(f"Unmerged index entry: {path!r}")
        entries[path] = (mode, oid)
    return entries


def attributes(root: Path, paths: list[str], source: str) -> dict:
    if not paths:
        return {}
    args = ["check-attr", "-z"]
    if source == "index":
        args.append("--cached")
    args.extend(["--stdin", "text", "eol", "working-tree-encoding"])
    data = b"".join(os.fsencode(path) + b"\0" for path in paths)
    parts = git(root, *args, data=data).split(b"\0")[:-1]
    result: dict[str, dict[str, str]] = {path: {} for path in paths}
    for i in range(0, len(parts), 3):
        path, key, value = parts[i:i + 3]
        result[os.fsdecode(path)][key.decode("ascii")] = value.decode("ascii")
    return result


def read_worktree(root: Path, path: str) -> bytes:
    # Never follow tracked links, directory junctions, or replacement symlinks.
    # This is a local validation tool, not a sandbox for concurrent hostile edits.
    parts = path.split("/")
    if any(part in ("", ".", "..") for part in parts):
        raise ScanError(f"Unsafe tracked path: {path!r}")
    current = root
    for index, part in enumerate(parts):
        current = current / part
        info = current.lstat()
        reparse = getattr(info, "st_file_attributes", 0) & 0x400
        if stat.S_ISLNK(info.st_mode) or reparse:
            raise ScanError(f"Refusing link/reparse point: {path!r}")
        expected = stat.S_ISREG if index == len(parts) - 1 else stat.S_ISDIR
        if not expected(info.st_mode):
            raise ScanError(f"Not an ordinary tracked file: {path!r}")
    with current.open("rb") as stream:
        return stream.read()


def classify(data: bytes, attrs: dict[str, str]) -> str:
    if attrs.get("text") == "unset":
        return "binary-attribute"
    if attrs.get("text") == "set":
        return "text"
    # BOMs must be diagnosed, even when UTF-16/32 includes NUL bytes.
    if data.startswith((b"\xef\xbb\xbf", b"\xff\xfe", b"\xfe\xff", b"\x00\x00\xfe\xff")):
        return "text"
    if b"\0" in data[:8000]:
        return "binary-detected"
    # Do not classify invalid UTF-8 as binary: that hides legacy encodings.
    return "text"


def violations(data: bytes, attrs: dict[str, str]) -> list[str]:
    issues = []
    if data.startswith(b"\xef\xbb\xbf"):
        issues.append("utf8-bom")
    elif data.startswith((b"\xff\xfe", b"\xfe\xff", b"\x00\x00\xfe\xff")):
        issues.append("utf16-or-utf32-bom")
    try:
        data.decode("utf-8", "strict")
    except UnicodeDecodeError:
        issues.append("invalid-utf8")
    if b"\0" in data:
        issues.append("nul-byte")
    if b"\r\n" in data:
        issues.append("crlf")
    if b"\r" in data.replace(b"\r\n", b""):
        issues.append("bare-cr")
    if data and not data.endswith(b"\n"):
        issues.append("missing-final-newline")
    if attrs.get("eol") not in (None, "unspecified", "lf"):
        issues.append("conflicting-eol-attribute")
    if attrs.get("working-tree-encoding") not in (None, "unspecified", "UTF-8", "utf-8"):
        issues.append("conflicting-encoding-attribute")
    return issues


def scan(root: Path, source: str, selected: list[str]) -> dict:
    entries = tracked(root)
    unknown = set(selected) - entries.keys()
    if unknown:
        raise ScanError(f"Paths must be exact tracked repository-relative names: {sorted(unknown)!r}")
    paths = sorted(set(selected) if selected else entries)
    attrs = attributes(root, paths, source)
    records = []
    for path in paths:
        mode, oid = entries[path]
        record = {"path": path, "classification": "", "issues": []}
        if mode in ("120000", "160000"):
            record["classification"] = "symlink" if mode == "120000" else "submodule"
        elif mode not in ("100644", "100755"):
            record.update(classification="error", error=f"Unsupported Git mode: {mode}")
        elif attrs[path].get("text") == "unset":
            record["classification"] = "binary-attribute"
        else:
            try:
                data = (git(root, "cat-file", "blob", oid) if source == "index"
                        else read_worktree(root, path))
                record["classification"] = classify(data, attrs[path])
                if record["classification"] == "text":
                    record["issues"] = violations(data, attrs[path])
            except (OSError, ScanError) as exc:
                record.update(classification="error", error=str(exc))
        records.append(record)
    counts = Counter(record["classification"] for record in records)
    issue_counts = Counter(issue for record in records for issue in record["issues"])
    return {
        "source": source,
        "summary": {
            "tracked_files": len(records),
            "classifications": dict(sorted(counts.items())),
            "files_with_issues": sum(bool(record["issues"]) for record in records),
            "issues": dict(sorted(issue_counts.items())),
            "errors": counts["error"],
        },
        "files": records,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="exact tracked paths relative to repository root")
    parser.add_argument("--root", type=Path, default=Path(__file__).absolute().parent.parent,
                        help="repository root (default: script's parent repository)")
    parser.add_argument("--source", choices=("worktree", "index"), default="worktree",
                        help="read working files or staged blobs and staged attributes")
    parser.add_argument("--inventory", "--dry-run", action="store_true",
                        help="report violations without failing (scan errors still fail)")
    parser.add_argument("--json", action="store_true", help="emit complete machine-readable inventory")
    args = parser.parse_args(argv)
    try:
        root = args.root.absolute()
        # ls-files must be scoped to the actual root, never an accidental subtree.
        if git(root, "rev-parse", "--show-prefix").strip():
            raise ScanError("--root must name the repository root")
        report = scan(root, args.source, args.paths)
    except (OSError, ScanError) as exc:
        if args.json:
            print(json.dumps({"error": str(exc)}, ensure_ascii=True))
        else:
            print(f"Scan error: {ascii(str(exc))}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=True))
    else:
        for record in report["files"]:
            if record["issues"] or record.get("error"):
                detail = record.get("error") or ", ".join(record["issues"])
                print(f"{ascii(record['path'])}: {ascii(detail)}")
        print(json.dumps(report["summary"], sort_keys=True))
    if report["summary"]["errors"]:
        return 2
    return 0 if args.inventory or not report["summary"]["files_with_issues"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
