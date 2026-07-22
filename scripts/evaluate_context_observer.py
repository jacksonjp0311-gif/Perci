#!/usr/bin/env python3
"""Run the held-out PERCICTX1 observer gate against one fresh local process.

This is an external observer proxy. It scores whether a reader can recover the
expected context, geometry relation, and viable next action from the rendered
answer. It does not inspect private reasoning or mutate weights.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
REPLY = re.compile(r"◆ PERCI // [^\n]*\n(.*?)(?=\n\n  ◉ YOU|\Z)", re.S)


def clamp(value: float) -> float:
    return max(0.0, min(1.0, value))


def harmonic(values: list[float]) -> float:
    if not values or any(value <= 0 for value in values):
        return 0.0
    return len(values) / sum(1.0 / value for value in values)


def tokens(text: str) -> set[str]:
    return {
        token
        for token in re.split(r"[^a-z0-9]+", text.lower())
        if len(token) > 2
    }


def overlap(left: str, right: str) -> float:
    a, b = tokens(left), tokens(right)
    if not a or not b:
        return 0.0
    return len(a & b) / max(len(a), len(b))


def score_case(case: dict, answer: str, previous: str) -> dict:
    lower = answer.lower()
    expected = [term.lower() for term in case.get("expected", [])]
    geometry = [term.lower() for term in case.get("geometry", [])]
    viability = [term.lower() for term in case.get("viability", [])]

    expected_hits = sum(1 for term in expected if term in lower)
    geometry_hits = sum(1 for term in geometry if term in lower)
    viability_hits = sum(1 for term in viability if term in lower)
    fidelity = expected_hits / max(len(expected), 1)
    geometry_score = geometry_hits / max(len(geometry), 1)
    viability_score = viability_hits / max(len(viability), 1)
    sentenceful = any(mark in answer for mark in ".?!")
    markdown = "**" in answer or "\n-" in answer or "\n•" in answer
    fluency = clamp(
        0.45
        + 0.20 * float(sentenceful)
        + 0.20 * float(not markdown)
        + 0.15 * float(len(answer.split()) <= 180)
    )
    oversmoothing = 0.35 if previous and overlap(previous, answer) > 0.86 else 0.0
    observer_score = harmonic([fluency, fidelity, viability_score, geometry_score]) * (
        1.0 - oversmoothing
    )
    passed = expected_hits == len(expected) and viability_hits >= max(1, len(viability) // 3)
    return {
        "id": case["id"],
        "prompt": case["prompt"],
        "answer": answer,
        "pass": passed,
        "expected_hits": f"{expected_hits}/{len(expected)}",
        "viability_hits": f"{viability_hits}/{len(viability)}",
        "geometry_hits": f"{geometry_hits}/{len(geometry)}",
        "fluency": round(fluency, 3),
        "context_fidelity": round(fidelity, 3),
        "viability": round(viability_score, 3),
        "geometry_alignment": round(geometry_score, 3),
        "oversmoothing_penalty": round(oversmoothing, 3),
        "observer_score": round(observer_score, 3),
    }


def parse_replies(output: str) -> list[str]:
    clean = ANSI.sub("", output)
    replies: list[str] = []
    current: list[str] | None = None
    for line in clean.splitlines():
        if "PERCI //" in line:
            if current is not None:
                replies.append(" ".join(current).strip())
            current = []
            continue
        if current is not None and "YOU" in line and (">" in line or "›" in line):
            replies.append(" ".join(current).strip())
            current = None
            continue
        if current is not None and line.strip():
            current.append(line.strip())
    if current is not None:
        replies.append(" ".join(current).strip())
    return replies


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cases",
        type=Path,
        default=ROOT / "models" / "candidates" / "evaluation-context-observer-v1.json",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=ROOT / "target" / "live" / "release" / "perci.exe",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "models" / "candidates" / "evaluation-context-observer-latest.json",
    )
    args = parser.parse_args()
    data = json.loads(args.cases.read_text(encoding="utf-8"))
    cases = data.get("cases", [])
    if not args.binary.is_file():
        raise SystemExit(f"live binary missing: {args.binary}")

    prompts = "\n".join(case["prompt"] for case in cases) + "\n/quit\n"
    with tempfile.TemporaryDirectory(prefix="perci-context-observer-") as temp:
        env = os.environ.copy()
        env["PERCI_SESSION"] = str(Path(temp) / "session.jsonl")
        proc = subprocess.run(
            [str(args.binary), "chat"],
            input=prompts,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            cwd=ROOT,
            env=env,
            timeout=120,
            check=False,
        )
    replies = parse_replies(proc.stdout)
    rows = []
    previous = ""
    for index, case in enumerate(cases):
        answer = replies[index] if index < len(replies) else ""
        row = score_case(case, answer, previous)
        rows.append(row)
        previous = answer

    passed = sum(1 for row in rows if row["pass"])
    status = "PASS" if passed == len(rows) and proc.returncode == 0 else "FAIL"
    payload = {
        "schema": "perci.context-observer.v1",
        "status": status,
        "passed": passed,
        "case_count": len(rows),
        "failed": [row["id"] for row in rows if not row["pass"]],
        "mean_observer_score": round(
            sum(row["observer_score"] for row in rows) / max(len(rows), 1), 3
        ),
        "mean_geometry_alignment": round(
            sum(row["geometry_alignment"] for row in rows) / max(len(rows), 1), 3
        ),
        "cases": rows,
        "binary": str(args.binary.relative_to(ROOT)),
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
    }
    encoded = json.dumps(payload, indent=2, ensure_ascii=False) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(encoded, encoding="utf-8")
    payload["receipt_sha256"] = hashlib.sha256(encoded.encode("utf-8")).hexdigest()
    args.output.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": status,
        "passed": passed,
        "case_count": len(rows),
        "mean_observer_score": payload["mean_observer_score"],
        "mean_geometry_alignment": payload["mean_geometry_alignment"],
        "failed": payload["failed"],
        "receipt_sha256": payload["receipt_sha256"],
    }, indent=2))
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
