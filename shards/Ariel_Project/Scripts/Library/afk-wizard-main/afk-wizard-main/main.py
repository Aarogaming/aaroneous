"""
afk-wizard (bridged)
Original author: (see main_orig.py)
Bridge by: Project Maelstrom - uses trainer snapshot/input bridges instead of pyautogui.
"""

import json
import sys
import time
from pathlib import Path
from random import randint

LIB_ROOT = Path(__file__).resolve().parents[3]  # .../Scripts/Library
sys.path.insert(0, str(LIB_ROOT))

try:
    from snapshot_reader import load_snapshot
except ImportError:
    print("snapshot_reader.py not found; ensure PYTHONPATH includes the Library root.")
    raise

CACHE_DIR = LIB_ROOT / ".cache"
COMMANDS_PATH = CACHE_DIR / "commands.json"

RUN_TIME = 3600  # seconds


def write_commands(commands):
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(COMMANDS_PATH, "w", encoding="utf-8") as f:
        json.dump(commands, f)


def wander_commands():
    cmds = []
    # Simple wander: alternate WASD with random choice; using 1280x720 no coords needed
    choices = [("W", 400), ("S", 400), ("A", 400), ("D", 400)]
    key, delay = choices[randint(0, len(choices) - 1)]
    cmds.append({"type": "key_press", "key": key, "delayMs": delay})
    return cmds


def battle_cycle():
    """
    Placeholder: press pass and a spell click region.
    Adjust coordinates to match your resolution (assumes 1280x720).
    """
    cmds = []
    cmds.append({"type": "click", "x": 880, "y": 610, "delayMs": 140})  # pass
    cmds.append({"type": "click", "x": 980, "y": 610, "delayMs": 140})  # spell
    return cmds


def main():
    print("afk-wizard bridge running. Enable Input Bridge in trainer.")
    start = time.time()
    while time.time() - start < RUN_TIME:
        snap = load_snapshot()
        if snap.warnings:
            print("Warnings:", snap.warnings)

        commands = []
        # Heuristic: if mana is low, just wander; otherwise do battle cycle
        if snap.mana and snap.mana.current is not None and snap.mana.current < 20:
            commands.extend(wander_commands())
        else:
            commands.extend(battle_cycle())

        write_commands(commands)
        time.sleep(2)


if __name__ == "__main__":
    main()
