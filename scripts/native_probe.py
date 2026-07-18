#!/usr/bin/env python3
"""Run a reproducible 1,000-question native Perci dialogue probe.

The probe is deliberately broad but bounded. It does not claim that repeated
generation is learning; it records the conversation, checks topic binding and
variation, and leaves a report for human review.
"""
from __future__ import annotations

import json
import os
import re
import statistics
import subprocess
import sys
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "target" / "release" / ("perci.exe" if os.name == "nt" else "perci")
DEFAULT_TAG = "v0.8.1"

TOPICS = [
    "boundaries", "memory", "attention", "promises", "clocks", "childhood", "death", "repair",
    "trust", "uncertainty", "evidence", "geometry", "music", "code", "language", "learning",
    "change", "identity", "silence", "scale", "rhythm", "entropy", "maps", "patterns", "failure",
    "causality", "freedom", "limits", "signals", "stories", "measurement", "curiosity", "time",
    "life", "systems", "truth", "observation", "invention", "loss", "growth", "invariants",
    "translation", "feedback", "choice", "structure", "meaning", "conflict", "emergence", "care",
]

TEMPLATES = [
    "Reflect creatively: what does geometry teach us about {topic}, and what mechanism supports the analogy?",
    "Imagine a scientific explanation for {topic}; which observation would distinguish it from a nearby explanation?",
    "Give an original thought about human language and {topic}, then mark where the metaphor stops transferring.",
    "Reflect on what could emerge when {topic} changes inside a bounded system; name one testable prediction.",
    "Imagine connecting life, death, and {topic} in one idea; separate mechanism from metaphor.",
    "What does {topic} reveal about learning? Give a direct claim, evidence, and an uncertainty boundary.",
    "Reflect on {topic} as a geometry problem: what is inside, outside, and exchanged across the boundary?",
    "Give an original thought connecting code, music, and {topic}; preserve each domain's distinct mechanism.",
    "Imagine a dialogue between memory and {topic}; what would each remember and what would each forget?",
    "Why might {topic} produce trust or distrust in a system? Reflect and propose a small experiment.",
    "Connect childhood, clocks, and {topic} without claiming they share one literal physical cause.",
    "Reflect on whether {topic} is a signal, a state, or a story; what evidence would change your classification?",
    "Imagine {topic} failing gracefully. What invariant survives, and what new behavior could emerge?",
    "Give an original comparison between entropy and {topic}; state the limit of the comparison.",
    "What can a boundary teach us about {topic}? Answer in human language with one concrete image.",
    "Reflect on {topic} as a promise: what makes the promise falsifiable rather than merely comforting?",
    "Imagine a small machine learning from {topic} without a transformer; what representation would you choose?",
    "Connect attention, evidence, and {topic}; distinguish what is stored from what is currently selected.",
    "Give an original thought about how {topic} changes across scale, and identify a counterexample.",
    "Reflect on {topic} and identity: which boundary is operational, and which claim would be unjustified?",
]


def build_questions(count: int = 1000, offset: int = 0) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for index in range(offset, offset + count):
        template_index = index % len(TEMPLATES)
        topic = TOPICS[(index * 7 + template_index) % len(TOPICS)]
        rows.append(
            {
                "index": index,
                "family": template_index,
                "topic": topic,
                "prompt": TEMPLATES[template_index].format(topic=topic),
            }
        )
    return rows


def load_questions(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict) or not str(row.get("prompt", "")).strip():
            continue
        row.setdefault("index", len(rows))
        row.setdefault("family", -1)
        row.setdefault("topic", row.get("motif_a", ""))
        rows.append(row)
    return rows


def parse_transcript(raw: str, questions: list[dict[str, object]]) -> list[dict[str, object]]:
    prompt_re = re.compile(r"^\s*◉ YOU\s+›\s*(.*)$")
    response_marker = "◆ PERCI // CHAT"
    rows: list[dict[str, object]] = []
    current_prompt: str | None = None
    current_response: list[str] = []
    in_response = False

    def flush() -> None:
        nonlocal current_prompt, current_response, in_response
        if current_prompt is None:
            return
        response = " ".join(part.strip() for part in current_response).strip()
        item = questions[len(rows)].copy() if len(rows) < len(questions) else {"prompt": current_prompt}
        item["observed_prompt"] = current_prompt
        item["response"] = response
        rows.append(item)
        current_prompt = None
        current_response = []
        in_response = False

    for line in raw.splitlines():
        match = prompt_re.match(line)
        if match:
            flush()
            current_prompt = match.group(1).strip()
            continue
        if response_marker in line:
            in_response = True
            continue
        if current_prompt is not None and in_response:
            if line.strip().startswith("◆ OPENING") or line.strip().startswith("╭"):
                continue
            current_response.append(line)
    flush()
    return rows[: len(questions)]


def _slot_pair_bound(row: dict[str, object], response: str) -> bool:
    """Both motif slots appear — relation transfer, not name-parrot."""
    a = str(row.get("motif_a") or row.get("topic") or "").lower().strip()
    b = str(row.get("motif_b") or "").lower().strip()
    if not a:
        return False
    if a not in response:
        return False
    if b and b not in response:
        return False
    return True


def score(rows: list[dict[str, object]], requested: int) -> dict[str, object]:
    lengths = [len(str(row.get("response", ""))) for row in rows]
    normalized = [" ".join(str(row.get("response", "")).lower().split()) for row in rows]
    topic_hits = 0
    slot_pair_hits = 0
    slot_pair_n = 0
    punctuation = 0
    for row in rows:
        response = str(row.get("response", "")).lower()
        topic = str(row.get("topic", "")).lower()
        if topic and topic in response:
            topic_hits += 1
        if str(row.get("motif_a") or "").strip() and str(row.get("motif_b") or "").strip():
            slot_pair_n += 1
            if _slot_pair_bound(row, response):
                slot_pair_hits += 1
        if response.endswith((".", "?", "!")):
            punctuation += 1
    duplicate_count = len(rows) - len(set(normalized))
    family_rows: dict[str, list[dict[str, object]]] = {}
    for row in rows:
        family = str(row.get("family_name", row.get("family", "-1")))
        family_rows.setdefault(family, []).append(row)
    family_metrics: dict[str, dict[str, object]] = {}
    for family, items in sorted(family_rows.items()):
        family_norm = [" ".join(str(item.get("response", "")).lower().split()) for item in items]
        family_hits = sum(
            str(item.get("topic", "")).lower() in str(item.get("response", "")).lower()
            for item in items
            if str(item.get("topic", "")).strip()
        )
        pair_n = sum(
            1
            for item in items
            if str(item.get("motif_a") or "").strip() and str(item.get("motif_b") or "").strip()
        )
        pair_hits = sum(
            1
            for item in items
            if _slot_pair_bound(item, str(item.get("response", "")).lower())
        )
        family_metrics[family] = {
            "responses": len(items),
            "unique_responses": len(set(family_norm)),
            "topic_binding_rate": round(family_hits / len(items), 4) if items else 0.0,
            "slot_pair_binding_rate": round(pair_hits / pair_n, 4) if pair_n else None,
        }
    return {
        "questions_requested": requested,
        "responses_parsed": len(rows),
        "unique_responses": len(set(normalized)),
        "duplicate_responses": duplicate_count,
        "topic_binding_rate": round(topic_hits / len(rows), 4) if rows else 0.0,
        "slot_pair_binding_rate": round(slot_pair_hits / slot_pair_n, 4) if slot_pair_n else None,
        "punctuated_response_rate": round(punctuation / len(rows), 4) if rows else 0.0,
        "mean_chars": round(statistics.mean(lengths), 2) if lengths else 0.0,
        "median_chars": statistics.median(lengths) if lengths else 0,
        "min_chars": min(lengths) if lengths else 0,
        "max_chars": max(lengths) if lengths else 0,
        "families": len(set(int(row.get("family", -1)) for row in rows)),
        "topics": len(set(str(row.get("topic", "")) for row in rows)),
        "family_metrics": family_metrics,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--tag",
        default=DEFAULT_TAG,
        help="output tag; the default preserves the original v0.8.1 artifacts",
    )
    parser.add_argument("--count", type=int, default=1000, help="number of questions to ask")
    parser.add_argument("--offset", type=int, default=0, help="question-index offset for held-out variants")
    parser.add_argument("--questions-file", type=Path, help="JSONL curriculum with prompt metadata")
    parser.add_argument(
        "--phrase-weights",
        type=Path,
        help="evaluate an isolated PERCPHR1 candidate without replacing active weights",
    )
    parser.add_argument(
        "--world-weights",
        type=Path,
        help="evaluate an isolated PERCIWM1 typed world-model candidate",
    )
    parser.add_argument("--output", type=Path, help="explicit JSONL output path")
    parser.add_argument("--summary", type=Path, help="explicit summary JSON path")
    args = parser.parse_args()
    if not EXE.is_file():
        print(f"missing release binary: {EXE}; run cargo build --release first", file=sys.stderr)
        return 2
    out = (args.output or ROOT / "models" / "candidates" / f"native-probe-{args.tag}.jsonl").resolve()
    summary = (args.summary or ROOT / "models" / "candidates" / f"native-probe-{args.tag}-summary.json").resolve()
    if args.count <= 0 or args.offset < 0:
        print("--count must be positive and --offset must be non-negative", file=sys.stderr)
        return 2
    questions = load_questions(args.questions_file) if args.questions_file else build_questions(args.count, args.offset)
    if not questions:
        print("questions file contains no usable prompts", file=sys.stderr)
        return 2
    payload = "\n".join(str(row["prompt"]) for row in questions) + "\n/quit\n"
    env = os.environ.copy()
    env["PERCI_COLOR"] = "never"
    env["NO_COLOR"] = "1"
    if args.phrase_weights:
        env["PERCI_PHRASE_WEIGHTS"] = str(args.phrase_weights.resolve())
    if args.world_weights:
        env["PERCI_WORLD_WEIGHTS"] = str(args.world_weights.resolve())
    raw_path = out.with_suffix(".transcript.txt")
    with raw_path.open("w", encoding="utf-8", newline="") as raw_handle:
        process = subprocess.run(
            [str(EXE), "chat"],
            cwd=ROOT,
            input=payload,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=raw_handle,
            stderr=subprocess.PIPE,
            env=env,
            timeout=900,
        )
    raw = raw_path.read_text(encoding="utf-8", errors="replace")
    if process.returncode != 0:
        print(raw[-4000:], file=sys.stderr)
        print(process.stderr[-2000:], file=sys.stderr)
        return process.returncode
    rows = parse_transcript(raw, questions)
    metrics = score(rows, len(questions))
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")
    summary.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metrics, indent=2))
    print(f"transcript: {out}")
    print(f"summary: {summary}")
    return 0 if len(rows) == len(questions) else 1


if __name__ == "__main__":
    raise SystemExit(main())
