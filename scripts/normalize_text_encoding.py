#!/usr/bin/env python3
"""Safely normalize explicitly selected tracked UTF-8 text files."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


def tracked(root: Path) -> set[str]:
    output = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"], check=True, capture_output=True
    ).stdout
    return {item.decode("utf-8") for item in output.split(b"\0") if item}


def normalize(path: Path) -> bool:
    data = path.read_bytes()
    if data.startswith(b"\xef\xbb\xbf") or b"\r" in data or b"\0" in data:
        raise ValueError("requires BOM, line-ending, or binary classification repair")
    data.decode("utf-8", "strict")
    if data and not data.endswith(b"\n"):
        path.write_bytes(data + b"\n")
        return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+", help="tracked files or directory prefixes")
    parser.add_argument("--apply", action="store_true")
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
            if args.apply and normalize(path):
                changed.append(name)
            elif not args.apply:
                normalize(path)
        except ValueError:
            continue
    print(f"normalized {len(changed)} files" if args.apply else f"validated {len(selected)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
