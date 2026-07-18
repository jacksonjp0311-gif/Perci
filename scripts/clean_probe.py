#!/usr/bin/env python3
"""Build a repetition-capped training corpus from a native Perci probe."""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path


def normalize(text: str) -> str:
    return " ".join(text.lower().split())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--limit-per-response", type=int, default=2)
    args = parser.parse_args()
    if args.limit_per_response <= 0:
        parser.error("--limit-per-response must be positive")
    rows = [
        json.loads(line)
        for line in args.source.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    seen: Counter[str] = Counter()
    kept: list[dict[str, object]] = []
    for row in rows:
        response = str(row.get("response", "")).strip()
        key = normalize(response)
        if not key or seen[key] >= args.limit_per_response:
            continue
        seen[key] += 1
        kept.append(row)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        for row in kept:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(json.dumps({"input_rows": len(rows), "output_rows": len(kept), "unique_responses": len(seen)}, indent=2))
    print(f"corpus: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
