"""
Farming_Bazaar.py (bridged)
Original author: Nico (Wizard101-Bazaar-Tc-Farming)
Bridge by: Project Maelstrom - uses trainer snapshot/input bridges instead of pyautogui.
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
CHOSEN_TC = "Deer Knight"  # retained for attribution/logging


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def buy_cycle():
    """
    Click flow adapted from original script for 1280x720 window.
    Adjust coordinates if using a different resolution.
    """
    cmds = []
    # Start confirm (buy button) near bottom right
    cmds.append({"type": "click", "x": 1150, "y": 620, "delayMs": 120})
    # OK confirmation slightly lower
    cmds.append({"type": "click", "x": 1150, "y": 660, "delayMs": 160})
    # Refresh/next page around center-right
    cmds.append({"type": "click", "x": 1180, "y": 420, "delayMs": 140})
    return cmds


def main():
    print("BazaarTCBot bridge running. Enable Input Bridge in trainer.")
    start = time.time()

    while time.time() - start < RUN_TIME:
        snap = load_snapshot()
        if snap.warnings:
            print("Warnings:", snap.warnings)

        commands = buy_cycle()
        write_commands(commands)
        time.sleep(2)  # pacing between cycles


if __name__ == "__main__":
    main()
