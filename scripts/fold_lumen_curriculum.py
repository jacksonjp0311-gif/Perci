#!/usr/bin/env python3
"""Fold Lumen evolve/approved JSONL into a curated Perci curriculum stage.

Does NOT rebuild the 200 MiB .pwgt by itself (that is an explicit offline step).
Reviews should redact secrets before build_weights.py.

Usage:
  python scripts/fold_lumen_curriculum.py
  python scripts/fold_lumen_curriculum.py --max 400
"""
from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FROM_LUMEN = ROOT / "training" / "from-lumen"
OUT_DIR = ROOT / "training" / "curriculum"
SECRETISH = re.compile(
    r"(?i)(api[_-]?key|secret|password|token|bearer\s+[a-z0-9\-\._]{12,}|sk-[a-z0-9]{10,})"
)


def load_rows(path: Path) -> list[dict]:
    rows: list[dict] = []
    if not path.is_file():
        return rows
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            # plain-ish lines skipped
            continue
    return rows


def redact(text: str) -> str:
    return SECRETISH.sub("[REDACTED]", text)


def normalize(row: dict) -> dict | None:
    # Cortex evolve export
    if row.get("kind") == "cortex" or "event_kind" in row:
        text = redact(str(row.get("text", "")).strip())
        if len(text) < 12:
            return None
        return {
            "messages": [
                {"role": "user", "content": "Apply this durable operational lesson."},
                {"role": "assistant", "content": text},
            ],
            "source": row.get("source", "cortex"),
            "label": row.get("event_kind", "insight"),
            "tags": row.get("tags", []),
        }
    # Learning receipt export
    if "prompt" in row and ("response" in row or "feedback" in row):
        prompt = redact(str(row.get("prompt", "")).strip())
        preferred = row.get("feedback") if row.get("kind") == "correction" else None
        response = redact(str(preferred or row.get("response", "")).strip())
        if len(prompt) < 4 or len(response) < 4:
            return None
        return {
            "messages": [
                {"role": "user", "content": prompt[:2000]},
                {"role": "assistant", "content": response[:4000]},
            ],
            "source": "lumen.learning.receipt",
            "label": str(row.get("kind", "good")).lower(),
            "receipt_id": row.get("id"),
        }
    # Already messages-shaped
    if "messages" in row:
        return row
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--max", type=int, default=500)
    args = ap.parse_args()

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    seen: set[str] = set()
    out_rows: list[dict] = []

    files = sorted(FROM_LUMEN.glob("*.jsonl"))
    for path in files:
        for raw in load_rows(path):
            norm = normalize(raw)
            if not norm:
                continue
            key = json.dumps(norm.get("messages"), sort_keys=True)
            if key in seen:
                continue
            # Drop secretish rows entirely if still present after redact fail-open
            blob = json.dumps(norm)
            if "[REDACTED]" in blob and SECRETISH.search(blob.replace("[REDACTED]", "x")):
                continue
            seen.add(key)
            norm["folded_from"] = path.name
            out_rows.append(norm)
            if len(out_rows) >= args.max:
                break
        if len(out_rows) >= args.max:
            break

    stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S")
    out = OUT_DIR / f"curriculum-folded-{stamp}.jsonl"
    with out.open("w", encoding="utf-8") as f:
        f.write(f"# folded_at={datetime.now(timezone.utc).isoformat()}\n")
        f.write(f"# count={len(out_rows)}\n")
        f.write("# note=review before build_weights.py; never auto-promote secrets\n")
        for row in out_rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")

    latest = OUT_DIR / "curriculum-latest.jsonl"
    latest.write_text(out.read_text(encoding="utf-8"), encoding="utf-8")

    print(f"folded {len(out_rows)} rows → {out}")
    print(f"latest → {latest}")
    print("next (explicit, offline, long):")
    print("  python scripts/build_weights.py --output models/perci-cognitive-v0.1.pwgt")
    print("  python scripts/verify_weights.py")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
