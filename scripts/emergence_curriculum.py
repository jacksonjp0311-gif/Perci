#!/usr/bin/env python3
"""Mine Perci probe motifs and generate a transfer-focused next curriculum."""
from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path

STOP = set(
    "a about an and answer as at are can connect does each for from give has have how i if imagine in is it me not of one only or reflect same the then that this to under was what when which why with without you your original thought human language useful way literal physical share named system small".split()
)
TEMPLATES = (
    "Take the relation between {a} and {b} and apply it to {domain}. What mechanism transfers, what breaks, and what observation would decide?",
    "Suppose {a} changes while {b} remains stable in {domain}. Give two explanations and the smallest test that separates them.",
    "Construct a counterexample where {a} appears in {domain} but {b} fails. State the boundary of the analogy and the evidence needed.",
    "Connect {domain_a}, {domain_b}, and {a} in one coherent idea. Keep their mechanisms distinct and give one falsifiable prediction about {b}.",
    "An answer claims that {a} explains {b}. How would you distinguish genuine transfer from memorized wording in {domain}?",
    "Imagine an unseen system called {entity}. Map {a} and {b} onto it, then name the relation that survives and the point where it stops transferring.",
)
DOMAINS = (
    "a biological membrane",
    "a distributed service",
    "a childhood memory",
    "a musical rhythm",
    "a compiler pipeline",
    "a clock network",
    "a community promise",
    "a dying ecosystem",
    "a geometric construction",
    "a learning agent",
    "a translation system",
    "an unfamiliar machine",
)
EMERGENT_SEEDS = (
    "boundary",
    "structure",
    "evidence",
    "mechanism",
    "state",
    "relation",
    "transfer",
    "invariant",
    "scale",
    "repair",
    "memory",
    "entropy",
    "trust",
    "identity",
    "learning",
    "uncertainty",
    "signal",
    "promise",
    "change",
    "failure",
    "measurement",
    "pattern",
    "composition",
    "attention",
)
PAIR_STOP = set(
    "between while would two teach examine name test checkable constrained invention free treat active fails mechanisms image like switchyard sparse tracks few rails".split()
)
ENTITIES = (
    "Nara-7",
    "AetherBus",
    "QuorilNode",
    "the silent lattice",
    "a machine with no recorded history",
    "a system whose boundaries are unknown",
)
QUALIFIERS = (
    "Use a concrete example.",
    "Name one hidden assumption.",
    "State what would falsify the bridge.",
    "Keep mechanism separate from metaphor.",
    "Include a failure mode.",
    "Give the smallest next experiment.",
    "Compare two scales.",
    "Track what is stored and what is selected.",
    "Mark the uncertainty explicitly.",
    "Use an unfamiliar entity rather than a familiar example.",
    "Describe the boundary before the analogy.",
    "Explain what changes under perturbation.",
    "Separate observation from interpretation.",
    "Name the invariant, if one survives.",
    "Give a counterexample before concluding.",
    "Use one sentence for the claim and one for the test.",
    "Avoid claiming that the domains share a physical cause.",
    "State which premise carries the inference.",
    "Prefer a measurable prediction to an image.",
    "Describe how the relation could fail.",
    "Bind the answer to the named subject.",
    "Use a different mechanism in each domain.",
    "Name the evidence that would change your mind.",
    "End with the smallest reversible action.",
)


def content_tokens(text: str) -> list[str]:
    return [
        token
        for token in re.findall(r"[a-z]{3,}", text.lower())
        if token not in STOP
    ]


def mine_pairs(rows: list[dict[str, object]]) -> tuple[list[str], list[tuple[str, str]]]:
    words = Counter()
    pairs = Counter()
    for row in rows:
        tokens = list(dict.fromkeys(content_tokens(str(row.get("response", "")))))
        words.update(tokens)
        for left_index, left in enumerate(tokens):
            for right in tokens[left_index + 1 : left_index + 5]:
                if left != right:
                    pairs[(left, right)] += 1
    mined = [word for word, _ in words.most_common(48) if word not in PAIR_STOP]
    motifs = list(dict.fromkeys((*EMERGENT_SEEDS, *mined)))[:24]
    mined_pairs = [
        pair
        for pair, _ in pairs.most_common(96)
        if pair[0] in EMERGENT_SEEDS and pair[1] in EMERGENT_SEEDS
    ]
    seed_pairs = list(zip(EMERGENT_SEEDS, EMERGENT_SEEDS[1:] + EMERGENT_SEEDS[:1]))
    strong_pairs = list(dict.fromkeys((*seed_pairs, *mined_pairs)))
    if len(motifs) < 8:
        motifs = list(EMERGENT_SEEDS)
    if len(strong_pairs) < 8:
        strong_pairs = seed_pairs
    return motifs, strong_pairs


def build_curriculum(rows: list[dict[str, object]], count: int, offset: int = 0) -> list[dict[str, object]]:
    motifs, pairs = mine_pairs(rows)
    output: list[dict[str, object]] = []
    seen: set[str] = set()
    index = offset
    while len(output) < count:
        family = index % len(TEMPLATES)
        pair = pairs[index % len(pairs)]
        a, b = pair
        domain = DOMAINS[(index * 5 + family) % len(DOMAINS)]
        domain_a = DOMAINS[(index * 3 + family) % len(DOMAINS)]
        domain_b = DOMAINS[(index * 7 + family + 1) % len(DOMAINS)]
        entity = ENTITIES[(index * 11 + family) % len(ENTITIES)]
        pass_index = index // len(pairs)
        qualifier = f"{QUALIFIERS[pass_index % len(QUALIFIERS)]} This is transfer variant {pass_index + 1}."
        prompt = TEMPLATES[family].format(
            a=a,
            b=b,
            domain=domain,
            domain_a=domain_a,
            domain_b=domain_b,
            entity=entity,
        )
        prompt = f"{prompt} {qualifier}"
        if prompt in seen:
            index += 1
            continue
        seen.add(prompt)
        output.append(
            {
                "index": len(output),
                "curriculum_index": index,
                "family": family,
                "topic": a,
                "motif_a": a,
                "motif_b": b,
                "prompt": prompt,
                "source": "emergence_curriculum",
                "source_rows": len(rows),
                "motif_inventory": motifs[:12],
            }
        )
        index += 1
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--count", type=int, default=1000)
    parser.add_argument("--offset", type=int, default=0)
    args = parser.parse_args()
    if args.count <= 0:
        parser.error("--count must be positive")
    if args.offset < 0:
        parser.error("--offset must be non-negative")
    rows = [
        json.loads(line)
        for line in args.source.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    curriculum = build_curriculum(rows, args.count, args.offset)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        for row in curriculum:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")
    motifs, pairs = mine_pairs(rows)
    print(json.dumps({"questions": len(curriculum), "source_rows": len(rows), "motifs": motifs[:24], "pairs": pairs[:16]}, indent=2))
    print(f"curriculum: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
