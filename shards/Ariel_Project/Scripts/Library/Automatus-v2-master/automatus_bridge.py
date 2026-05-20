"""
Automatus v2 bridge to Project Maelstrom snapshot/input bridges.
Credits: Original Automatus v2 by Drayux (https://github.com/Drayux/Automatus-v2).

This bridge reads snapshots from Scripts/.cache/snapshot.json and writes commands
to Scripts/.cache/commands.json so Automatus-like behaviors can be driven through
the trainer without direct wizwalker/memory hooks.
"""

import json
import os
import time
from pathlib import Path

import sys

# Allow import of snapshot_reader from the Library root
LIB_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(LIB_ROOT))

try:
    from snapshot_reader import load_snapshot
except ImportError:
    print("snapshot_reader.py not found; ensure PYTHONPATH includes the Library root.")
    raise

CACHE_DIR = LIB_ROOT / ".cache"
COMMANDS_PATH = CACHE_DIR / "commands.json"


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def main():
    print("Automatus bridge running. Input bridge must be enabled in the trainer.")
    while True:
        snap = load_snapshot()
        # Example heuristic: if health is low, press a key; otherwise idle.
        cmds = []
        if snap.health and snap.health.current is not None and snap.health.current < 200:
            cmds.append({"type": "key_press", "key": "VK_X", "delayMs": 0})
        # Write commands if any
        if cmds:
            write_commands(cmds)
        time.sleep(5)


if __name__ == "__main__":
    main()
