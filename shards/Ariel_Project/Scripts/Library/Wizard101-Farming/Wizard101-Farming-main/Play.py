"""
Play.py (bridged)
Original project: Wizard101-Farming
Bridge by: Project Maelstrom - uses trainer snapshot/input bridges instead of pyautogui/image matching.
"""

import json
import sys
import time
from pathlib import Path

LIB_ROOT = Path(__file__).resolve().parents[3]  # .../Scripts/Library
sys.path.insert(0, str(LIB_ROOT))

try:
    from snapshot_reader import load_snapshot
except ImportError:
    print("snapshot_reader.py not found; ensure PYTHONPATH includes the Library root.")
    raise

CACHE_DIR = LIB_ROOT / ".cache"
COMMANDS_PATH = CACHE_DIR / "commands.json"

RUN_TIME = 300  # seconds


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def farming_cycle():
    """
    Placeholder cycle: click to interact and press a key.
    Adjust coordinates for your resolution (assumes 1280x720). This targets center-ish interactables.
    """
    cmds = []
    cmds.append({"type": "click", "x": 900, "y": 500, "delayMs": 140})
    cmds.append({"type": "click", "x": 980, "y": 560, "delayMs": 140})
    cmds.append({"type": "key_press", "key": "SPACE", "delayMs": 160})
    return cmds


def main():
    print("Wizard101-Farming bridge running. Enable Input Bridge in trainer.")
    start = time.time()
    while time.time() - start < RUN_TIME:
        snap = load_snapshot()
        if snap.warnings:
            print("Warnings:", snap.warnings)
        commands = farming_cycle()
        write_commands(commands)
        time.sleep(2)


if __name__ == "__main__":
    main()
