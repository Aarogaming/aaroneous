# Bridges for External Scripts

## Snapshot Bridge
- The trainer writes OCR snapshots to `Scripts/.cache/snapshot.json` about every 5 seconds.
- Use the provided `snapshot_reader.py` (Python) to consume health/mana/gold/sync data without doing your own OCR.

## Input Bridge
- When “Enable Input Bridge” is checked in the trainer, it polls `Scripts/.cache/commands.json` and executes commands.
- Commands file format (JSON array):
  ```json
  [
    {"type": "click", "x": 500, "y": 400, "delayMs": 0},
    {"type": "key_press", "key": "SPACE", "delayMs": 200}
  ]
  ```
- Supported keys are VirtualKeyCode names (e.g., `SPACE`, `VK_X`, `UP`, `LEFT`, `RIGHT`, `DOWN`, `RETURN`).
- After processing, the trainer deletes `commands.json`, so rewrite it for the next batch.

## Usage pattern (Python)
```python
from snapshot_reader import load_snapshot
import json, os, time

cache_dir = os.path.join(os.path.dirname(__file__), "..", ".cache")
commands_path = os.path.join(cache_dir, "commands.json")

# Read snapshot
snap = load_snapshot()
print("Health:", snap.health)

# Send a click and a key
commands = [
    {"type": "click", "x": 500, "y": 400, "delayMs": 0},
    {"type": "key_press", "key": "SPACE", "delayMs": 200}
]
with open(commands_path, "w", encoding="utf-8") as f:
    json.dump(commands, f)
```

Notes:
- Keep command batches small to avoid blocking the trainer UI.
- Coordinate system is absolute screen pixels. Ensure resolution matches the trainer’s configured resolution.
