#!/usr/bin/env python3
"""Full 120-question adversarial re-probe with slot_pair_binding_rate (v0.8.6).

Uses `perci ask` per question (reliable on Windows) and writes:
  models/candidates/native-probe-v0.8.6-adversarial-heldout.jsonl
  models/candidates/native-probe-v0.8.6-adversarial-heldout-summary.json
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "target" / "release" / ("perci.exe" if os.name == "nt" else "perci")
QUESTIONS = ROOT / "models" / "candidates" / "adversarial-v0.8.4-heldout.jsonl"
OUT = ROOT / "models" / "candidates" / "native-probe-v0.8.6-adversarial-heldout.jsonl"
SUMMARY = ROOT / "models" / "candidates" / "native-probe-v0.8.6-adversarial-heldout-summary.json"


def ask(prompt: str) -> str:
    p = subprocess.run(
        [str(EXE), "ask", prompt],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=120,
    )
    return ((p.stdout or "") + (p.stderr or "")).strip()


def main() -> int:
    if not EXE.is_file():
        print(f"missing {EXE}; cargo build --release first", file=sys.stderr)
        return 2
    if not QUESTIONS.is_file():
        print(f"missing {QUESTIONS}", file=sys.stderr)
        return 2
    rows = []
    for line in QUESTIONS.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        rows.append(json.loads(line))
    # ensure 120
    rows = rows[:120]
    results = []
    family_stats: dict[str, dict[str, int]] = defaultdict(lambda: {"n": 0, "topic": 0, "pair": 0, "pair_n": 0})
    for i, row in enumerate(rows):
        prompt = str(row.get("prompt", ""))
        resp = ask(prompt)
        topic = str(row.get("topic") or row.get("motif_a") or "").lower()
        a = str(row.get("motif_a") or "").lower()
        b = str(row.get("motif_b") or "").lower()
        low = resp.lower()
        topic_hit = bool(topic) and topic in low
        pair_hit = bool(a and b) and a in low and b in low
        fam = str(row.get("family_name") or row.get("constraint") or row.get("family") or "?")
        family_stats[fam]["n"] += 1
        if topic_hit:
            family_stats[fam]["topic"] += 1
        if a and b:
            family_stats[fam]["pair_n"] += 1
            if pair_hit:
                family_stats[fam]["pair"] += 1
        out_row = dict(row)
        out_row["response"] = resp
        out_row["topic_bound"] = topic_hit
        out_row["slot_pair_bound"] = pair_hit
        results.append(out_row)
        if (i + 1) % 20 == 0:
            print(f"  … {i+1}/{len(rows)}", flush=True)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", encoding="utf-8") as f:
        for r in results:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")

    topic_rate = sum(1 for r in results if r.get("topic_bound")) / max(1, len(results))
    pair_n = sum(1 for r in results if str(r.get("motif_a") or "") and str(r.get("motif_b") or ""))
    pair_rate = sum(1 for r in results if r.get("slot_pair_bound")) / max(1, pair_n)
    fam_metrics = {}
    for fam, st in sorted(family_stats.items()):
        fam_metrics[fam] = {
            "responses": st["n"],
            "topic_binding_rate": round(st["topic"] / st["n"], 4) if st["n"] else 0.0,
            "slot_pair_binding_rate": round(st["pair"] / st["pair_n"], 4) if st["pair_n"] else None,
        }
    summary = {
        "tag": "v0.8.6-adversarial-heldout",
        "questions_requested": len(rows),
        "responses_parsed": len(results),
        "topic_binding_rate": round(topic_rate, 4),
        "slot_pair_binding_rate": round(pair_rate, 4),
        "family_metrics": fam_metrics,
        "claim_boundary": "measured probe only — not AGI; no weight promote",
    }
    SUMMARY.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))
    print(f"transcript: {OUT}")
    print(f"summary: {SUMMARY}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
