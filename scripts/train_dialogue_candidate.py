#!/usr/bin/env python3
"""Build an isolated native phrase candidate from reviewed dialogue examples.

The trainer intentionally feeds only approved response text to PERCPHR1. Prompt
scaffolding and held-out questions are kept in the receipt and evaluation set,
not mixed into the continuation corpus. The active weights are never replaced.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PRIMERS = (
    "<intent> a useful way to think about <topic> is",
    "<intent> one useful distinction for <topic> is",
    "<intent> a practical way to approach <topic> is",
    "<intent> when we examine <topic>, the mechanism is",
    "<intent> a deeper connection in <topic> is",
)
INTENT_PRIMERS = {
    "improvement": (
        "<intent> a measurable improvement in <topic> is",
        "<intent> the useful change in <topic> is the one that",
        "<intent> to improve <topic>, first observe whether",
    ),
    "repair": (
        "<intent> you are pointing to a dialogue failure: the missing link is",
        "<intent> the repair is to connect your meaning to the answer by",
        "<intent> I should not guess past your point; the direct issue is",
    ),
    "social": (
        "<intent> I am with you; the point worth carrying forward is",
        "<intent> that reaction matters because it notices",
        "<intent> I hear the opening; we can follow it toward",
    ),
    "capability": (
        "<intent> the language gap around <topic> is coverage and discourse state: the next test is",
        "<intent> a learned sequence can sound natural when it preserves <topic> across turns",
        "<intent> the honest boundary for <topic> is that this field learns transitions, not",
    ),
}
TOPIC_STOP = {
    "what", "why", "how", "are", "the", "this", "that", "you", "can", "does",
    "about", "tell", "explain", "connect", "give", "reflect", "creatively", "imagine",
    "original", "thought", "express", "new", "say", "differently", "more", "next",
    "one", "direct", "claim", "evidence", "test", "name", "first", "state", "which",
    "would", "could", "when", "inside", "without", "think", "between", "from", "into",
    "dont", "don't", "thats", "that's", "saying", "instead", "im", "i'm", "like",
}


def load_rows(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    ids: set[str] = set()
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_number}: expected an object")
        row_id = str(value.get("id", "")).strip()
        prompt = str(value.get("prompt", "")).strip()
        response = str(value.get("response", "")).strip()
        if not row_id or not prompt or not response:
            raise ValueError(f"{path}:{line_number}: id, prompt, and response are required")
        if row_id in ids:
            raise ValueError(f"{path}:{line_number}: duplicate id {row_id}")
        ids.add(row_id)
        if str(value.get("split", "train")) != "train":
            continue
        rows.append(
            {
                "id": row_id,
                "prompt": prompt,
                "response": response,
                "family": str(value.get("family", "general")),
                "topic": str(value.get("topic", "")).strip(),
                "previous_prompt": str(value.get("previous_prompt", "")).strip(),
                "previous_response": str(value.get("previous_response", "")).strip(),
            }
        )
    if not rows:
        raise ValueError(f"{path}: no train rows found")
    return rows


def topic_for(row: dict[str, object]) -> str:
    explicit = str(row.get("topic", "")).strip()
    if explicit:
        return explicit
    words = [
        word.lower()
        for word in re.findall(r"[A-Za-z][A-Za-z'-]+", str(row["prompt"]))
        if len(word) >= 4 and word.lower() not in TOPIC_STOP
    ]
    return " ".join(words[:3]) or "question"


def intent_for(row: dict[str, object]) -> str:
    prompt = str(row.get("prompt", "")).lower()
    if "improv" in prompt or ("evolv" in prompt and "system" in prompt):
        return "improvement"
    if ("why dont you" in prompt or "why don't you" in prompt) and any(
        word in prompt for word in ("say", "saying", "think", "mean")
    ):
        return "repair"
    if "frontier" in prompt and any(
        word in prompt for word in ("response", "language", "natural", "like")
    ):
        return "capability"
    if prompt.strip() in {"interesting", "thats interesting", "that's interesting", "wow"}:
        return "social"
    family = str(row.get("intent") or row.get("family") or "general").strip().lower()
    aliases = {
        "rephrase": "clarification",
        "scope": "clarification",
        "memory_identity": "general",
        "memory": "learning",
        "context": "learning",
        "promotion": "evidence",
        "analogy": "general",
        "emergence": "evidence",
        "native": "general",
        "natural": "general",
        "repair": "general",
    }
    return aliases.get(family, family)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def context_for(row: dict[str, object], prompt_conditioned: bool) -> str:
    if not prompt_conditioned:
        return ""
    parts = [
        str(row.get("previous_prompt", "")).strip(),
        str(row.get("previous_response", "")).strip(),
        str(row.get("prompt", "")).strip(),
    ]
    return " ".join(part for part in parts if part)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus",
        type=Path,
        default=ROOT / "training" / "dialogue-continuity-v1.jsonl",
    )
    parser.add_argument(
        "--output-base",
        type=Path,
        default=ROOT / "models" / "candidates" / "perci-dialogue-continuity-v1.blng",
    )
    parser.add_argument("--order", type=int, default=4)
    parser.add_argument(
        "--prompt-conditioned",
        action="store_true",
        help="append reviewed prior/current turn text to the hidden training context",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=ROOT / "target" / "release" / "perci.exe",
    )
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()

    corpus = args.corpus.resolve()
    binary = args.binary.resolve()
    output_base = args.output_base.resolve()
    if not corpus.is_file():
        raise SystemExit(f"missing corpus: {corpus}")
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}; run cargo build --release first")

    rows = load_rows(corpus)
    output_base.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=".jsonl", delete=False
    ) as handle:
        # Responses are the learned continuation surface. Prefix each reviewed
        # response with the same runtime primers used by BinaryPhraseModel so
        # the candidate learns continuations at the contexts it will actually
        # receive. Prompts remain evaluation metadata.
        for row in rows:
            topic = topic_for(row)
            intent = intent_for(row)
            for primer in INTENT_PRIMERS.get(intent, PRIMERS):
                context = context_for(row, args.prompt_conditioned)
                suffix = f" {context}" if context else ""
                handle.write(
                    json.dumps(
                        {
                            "text": primer.replace("<intent>", intent).replace("<topic>", topic)
                            + suffix
                            + f" {row['response']}"
                        },
                        ensure_ascii=False,
                    )
                    + "\n"
                )
        response_corpus = Path(handle.name)
    try:
        command = [
            str(binary),
            "language",
            "train",
            str(response_corpus),
            str(output_base),
            str(max(1, min(4, args.order))),
        ]
        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            encoding="utf-8",
            errors="replace",
            capture_output=True,
            check=False,
        )
    finally:
        response_corpus.unlink(missing_ok=True)
    if result.returncode != 0:
        print(result.stdout)
        print(result.stderr)
        return result.returncode

    phrase = output_base.with_suffix(".bphr")
    byte_field = output_base
    missing = [str(path) for path in (byte_field, phrase) if not path.is_file()]
    if missing:
        raise SystemExit(f"trainer did not produce expected artifacts: {', '.join(missing)}")
    receipt = {
        "schema": "perci.dialogue-candidate.v1",
        "corpus": str(corpus),
        "corpus_sha256": sha256(corpus),
        "train_rows": len(rows),
        "response_source_only": True,
        "primer_conditioned": True,
        "topic_conditioned": True,
        "prompt_conditioned": args.prompt_conditioned,
        "context_rows": sum(bool(context_for(row, args.prompt_conditioned)) for row in rows),
        "primer_count": len(PRIMERS),
        "intent_primer_counts": {
            key: len(value) for key, value in INTENT_PRIMERS.items()
        },
        "order": max(1, min(4, args.order)),
        "byte_field": str(byte_field),
        "byte_field_sha256": sha256(byte_field),
        "phrase_field": str(phrase),
        "phrase_field_sha256": sha256(phrase),
        "phrase_field_bytes": phrase.stat().st_size,
        "trainer_stdout_tail": result.stdout[-2000:],
        "promotion": "HOLD",
        "promote_recommended": False,
        "reason": "Candidate is isolated until fresh-process held-out comparison passes.",
    }
    receipt_path = (args.receipt or output_base.with_suffix(".json")).resolve()
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
