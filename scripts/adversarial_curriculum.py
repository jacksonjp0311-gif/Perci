#!/usr/bin/env python3
"""Build a held-out adversarial curriculum for native Perci.

The normal emergence curriculum measures breadth and transfer.  This pack
targets failure modes that can look fluent while being wrong: paraphrase
collapse, negation loss, entity substitution, contradiction, and boundary
overreach.  It emits JSONL metadata compatible with ``native_probe.py``; the
answers are judged by topic binding, variation, punctuation, and human review
of the family-specific constraint.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

MOTIFS = [
    "boundary", "memory", "evidence", "repair", "trust", "uncertainty", "scale",
    "identity", "signal", "learning", "entropy", "structure", "attention", "change",
]

FAMILIES = [
    "paraphrase",
    "negation",
    "entity_swap",
    "contradiction",
    "boundary_limit",
    "counterfactual",
]


def build_questions(count: int = 300, offset: int = 0) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index in range(offset, offset + count):
        family_id = index % len(FAMILIES)
        family = FAMILIES[family_id]
        motif_a = MOTIFS[(index * 5 + family_id) % len(MOTIFS)]
        motif_b = MOTIFS[(index * 11 + 3) % len(MOTIFS)]
        if motif_b == motif_a:
            motif_b = MOTIFS[(MOTIFS.index(motif_b) + 1) % len(MOTIFS)]
        if family == "paraphrase":
            prompt = (
                f"State the same testable relation in new words: how does {motif_a} "
                f"change what a bounded system can exchange, and what observation would check it?"
            )
        elif family == "negation":
            prompt = (
                f"Do not assume that {motif_a} automatically proves a mechanism. "
                f"What can be said, and what evidence is still missing?"
            )
        elif family == "entity_swap":
            prompt = (
                f"An unfamiliar device called Quoril-7 has {motif_a} and {motif_b}. "
                "Transfer one relation to it without treating the invented name as evidence."
            )
        elif family == "contradiction":
            prompt = (
                f"One report says {motif_a} increases when {motif_b} is removed; another says it "
                "decreases. List the competing explanations and the smallest discriminating test."
            )
        elif family == "boundary_limit":
            prompt = (
                f"Connect {motif_a} and {motif_b}, then name the exact boundary where the analogy "
                "stops transferring into a literal causal claim."
            )
        else:
            prompt = (
                f"If {motif_a} were reversed while the surrounding system stayed fixed, what would "
                f"remain invariant, and what new observation would falsify that prediction about {motif_b}?"
            )
        rows.append(
            {
                "index": index,
                "family": family_id,
                "family_name": family,
                "topic": motif_a,
                "motif_a": motif_a,
                "motif_b": motif_b,
                "prompt": prompt,
                "constraint": family,
            }
        )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--count", type=int, default=300)
    parser.add_argument("--offset", type=int, default=0)
    args = parser.parse_args()
    if args.count <= 0 or args.offset < 0:
        parser.error("--count must be positive and --offset must be non-negative")
    rows = build_questions(args.count, args.offset)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(json.dumps({"questions": len(rows), "families": len(FAMILIES), "output": str(args.output)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
