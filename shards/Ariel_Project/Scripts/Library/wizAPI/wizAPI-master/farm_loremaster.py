"""
farm_loremaster.py (bridged)
Original author: GloXFX (wizAPI)
Bridge by: Project Maelstrom - uses trainer snapshot/input bridges instead of native automation.
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

# User settings
RUN_TIME = 300  # seconds


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def loremaster_cycle():
    """
    Placeholder cycle: press space and click a target region.
    Coordinates assume 1280x720; adjust as needed.
    """
    cmds = []
    cmds.append({"type": "key_press", "key": "SPACE", "delayMs": 100})
    cmds.append({"type": "click", "x": 900, "y": 500, "delayMs": 150})
    return cmds


def main():
    print("wizAPI farm_loremaster bridge running. Enable Input Bridge in trainer.")
    start = time.time()
    while time.time() - start < RUN_TIME:
        snap = load_snapshot()
        if snap.warnings:
            print("Warnings:", snap.warnings)
        commands = loremaster_cycle()
        write_commands(commands)
        time.sleep(2)


if __name__ == "__main__":
    main()
