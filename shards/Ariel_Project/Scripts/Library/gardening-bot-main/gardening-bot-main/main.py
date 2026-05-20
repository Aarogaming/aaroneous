"""
Gardening Bot (bridged)
Original project credits: see main_orig.py and core modules.
Bridge by: Project Maelstrom - uses trainer snapshot/input bridges instead of in-process image automation.
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

RUN_TIME = 900  # seconds


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def garden_cycle():
    """
    Placeholder gardening cycle: open gardening menu, cast, close.
    Coordinates assume 1280x720; adjust as needed.
    """
    cmds = []
    cmds.append({"type": "key_press", "key": "G", "delayMs": 180})  # open gardening book
    cmds.append({"type": "click", "x": 820, "y": 520, "delayMs": 180})  # cast slot
    cmds.append({"type": "click", "x": 960, "y": 520, "delayMs": 180})  # confirm/cast
    cmds.append({"type": "key_press", "key": "ESCAPE", "delayMs": 200})  # close
    return cmds


def main():
    print("Gardening bot bridge running. Enable Input Bridge in trainer.")
    start = time.time()
    while time.time() - start < RUN_TIME:
        snap = load_snapshot()
        if snap.warnings:
            print("Warnings:", snap.warnings)

        commands = garden_cycle()
        write_commands(commands)
        time.sleep(3)


if __name__ == "__main__":
    main()
