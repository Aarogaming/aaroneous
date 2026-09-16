#!/usr/bin/env python3
"""Safely normalize explicitly selected tracked text files."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


def tracked(root: Path) -> set[str]:
    output = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], check=True, capture_output=True
    ).stdout
    return {item.decode("utf-8") for item in output.split(b"\0") if item}


def normalize(path: Path, legacy_encoding: str | None) -> bool:
    data = path.read_bytes()
    if b"\0" in data:
        raise ValueError("refusing to modify binary data")
    try:
        text = data.decode("utf-8-sig", "strict")
    except UnicodeDecodeError:
        if legacy_encoding is None:
            raise ValueError("requires an explicit legacy encoding") from None
        text = data.decode(legacy_encoding, "strict")
    normalized = text.replace("\r\n", "\n").replace("\r", "\n")
    if normalized and not normalized.endswith("\n"):
        normalized += "\n"
    repaired = normalized.encode("utf-8")
    if repaired != data:
        path.write_bytes(repaired)
        return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+", help="tracked files or directory prefixes")
    parser.add_argument("--apply", action="store_true")
    parser.add_argument(
        "--legacy-encoding",
        choices=("cp1252", "latin-1"),
        help="explicit source encoding permitted for non-UTF-8 text",
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    files = tracked(root)
    selected = sorted(
        name for name in files if any(name == item or name.startswith(item.rstrip("/") + "/") for item in args.paths)
    )
    changed = []
    for name in selected:
        path = root / name
        try:
            if args.apply and normalize(path, args.legacy_encoding):
                changed.append(name)
            elif not args.apply:
                normalize(path, args.legacy_encoding)
        except ValueError:
            continue
    print(f"normalized {len(changed)} files" if args.apply else f"validated {len(selected)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
