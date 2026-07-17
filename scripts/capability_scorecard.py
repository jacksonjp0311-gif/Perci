#!/usr/bin/env python3
"""Build a Perci capability scorecard from local runtime evidence.

Aggregates:
  - hardness evaluation receipt (if present or freshly runnable)
  - dialogue regression receipt
  - interaction-learning queue depth
  - live vs release binary freshness
  - capability registry status

Does not mutate weights.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def read_json(path: Path, default=None):
    if not path.is_file():
        return default
    return json.loads(path.read_text(encoding="utf-8"))


def mtime(path: Path) -> str | None:
    if not path.is_file():
        return None
    return datetime.fromtimestamp(path.stat().st_mtime, tz=timezone.utc).isoformat()


def learning_stats() -> dict:
    learning = ROOT / "memory" / "interaction-learning.jsonl"
    profile = ROOT / "memory" / "dialogue-profile.json"
    queue = ROOT / "training" / "adaptive" / "interaction-review.json"
    total = 0
    pending = 0
    if learning.is_file():
        for line in learning.read_text(encoding="utf-8", errors="replace").splitlines():
            if not line.strip():
                continue
            total += 1
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get("candidate_status") == "pending_review":
                pending += 1
    review = read_json(queue, {"candidates": []}) or {"candidates": []}
    candidates = review.get("candidates") or []
    approved = sum(1 for c in candidates if c.get("approved"))
    folded = sum(1 for c in candidates if c.get("folded"))
    return {
        "interaction_events": total,
        "pending_review_events": pending,
        "review_queue_total": len(candidates),
        "review_queue_approved": approved,
        "review_queue_folded": folded,
        "dialogue_profile": read_json(profile, {}),
    }


def gate_summary(path: Path) -> dict:
    data = read_json(path)
    if not data:
        return {"path": str(path), "present": False}
    return {
        "path": str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path),
        "present": True,
        "status": data.get("status"),
        "passed": data.get("passed"),
        "case_count": data.get("case_count"),
        "receipt_sha256": data.get("receipt_sha256"),
        "evaluated_at_utc": data.get("evaluated_at_utc"),
        "failed": [
            row.get("id")
            for row in (data.get("cases") or [])
            if not row.get("pass")
        ][:30],
        "by_capability": data.get("by_capability"),
    }


def binary_freshness() -> dict:
    live = ROOT / "target" / "live" / "release" / "perci.exe"
    release = ROOT / "target" / "release" / "perci.exe"
    live_hash = sha256_file(live)
    release_hash = sha256_file(release)
    live_m = live.stat().st_mtime if live.is_file() else None
    rel_m = release.stat().st_mtime if release.is_file() else None
    lag_seconds = None
    status = "unknown"
    if live_m is not None and rel_m is not None:
        lag_seconds = round(rel_m - live_m, 1)
        if live_hash and release_hash and live_hash == release_hash:
            status = "synced"
        elif live_m >= rel_m - 1:
            status = "live_current_or_newer"
        else:
            status = "stale_live"
    return {
        "live_path": str(live) if live.is_file() else None,
        "release_path": str(release) if release.is_file() else None,
        "live_sha256": live_hash,
        "release_sha256": release_hash,
        "live_mtime_utc": mtime(live),
        "release_mtime_utc": mtime(release),
        "release_ahead_seconds": lag_seconds,
        "status": status,
    }


def capability_status(registry: dict, hardness: dict) -> list[dict]:
    by_cap = (hardness or {}).get("by_capability") or {}
    failed_ids = set((hardness or {}).get("failed") or [])
    # also expand failed from cases if needed
    if hardness and hardness.get("present") and "failed" not in hardness:
        pass
    rows = []
    for cap in registry.get("capabilities") or []:
        cid = cap["id"]
        stats = by_cap.get(cid) or {}
        total = stats.get("total", 0)
        passed = stats.get("passed", 0)
        ratio = (passed / total) if total else None
        if total == 0:
            state = "unmeasured"
        elif passed == total:
            state = "green"
        elif passed == 0:
            state = "red"
        else:
            state = "yellow"
        rows.append({
            "id": cid,
            "name": cap.get("name"),
            "primary_layer": cap.get("primary_layer"),
            "state": state,
            "passed": passed,
            "total": total,
            "pass_rate": None if ratio is None else round(ratio, 3),
            "next_action": (
                "hold and repair failing hardness cases"
                if state in {"red", "yellow"}
                else "maintain with harder transfer variants"
                if state == "green"
                else "run evaluate_hardness.py"
            ),
        })
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--hardness",
        type=Path,
        default=ROOT / "models" / "candidates" / "evaluation-hardness-v1.json",
    )
    parser.add_argument(
        "--dialogue",
        type=Path,
        default=ROOT / "models" / "candidates" / "evaluation-v4-dialogue.json",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=ROOT / "training" / "hardness" / "capabilities.json",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "models" / "candidates" / "capability-scorecard-latest.json",
    )
    parser.add_argument(
        "--markdown",
        type=Path,
        default=ROOT / "docs" / "CAPABILITY_SCORECARD.md",
    )
    args = parser.parse_args()

    registry = read_json(args.registry, {"capabilities": []}) or {"capabilities": []}
    hardness = gate_summary(args.hardness)
    dialogue = gate_summary(args.dialogue)
    learning = learning_stats()
    binary = binary_freshness()
    caps = capability_status(registry, hardness)

    overall = "HOLD"
    if hardness.get("status") == "PASS" and binary.get("status") in {
        "synced",
        "live_current_or_newer",
    }:
        overall = "OPERATIONAL_CANDIDATE"
    elif hardness.get("status") == "PASS":
        overall = "PASS_WITH_STALE_LIVE"
    elif hardness.get("present"):
        overall = "HARDNESS_HOLD"
    else:
        overall = "UNMEASURED"

    scorecard = {
        "schema": "perci.capability-scorecard.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "overall_status": overall,
        "automatic_promotion": False,
        "north_star": registry.get("north_star"),
        "gates": {
            "hardness": hardness,
            "dialogue": dialogue,
        },
        "learning": learning,
        "binary_freshness": binary,
        "capabilities": caps,
        "recommended_next": recommended_next(caps, binary, learning, hardness),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(scorecard, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    args.markdown.parent.mkdir(parents=True, exist_ok=True)
    args.markdown.write_text(render_markdown(scorecard), encoding="utf-8")
    print(json.dumps({
        "overall_status": overall,
        "hardness": hardness.get("status"),
        "dialogue": dialogue.get("status"),
        "binary": binary.get("status"),
        "capabilities": {c["id"]: c["state"] for c in caps},
        "recommended_next": scorecard["recommended_next"],
        "json": str(args.output),
        "markdown": str(args.markdown),
    }, indent=2))
    return 0 if overall in {"OPERATIONAL_CANDIDATE", "PASS_WITH_STALE_LIVE"} else 1


def recommended_next(caps, binary, learning, hardness) -> list[str]:
    tips: list[str] = []
    if not hardness.get("present"):
        tips.append("Run: python scripts/evaluate_hardness.py")
    for cap in caps:
        if cap["state"] in {"red", "yellow"}:
            tips.append(
                f"Repair capability `{cap['id']}` at layer `{cap['primary_layer']}` "
                f"({cap['passed']}/{cap['total']} hardness cases)."
            )
    if binary.get("status") == "stale_live":
        tips.append(
            "Live chat binary is older than target/release/perci.exe — relaunch via Launch-Perci.ps1 "
            "or copy the release binary after gates pass."
        )
    approved = learning.get("review_queue_approved", 0)
    folded = learning.get("review_queue_folded", 0)
    if approved > folded:
        tips.append(
            f"{approved - folded} approved review candidates are not folded — "
            "run: python scripts/stage_interaction_learning.py --fold-approved"
        )
    if learning.get("review_queue_total", 0) == 0 and learning.get("interaction_events", 0) > 0:
        tips.append("Stage interaction evidence: python scripts/stage_interaction_learning.py")
    if not tips:
        tips.append(
            "Raise hardness: add entity-swapped / paraphrased cases to training/hardness/hardness-pack-v1.jsonl"
        )
    return tips[:8]


def render_markdown(scorecard: dict) -> str:
    lines = [
        "# Perci capability scorecard",
        "",
        f"_Generated {scorecard['generated_at_utc']}_",
        "",
        f"**Overall status:** `{scorecard['overall_status']}`",
        "",
        scorecard.get("north_star") or "",
        "",
        "## Gates",
        "",
        "| Gate | Status | Passed | Cases |",
        "|------|--------|--------|-------|",
    ]
    for name, gate in (scorecard.get("gates") or {}).items():
        if not gate.get("present"):
            lines.append(f"| {name} | missing | - | - |")
        else:
            lines.append(
                f"| {name} | {gate.get('status')} | {gate.get('passed')} | {gate.get('case_count')} |"
            )
    lines.extend([
        "",
        "## Capabilities",
        "",
        "| ID | Layer | State | Pass rate | Next |",
        "|----|-------|-------|-----------|------|",
    ])
    for cap in scorecard.get("capabilities") or []:
        rate = "-" if cap.get("pass_rate") is None else f"{cap['passed']}/{cap['total']}"
        lines.append(
            f"| `{cap['id']}` | {cap.get('primary_layer')} | {cap.get('state')} | {rate} | {cap.get('next_action')} |"
        )
    binary = scorecard.get("binary_freshness") or {}
    learning = scorecard.get("learning") or {}
    lines.extend([
        "",
        "## Binary freshness",
        "",
        f"- Status: `{binary.get('status')}`",
        f"- Live mtime: {binary.get('live_mtime_utc')}",
        f"- Release mtime: {binary.get('release_mtime_utc')}",
        f"- Release ahead (s): {binary.get('release_ahead_seconds')}",
        "",
        "## Learning queue",
        "",
        f"- Interaction events: {learning.get('interaction_events')}",
        f"- Pending review events: {learning.get('pending_review_events')}",
        f"- Review queue: {learning.get('review_queue_total')} "
        f"(approved={learning.get('review_queue_approved')}, folded={learning.get('review_queue_folded')})",
        "",
        "## Recommended next",
        "",
    ])
    for tip in scorecard.get("recommended_next") or []:
        lines.append(f"- {tip}")
    lines.append("")
    return "\n".join(lines)


if __name__ == "__main__":
    raise SystemExit(main())
