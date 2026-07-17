#!/usr/bin/env python3
"""Weekly Perci evolution cycle orchestrator.

Stages:
  1. stage     — fold interaction evidence into the review queue
  2. hardness  — run hardness pack against a chosen binary
  3. scorecard — emit capability scorecard JSON + markdown
  4. tests     — optional cargo test --lib
  5. report    — print next actions (never auto-promotes)

Promotion remains explicit via scripts/promote_v2.py with --authorize.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PY = sys.executable


def run(cmd: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    print(f"\n>> {' '.join(cmd)}")
    proc = subprocess.run(
        cmd,
        cwd=str(ROOT),
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if check and proc.returncode != 0:
        raise SystemExit(proc.returncode)
    return proc


def resolve_binary(preferred: Path | None) -> Path:
    candidates = []
    if preferred:
        candidates.append(preferred)
    candidates.extend([
        ROOT / "target" / "release" / "perci.exe",
        ROOT / "target" / "live" / "release" / "perci.exe",
    ])
    for path in candidates:
        if path and path.is_file():
            return path
    raise SystemExit("No perci binary found. Build with: cargo build --release")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--steps",
        default="stage,hardness,scorecard",
        help="comma-separated: stage,hardness,scorecard,tests,report",
    )
    parser.add_argument("--perci-bin", type=Path, default=None)
    parser.add_argument("--model", type=Path, default=ROOT / "models" / "perci-cognitive-v0.3.pwgt")
    parser.add_argument("--skip-stage", action="store_true")
    parser.add_argument("--min-hardness", type=int, default=1)
    parser.add_argument(
        "--report-out",
        type=Path,
        default=ROOT / "models" / "candidates" / "evolve-cycle-latest.json",
    )
    args = parser.parse_args()

    steps = [s.strip() for s in args.steps.split(",") if s.strip()]
    if args.skip_stage:
        steps = [s for s in steps if s != "stage"]

    binary = resolve_binary(args.perci_bin)
    started = datetime.now(timezone.utc).isoformat()
    results: dict = {
        "schema": "perci.evolve-cycle.v1",
        "started_at_utc": started,
        "binary": str(binary),
        "model": str(args.model),
        "steps": {},
        "automatic_promotion": False,
    }

    if "stage" in steps:
        proc = run([PY, str(ROOT / "scripts" / "stage_interaction_learning.py")], check=False)
        results["steps"]["stage"] = {"exit": proc.returncode}
        if proc.returncode != 0:
            print("stage failed (non-fatal for later measurement)")

    if "hardness" in steps:
        hardness_out = ROOT / "models" / "candidates" / "evaluation-hardness-v1.json"
        proc = run([
            PY,
            str(ROOT / "scripts" / "evaluate_hardness.py"),
            "--perci-bin",
            str(binary),
            "--model",
            str(args.model),
            "--output",
            str(hardness_out),
            "--min-hardness",
            str(args.min_hardness),
        ], check=False)
        results["steps"]["hardness"] = {
            "exit": proc.returncode,
            "output": str(hardness_out),
        }

    if "scorecard" in steps:
        proc = run([PY, str(ROOT / "scripts" / "capability_scorecard.py")], check=False)
        results["steps"]["scorecard"] = {"exit": proc.returncode}

    if "tests" in steps:
        proc = run(["cargo", "test", "--lib", "--release"], check=False)
        results["steps"]["tests"] = {"exit": proc.returncode}

    scorecard_path = ROOT / "models" / "candidates" / "capability-scorecard-latest.json"
    scorecard = {}
    if scorecard_path.is_file():
        scorecard = json.loads(scorecard_path.read_text(encoding="utf-8"))

    results["finished_at_utc"] = datetime.now(timezone.utc).isoformat()
    results["scorecard_overall"] = scorecard.get("overall_status")
    results["recommended_next"] = scorecard.get("recommended_next") or []
    results["promotion_gate"] = {
        "auto": False,
        "command": (
            "python scripts/promote_v2.py --candidate <candidate.pwgt> "
            "--evaluation <operational.json> "
            "--supplemental-evaluation models/candidates/evaluation-hardness-v1.json "
            "--authorize \"human: reason for promote\""
        ),
        "note": "Promotion requires OPERATIONAL gates + explicit --authorize. Hardness alone is not enough.",
    }

    args.report_out.parent.mkdir(parents=True, exist_ok=True)
    args.report_out.write_text(json.dumps(results, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print("\n=== evolve cycle summary ===")
    print(json.dumps({
        "scorecard_overall": results.get("scorecard_overall"),
        "steps": {k: v.get("exit") for k, v in results["steps"].items()},
        "recommended_next": results.get("recommended_next"),
        "report": str(args.report_out),
    }, indent=2))

    # Non-zero if hardness or scorecard failed when requested
    hard_exit = results["steps"].get("hardness", {}).get("exit", 0)
    card_exit = results["steps"].get("scorecard", {}).get("exit", 0)
    if "hardness" in steps and hard_exit != 0:
        return hard_exit
    if "scorecard" in steps and card_exit != 0:
        return card_exit
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
