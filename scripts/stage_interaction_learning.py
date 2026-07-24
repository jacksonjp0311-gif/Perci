#!/usr/bin/env python3
"""Stage Perci interaction evidence for explicit curriculum review.

Default mode creates/updates a review queue. It never changes weights or the
adaptive inject file. After a human sets `approved: true` and a valid `label`,
`--fold-approved` copies only those prompts into `inject_prompts.json`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LABELS = {
    "greeting", "identity", "english", "logic", "math", "geometry", "memory",
    "code", "governance", "planning", "explanation", "systems", "science",
    "creativity", "comparison", "general",
}
KEYWORDS = [
    ("code", ("rust", "cargo", "compile", "debug", "parser", "code")),
    ("math", ("calculate", "percent", "fraction", "equation")),
    ("geometry", ("triangle", "circle", "radius", "pythag")),
    ("governance", ("permission", "authority", "promotion", "rollback")),
    ("science", ("hypothesis", "falsif", "measurement", "experiment")),
    ("planning", ("plan", "milestone", "dependency", "sequence")),
    ("identity", ("perci", "who are you", "learning from")),
    ("comparison", ("compare", "contrast", "tradeoff")),
    ("logic", ("assumption", "premise", "contradiction", "infer")),
]


def fingerprint(text: str) -> str:
    normalized = re.sub(r"\s+", " ", text.strip().lower())
    return hashlib.sha256(normalized.encode()).hexdigest()[:20]


def suggested_label(text: str) -> str:
    lower = text.lower()
    scored = [(sum(key in lower for key in keys), label) for label, keys in KEYWORDS]
    score, label = max(scored)
    return label if score else "general"


def read_json(path: Path, default):
    if not path.is_file():
        return default
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    tmp.replace(path)


def stage(source: Path, queue_path: Path) -> int:
    queue = read_json(queue_path, {"schema": "perci.interaction-review.v1", "candidates": []})
    candidates = queue.setdefault("candidates", [])
    known = {row.get("id") for row in candidates}
    added = 0
    if source.is_file():
        for line in source.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            # Explicit `/teach` events store a claim; ordinary interaction
            # evidence stores the user turn. Both remain review-only here.
            prompt = str(event.get("claim") or event.get("user", "")).strip()
            if not prompt or "[redacted-sensitive]" in prompt or len(prompt) > 1200:
                continue
            event_id = str(event.get("candidate_id") or fingerprint(prompt))
            if event_id in known:
                continue
            candidates.append({
                "id": event_id,
                "prompt": prompt,
                "signal": event.get("signal", "observation"),
                "suggested_label": suggested_label(prompt),
                "label": None,
                "approved": False,
                "source_schema": event.get("schema"),
                "source_kind": "explicit_teaching"
                if event.get("signal") == "explicit_teaching"
                else "interaction",
            })
            known.add(event_id)
            added += 1
    write_json(queue_path, queue)
    print(f"staged={added} total={len(candidates)} queue={queue_path}")
    print("review candidates, set approved=true and a valid label, then use --fold-approved")
    return 0


def fold(queue_path: Path, inject_path: Path, accept_suggested: bool = False) -> int:
    queue = read_json(queue_path, {"candidates": []})
    inject = read_json(inject_path, {})
    folded = 0
    for row in queue.get("candidates", []):
        label = row.get("label")
        if accept_suggested and row.get("approved") and not label:
            suggested = str(row.get("suggested_label", "general"))
            if suggested in LABELS:
                row["label"] = suggested
                label = suggested
        prompt = str(row.get("prompt", "")).strip()
        if not row.get("approved") or label not in LABELS or not prompt:
            continue
        prompts = inject.setdefault(label, [])
        if prompt not in prompts:
            prompts.append(prompt)
            folded += 1
        row["folded"] = True
    write_json(inject_path, inject)
    write_json(queue_path, queue)
    print(f"folded={folded} inject={inject_path}")
    print("weights unchanged; build and sealed evaluation are still required before promotion")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=ROOT / "memory/interaction-learning.jsonl")
    parser.add_argument("--queue", type=Path, default=ROOT / "training/adaptive/interaction-review.json")
    parser.add_argument("--inject", type=Path, default=ROOT / "training/adaptive/inject_prompts.json")
    parser.add_argument("--fold-approved", action="store_true")
    parser.add_argument(
        "--accept-suggested",
        action="store_true",
        help="for already-approved rows only, use the bounded suggested label",
    )
    args = parser.parse_args()
    return (
        fold(args.queue, args.inject, args.accept_suggested)
        if args.fold_approved
        else stage(args.source, args.queue)
    )


if __name__ == "__main__":
    raise SystemExit(main())
