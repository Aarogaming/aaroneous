import json
import os
from dataclasses import dataclass
from typing import Optional


@dataclass
class MetricPair:
    current: Optional[int]
    max: Optional[int]
    confidence: float


@dataclass
class MetricSingle:
    value: Optional[int]
    confidence: float


@dataclass
class Snapshot:
    health: Optional[MetricPair]
    mana: Optional[MetricPair]
    gold: Optional[MetricSingle]
    warnings: list
    raw: dict


def load_snapshot(path: str = None) -> Snapshot:
    snapshot_path = path or os.path.join(os.path.dirname(__file__), "..", ".cache", "snapshot.json")
    with open(snapshot_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    def to_pair(key):
        item = data.get(key) or {}
        return MetricPair(
            current=item.get("Current"),
            max=item.get("Max"),
            confidence=item.get("Confidence", 0.0),
        ) if item else None

    def to_single(key):
        item = data.get(key) or {}
        return MetricSingle(
            value=item.get("Value"),
            confidence=item.get("Confidence", 0.0),
        ) if item else None

    return Snapshot(
        health=to_pair("Health"),
        mana=to_pair("Mana"),
        gold=to_single("Gold"),
        warnings=data.get("Warnings") or [],
        raw=data,
    )


if __name__ == "__main__":
    snap = load_snapshot()
    print("Health:", snap.health)
    print("Mana:", snap.mana)
    print("Gold:", snap.gold)
    print("Warnings:", snap.warnings)
