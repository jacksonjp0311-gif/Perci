#!/usr/bin/env python3
"""Build an isolated operation-conditioned phrase candidate.

Reviewed prompt/answer pairs are preserved as supervision. The Rust trainer
factorizes each pair into sparse operation, topic, and ordinary-language views;
prompts remain hidden controls and are never rendered as answer text. The
active weights are never replaced by this script.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
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


def dialogue_operation(prompt: str) -> int:
    lower = prompt.lower()
    if any(term in lower for term in ("compare", "difference", "separates", "unlike", "versus")):
        return 1
    if any(term in lower for term in ("connect", "relate", "connected", "shared structure")):
        return 2
    if any(term in lower for term in ("what next", "next move", "next practical", "what should", "plan", "happen next")):
        return 3
    if any(term in lower for term in ("what do you mean", "what exactly", "rephrase", "differently", "plain language")) or lower.startswith(("no, i mean", "i meant")):
        return 10
    if any(term in lower for term in ("benchmark", "evidence", "how do you know", "trust the result", "actually prove", "prove the new")):
        return 8
    if any(term in lower for term in ("test", "falsif", "distinguish", "tell recombination")):
        return 4
    if any(term in lower for term in ("why", "explain", "how does", "how do ", "how can ", "what does")):
        return 5
    if any(term in lower for term in ("go deeper", "one layer", "beneath", "elaborate", "tell me more", "take semantic")):
        return 6
    if any(term in lower for term in ("creative", "original", "imagine", "fresh", "new metaphor")):
        return 7
    if any(term in lower for term in ("dont agree", "don't agree", "claimed more", "seems wrong", "conclusion follows")):
        return 9
    if any(term in lower for term in ("learn", "remember", "durable knowledge", "teach")):
        return 11
    if any(term in lower for term in ("generic", "robotic", "procedure manual", "missed what")):
        return 12
    if lower.strip() in {"interesting", "wow"} or lower.startswith(("hello", "hi ")):
        return 13
    return 0


def dialogue_signature(prompt: str) -> bytes:
    stop = {
        "about", "actually", "after", "again", "also", "another", "answer", "are", "being",
        "can", "claim", "creative", "creatively", "compare", "connect", "concrete", "could",
        "domain", "does", "difference", "each", "evidence", "emerge", "emergence", "explain", "explanation",
        "falsifiable", "failing", "from", "give", "gracefully", "have", "how", "human",
        "image", "imagine", "invariant", "into", "mark", "mechanism", "mean", "metaphor",
        "more", "next", "observation", "one", "only", "original", "pretend", "prediction",
        "preserve", "rather", "reflect", "relation", "representation", "result", "same",
        "say", "scientific", "should", "show",
        "something", "system", "teach", "teaches", "tell", "test", "that", "the", "thought", "this", "through",
        "transfer", "transformer", "what", "when", "which", "why", "with", "without",
        "would", "you", "your",
    }
    bits = bytearray(32)
    aliases = {
        "coherent": "coherence", "coherence": "coherence", "true": "truth", "truth": "truth",
        "conversation": "dialogue", "conversational": "dialogue", "dialogue": "dialogue",
        "knowledge": "learning", "learn": "learning", "learning": "learning",
        "talking": "talk", "talked": "talk", "naturally": "natural", "normal": "natural",
        "normally": "natural", "preservation": "preserve", "preserving": "preserve",
        "preserved": "preserve", "progression": "progress", "progressive": "progress",
        "promises": "promise", "changes": "change", "changed": "change", "changing": "change",
        "better": "improve", "improve": "improve", "improved": "improve", "improvement": "improve",
    }

    def canonical(word: str) -> str:
        word = word.lower()
        if word in aliases:
            return aliases[word]
        if len(word) > 6 and word.endswith("ing"):
            return word[:-3]
        if len(word) > 5 and word.endswith("ed"):
            return word[:-2]
        if len(word) > 5 and word.endswith("s"):
            return word[:-1]
        return word

    tokens = set()
    for word in re.findall(r"[A-Za-z0-9]+", prompt):
        token = canonical(word)
        if (len(token) >= 4 or token == "map") and token not in stop:
            tokens.add(token)
    for token in tokens:
        value = 0xCBF29CE484222325
        for byte in token.encode("utf-8"):
            value ^= byte
            value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        index = value & 255
        bits[index // 8] |= 1 << (index % 8)
    return bytes(bits)


def build_dialogue_field(rows: list[dict[str, object]], path: Path) -> dict[str, object]:
    records = bytearray()
    text = bytearray()
    kept = 0
    for row in rows:
        prompt = str(row["prompt"]).strip().encode("utf-8")
        response = str(row["response"]).strip().encode("utf-8")
        if not prompt or not response or len(prompt) > 65535 or len(response) > 65535:
            continue
        prompt_offset = len(text)
        text.extend(prompt)
        response_offset = len(text)
        text.extend(response)
        records.extend(
            struct.pack(
                "<B3x32sIHIH",
                dialogue_operation(str(row["prompt"])),
                dialogue_signature(str(row["prompt"])),
                prompt_offset,
                len(prompt),
                response_offset,
                len(response),
            )
        )
        kept += 1
    header = struct.pack("<8sIIQQ", b"PERCDLG1", 1, kept, 32, 32 + len(records))
    path.write_bytes(header + records + text)
    return {"records": kept, "bytes": path.stat().st_size, "sha256": sha256(path)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus",
        type=Path,
        action="append",
        help="reviewed JSONL corpus; repeat to combine curricula",
    )
    parser.add_argument(
        "--output-base",
        type=Path,
        default=ROOT / "models" / "candidates" / "perci-dialogue-continuity-v1.blng",
    )
    parser.add_argument("--order", type=int, default=4)
    parser.add_argument(
        "--no-repo",
        action="store_true",
        help="train only the reviewed dialogue corpus (default retains knowledge/ and docs/)",
    )
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

    corpora = [
        path.resolve()
        for path in (
            args.corpus
            or [ROOT / "training" / "dialogue-continuity-v1.jsonl"]
        )
    ]
    binary = args.binary.resolve()
    output_base = args.output_base.resolve()
    for corpus in corpora:
        if not corpus.is_file():
            raise SystemExit(f"missing corpus: {corpus}")
    if not binary.is_file():
        raise SystemExit(f"missing release binary: {binary}; run cargo build --release first")

    rows: list[dict[str, object]] = []
    seen_ids: set[str] = set()
    for corpus in corpora:
        for row in load_rows(corpus):
            row_id = str(row["id"])
            if row_id in seen_ids:
                raise SystemExit(f"duplicate training id across corpora: {row_id}")
            seen_ids.add(row_id)
            rows.append(row)
    output_base.parent.mkdir(parents=True, exist_ok=True)
    repo_files = []
    if not args.no_repo:
        for directory in (ROOT / "knowledge", ROOT / "docs"):
            if directory.is_dir():
                repo_files.extend(
                    path
                    for path in directory.rglob("*")
                    if path.is_file()
                    and path.suffix.lower() in {".md", ".txt", ".json", ".jsonl", ".ndjson"}
                )
    repo_files.sort()
    repo_source_bytes = sum(path.stat().st_size for path in repo_files)

    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=".jsonl", delete=False
    ) as handle:
        # Retain the broad active curriculum. A dialogue candidate is an
        # additive evolution, not a tiny replacement field that forgets the
        # repository's language and domain coverage.
        for path in repo_files:
            try:
                text = path.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            handle.write(
                json.dumps(
                    {"text": text, "source": str(path.relative_to(ROOT))},
                    ensure_ascii=False,
                )
                + "\n"
            )
        # Preserve paired supervision. Rust owns operation/topic extraction so
        # training and inference cannot silently drift onto different labels.
        for row in rows:
            record = {
                "prompt": row["prompt"],
                "response": row["response"],
                "family": row["family"],
                "topic": topic_for(row),
            }
            if args.prompt_conditioned:
                record["previous_prompt"] = row["previous_prompt"]
                record["previous_response"] = row["previous_response"]
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")
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
    dialogue = output_base.with_suffix(".bdlg")
    dialogue_stats = build_dialogue_field(rows, dialogue)
    byte_field = output_base
    missing = [str(path) for path in (byte_field, phrase, dialogue) if not path.is_file()]
    if missing:
        raise SystemExit(f"trainer did not produce expected artifacts: {', '.join(missing)}")
    receipt = {
        "schema": "perci.dialogue-candidate.v2",
        "corpora": [
            {"path": str(corpus), "sha256": sha256(corpus)} for corpus in corpora
        ],
        "train_rows": len(rows),
        "repo_curriculum_included": not args.no_repo,
        "repo_curriculum_files": len(repo_files),
        "repo_curriculum_source_bytes": repo_source_bytes,
        "response_source_only": True,
        "paired_supervision": True,
        "factorized_controls": ["operation", "topic", "local_syntax"],
        "primer_conditioned": False,
        "topic_conditioned": True,
        "prompt_conditioned": args.prompt_conditioned,
        "context_rows": sum(bool(context_for(row, args.prompt_conditioned)) for row in rows),
        "primer_count": 0,
        "order": max(1, min(4, args.order)),
        "byte_field": str(byte_field),
        "byte_field_sha256": sha256(byte_field),
        "phrase_field": str(phrase),
        "phrase_field_sha256": sha256(phrase),
        "phrase_field_bytes": phrase.stat().st_size,
        "dialogue_field": str(dialogue),
        "dialogue_field_sha256": dialogue_stats["sha256"],
        "dialogue_field_bytes": dialogue_stats["bytes"],
        "dialogue_field_records": dialogue_stats["records"],
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
