#!/usr/bin/env python3
"""Build Perci's packed binary cognitive weight file.

This is intentionally not presented as a transformer or a replacement for a
pretrained LLM.  It is a large binary associative network designed for Perci's
Bitwork architecture:

* text -> 4096-bit activation vector
* learned class masks choose a cognitive expert
* nearest stored prototype is selected with AND + POPCOUNT
* exact arithmetic/geometry remain deterministic Rust tools

The resulting file is exactly 200 MiB and can be rebuilt deterministically.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import struct
import sys
import time
from array import array
from pathlib import Path
from typing import Callable, Iterable

MAGIC = b"PERCIW02"
VERSION = 2
BITS = 4096
WORDS = BITS // 64
HEADER_SIZE = 32 * 1024
RECORD_SIZE = 8 + WORDS * 8  # variant, quality, popcount, reserved + 4096 bits
ATTEMPTS_PER_LABEL = 25_205
LABEL_ENTRY_SIZE = 16 + 16 + WORDS * 8 * 2
SEED = 0x50455243495F5631

# Adaptive morph: when PERCI_ADAPTIVE=1, mix in inject_prompts.json and
# xor SEED with PERCI_ADAPTIVE_SEED so weights actually change with curriculum.
def _adaptive_seed() -> int:
    base = SEED
    frag = os.environ.get("PERCI_ADAPTIVE_SEED", "").strip()
    if frag:
        try:
            base ^= int(frag[:16], 16)
        except ValueError:
            h = 0xCBF29CE484222325
            for b in frag.encode("utf-8"):
                h ^= b
                h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
            base ^= h
    return base & 0xFFFFFFFFFFFFFFFF


def load_adaptive_inject() -> dict[str, list[str]]:
    path = os.environ.get("PERCI_ADAPTIVE_INJECT", "").strip()
    if not path:
        default = Path(__file__).resolve().parents[1] / "training" / "adaptive" / "inject_prompts.json"
        path = str(default)
    p = Path(path)
    if not p.is_file():
        return {}
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    out: dict[str, list[str]] = {}
    if isinstance(data, dict):
        for k, v in data.items():
            if isinstance(v, list):
                out[str(k)] = [str(x).strip() for x in v if str(x).strip()]
    return out


LABELS = [
    "greeting",
    "identity",
    "english",
    "logic",
    "math",
    "geometry",
    "memory",
    "code",
    "governance",
    "planning",
    "explanation",
    "systems",
    "science",
    "creativity",
    "comparison",
    "general",
]

NOUNS = [
    "system", "pattern", "language", "memory", "agent", "network", "signal",
    "triangle", "circle", "number", "proof", "program", "repository", "model",
    "process", "energy", "geometry", "reasoning", "concept", "structure",
]
ADJECTIVES = [
    "clear", "compact", "reliable", "binary", "local", "careful", "exact",
    "robust", "simple", "complex", "governed", "efficient", "logical",
]
VERBS = [
    "explain", "analyze", "compare", "verify", "build", "improve", "test",
    "connect", "summarize", "classify", "reason about", "describe",
]
TOPICS = [
    "binary neural networks", "Rust ownership", "English grammar", "fractions",
    "coordinate geometry", "causal reasoning", "local AI", "software testing",
    "memory retrieval", "system governance", "graph connectivity", "energy",
]
SYSTEM_TERMS = [
    "Perci", "Lumen", "Cortex", "Bitwork", "NEMO", "RHP", "origin alignment",
    "append-only ledger", "governed mutation", "reflex router",
]


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def normalize(text: str) -> str:
    out = []
    previous_space = True
    for ch in text.lower():
        if ch.isascii() and ch.isalnum():
            out.append(ch)
            previous_space = False
        elif not previous_space:
            out.append(" ")
            previous_space = True
    return "".join(out).strip()


def feature_strings(text: str) -> Iterable[str]:
    norm = normalize(text)
    words = norm.split()
    yield "bias"
    yield f"len:{min(len(words), 31)}"
    for word in words:
        yield "w:" + word
        if len(word) >= 3:
            yield "p:" + word[:3]
            yield "s:" + word[-3:]
    for a, b in zip(words, words[1:]):
        yield f"b:{a}|{b}"
    compact = "_".join(words)
    for i in range(max(0, len(compact) - 2)):
        yield "c:" + compact[i : i + 3]


def encode(text: str) -> tuple[array, int]:
    words = array("Q", [0]) * WORDS
    for feature in feature_strings(text):
        h = fnv1a64(feature.encode("utf-8"))
        # Four positions per lexical feature create a sparse distributed code.
        for shift in (0, 12, 24, 36):
            pos = (h >> shift) & (BITS - 1)
            words[pos >> 6] |= 1 << (pos & 63)
    pop = sum(int(w).bit_count() for w in words)
    return words, pop


def pick(rng: random.Random, values: list[str]) -> str:
    return values[rng.randrange(len(values))]


# Loaded once per build process
_ADAPTIVE_CACHE: dict[str, list[str]] | None = None


def adaptive_prompts() -> dict[str, list[str]]:
    global _ADAPTIVE_CACHE
    if _ADAPTIVE_CACHE is None:
        _ADAPTIVE_CACHE = load_adaptive_inject()
    return _ADAPTIVE_CACHE


def prompt_for(label: str, rng: random.Random, index: int) -> tuple[str, int, int]:
    a, b, c = rng.randint(1, 999), rng.randint(1, 999), rng.randint(1, 99)
    noun, noun2 = pick(rng, NOUNS), pick(rng, NOUNS)
    topic = pick(rng, TOPICS)
    adjective = pick(rng, ADJECTIVES)
    variant = index % 32
    quality = 500 + (index % 500)

    # Adaptive injection: every 5th prototype can be a real curriculum surface form
    # with boosted quality so associative routing prefers lived experience.
    if os.environ.get("PERCI_ADAPTIVE", "").strip() in {"1", "true", "on", "yes"}:
        inject = adaptive_prompts().get(label) or []
        if inject and index % 5 == 0:
            text = inject[index % len(inject)]
            # light paraphrase noise
            if index % 10 == 0:
                text = pick(rng, ["please ", "can you ", ""]) + text
            return text, variant, min(999, quality + 220)

    templates: dict[str, list[Callable[[], str]]] = {
        "greeting": [
            lambda: pick(rng, ["hello", "hi", "hey", "good morning", "good evening"]) + " Perci",
            lambda: "can we talk for a minute",
            lambda: "hello are you there",
            lambda: "let us get started",
        ],
        "identity": [
            lambda: "who are you and what can you do",
            lambda: "tell me about Perci",
            lambda: "are you conscious or just software",
            lambda: "what kind of intelligence are you",
            lambda: "describe your limitations honestly",
        ],
        "english": [
            lambda: f"explain the English meaning of {noun}",
            lambda: f"rewrite this sentence so it is {adjective}",
            lambda: f"what part of speech is the word {noun}",
            lambda: f"improve the grammar of a sentence about {topic}",
            lambda: f"give a concise definition of {noun}",
            lambda: "help me write clear professional English",
        ],
        "logic": [
            lambda: f"if every {noun} is a {noun2} what follows logically",
            lambda: f"reason step by step about {topic}",
            lambda: f"find the contradiction in a claim about {noun}",
            lambda: f"what assumptions are required to infer {noun} from {noun2}",
            lambda: "separate evidence assumptions and conclusion",
            lambda: f"analyze whether {a} being larger than {b} proves anything else",
        ],
        "math": [
            lambda: f"calculate {a} plus {b}",
            lambda: f"what is {a} minus {b}",
            lambda: f"multiply {a} by {c}",
            lambda: f"divide {a} by {c}",
            lambda: f"solve {c} x plus {b} equals {a+b}",
            lambda: f"simplify the fraction {a} over {max(1,c)}",
            lambda: f"what percent is {c} of {a}",
        ],
        "geometry": [
            lambda: f"find triangle area with base {a} and height {c}",
            lambda: f"use pythagorean theorem with sides {c} and {c+1}",
            lambda: f"find the circumference of a circle radius {c}",
            lambda: f"find the area of a circle radius {c}",
            lambda: f"explain why triangle angles total 180 degrees",
            lambda: f"analyze a coordinate point {a%20} {b%20}",
        ],
        "memory": [
            lambda: f"remember that {noun} is {adjective}",
            lambda: f"store this fact about {topic}",
            lambda: f"recall what we said about {noun}",
            lambda: f"search local memory for {topic}",
            lambda: "what do you remember from our previous work",
        ],
        "code": [
            lambda: f"write idiomatic Rust code for a {noun}",
            lambda: f"debug a PowerShell script that handles {topic}",
            lambda: f"explain an ownership error in Rust involving {noun}",
            lambda: f"design a command line interface for {topic}",
            lambda: f"review code for safety performance and correctness",
            lambda: f"how should I test a repository that implements {topic}",
        ],
        "governance": [
            lambda: "verify permission before making a durable change",
            lambda: "explain origin alignment and the compounding gate",
            lambda: "why should an append only ledger record mutations",
            lambda: "separate observation sandbox execution and authorized execution",
            lambda: "do not claim a test passed unless it actually ran",
            lambda: "evaluate whether this action should be allowed blocked or sandboxed",
        ],
        "planning": [
            lambda: f"make a practical plan to build {topic}",
            lambda: f"break {topic} into milestones and tests",
            lambda: f"what should we build first for a {adjective} {noun}",
            lambda: "turn this large goal into ordered executable steps",
            lambda: "identify dependencies risks and acceptance criteria",
        ],
        "explanation": [
            lambda: f"explain {topic} in plain English",
            lambda: f"why does {noun} matter",
            lambda: f"how does a {adjective} {noun} work",
            lambda: f"teach me the core idea behind {topic}",
            lambda: f"give an example and a counterexample for {noun}",
        ],
        "systems": [
            lambda: f"how should {pick(rng, SYSTEM_TERMS)} connect to {pick(rng, SYSTEM_TERMS)}",
            lambda: "describe Perci as a local neuro symbolic intelligence",
            lambda: "use Bitwork as a reflex layer beneath deliberate reasoning",
            lambda: "how does Cortex memory support Lumen without uncontrolled mutation",
            lambda: "explain the rehydration protocol and origin certificate",
            lambda: "design a compact governed agent architecture",
        ],
        "science": [
            lambda: f"explain the scientific concept of {pick(rng, ['energy','force','momentum','pressure','cells','atoms','waves'])}",
            lambda: f"compare potential and kinetic energy",
            lambda: f"what evidence would test a claim about {topic}",
            lambda: "distinguish a model hypothesis measurement and conclusion",
            lambda: f"describe a controlled experiment involving {noun}",
        ],
        "creativity": [
            lambda: f"invent a name for a {adjective} {noun}",
            lambda: f"write a short story about {topic}",
            lambda: f"brainstorm unusual ideas connecting {noun} and {noun2}",
            lambda: f"design a futuristic interface for {topic}",
            lambda: "give me an original but practical concept",
        ],
        "comparison": [
            lambda: f"compare {noun} with {noun2}",
            lambda: f"what are the tradeoffs between {topic} and {pick(rng, TOPICS)}",
            lambda: f"which approach is more {adjective} and why",
            lambda: "make a fair comparison using explicit criteria",
            lambda: "contrast neural prediction with deterministic reasoning",
        ],
        "general": [
            lambda: f"what do you think about {topic}",
            lambda: f"help me understand this {noun}",
            lambda: f"what is the best next action for this {noun}",
            lambda: f"analyze this idea carefully and be honest",
            lambda: f"answer a general question involving {noun} and {noun2}",
        ],
    }
    funcs = templates[label]
    text = funcs[rng.randrange(len(funcs))]()
    # Controlled paraphrase noise broadens surface-form coverage.
    if index % 5 == 0:
        text = pick(rng, ["please ", "can you ", "I need you to ", ""]) + text
    if index % 11 == 0:
        text += pick(rng, [" clearly", " with precision", " and verify the result", " in simple terms"])
    return text, variant, quality


def top_mask(
    own: array,
    others: array,
    own_records: int,
    other_records: int,
    count: int = 512,
) -> tuple[array, array]:
    # Compare prevalence, not raw counts. After prototype deduplication domains
    # have very different record counts; raw frequency would make the largest
    # domain look like universal evidence.
    own_den = max(own_records, 1)
    other_den = max(other_records, 1)
    scores = [
        (int(own[i]) / own_den - int(others[i]) / other_den, i)
        for i in range(BITS)
    ]
    scores.sort(reverse=True)
    positive = array("Q", [0]) * WORDS
    for _, bit in scores[:count]:
        positive[bit >> 6] |= 1 << (bit & 63)

    neg_scores = [
        (int(others[i]) / other_den - int(own[i]) / own_den, i)
        for i in range(BITS)
    ]
    neg_scores.sort(reverse=True)
    negative = array("Q", [0]) * WORDS
    for _, bit in neg_scores[:count]:
        negative[bit >> 6] |= 1 << (bit & 63)
    return positive, negative


def write_header(
    fh,
    total_records: int,
    label_offsets: list[int],
    label_counts: list[int],
    positive_masks: list[array],
    negative_masks: list[array],
    corpus_sha256: bytes,
    declared_size: int,
) -> None:
    fh.seek(0)
    fixed = struct.pack(
        "<8sIIIIQQQ32s",
        MAGIC,
        VERSION,
        BITS,
        WORDS,
        len(LABELS),
        total_records,
        HEADER_SIZE,
        declared_size,
        corpus_sha256,
    )
    fh.write(fixed)
    for label_id, label in enumerate(LABELS):
        name = label.encode("ascii")[:15] + b"\0"
        name = name.ljust(16, b"\0")
        fh.write(name)
        fh.write(struct.pack("<IIII", label_id, label_offsets[label_id], label_counts[label_id], 0))
        positive_masks[label_id].tofile(fh)
        negative_masks[label_id].tofile(fh)
    position = fh.tell()
    if position > HEADER_SIZE:
        raise RuntimeError(f"header overflow: {position} > {HEADER_SIZE}")
    fh.write(b"\0" * (HEADER_SIZE - position))


def build(output: Path) -> dict:
    output.parent.mkdir(parents=True, exist_ok=True)
    # v2 builds coverage, not file size. Generate the same broad deterministic
    # curriculum surface, then retain one record per unique activation in each
    # label. Repeating an identical 4,096-bit vector adds storage, not geometry.
    records_by_label: list[list[tuple[int, int, int, array]]] = []
    label_counts: list[int] = []
    label_offsets: list[int] = []
    frequencies = [array("I", [0]) * BITS for _ in LABELS]
    corpus_digest = hashlib.sha256()
    started = time.time()
    adaptive_on = os.environ.get("PERCI_ADAPTIVE", "").strip() in {"1", "true", "on", "yes"}
    active_seed = _adaptive_seed() if adaptive_on else SEED
    if adaptive_on:
        print(f"adaptive seed active: {active_seed:#x}", flush=True)
        inj = adaptive_prompts()
        print(f"adaptive labels injected: { {k: len(v) for k,v in inj.items() if v} }", flush=True)

    generated_records = 0
    for label_id, label in enumerate(LABELS):
        rng = random.Random(active_seed ^ (label_id * 0x9E3779B97F4A7C15))
        unique: dict[bytes, tuple[int, int, int, array]] = {}
        for local_index in range(ATTEMPTS_PER_LABEL):
            prompt, variant, quality = prompt_for(label, rng, local_index)
            bits, pop = encode(prompt)
            corpus_digest.update(label.encode("ascii"))
            corpus_digest.update(b"\0")
            corpus_digest.update(prompt.encode("utf-8"))
            corpus_digest.update(b"\n")
            key = bits.tobytes()
            current = unique.get(key)
            if current is None or quality > current[1]:
                unique[key] = (variant, quality, pop, bits)
            generated_records += 1
        records = list(unique.values())
        records_by_label.append(records)
        label_counts.append(len(records))
        print(
            f"{label:12} generated={ATTEMPTS_PER_LABEL:,} unique={len(records):,} "
            f"dedup={(1.0 - len(records) / ATTEMPTS_PER_LABEL) * 100.0:.1f}%",
            flush=True,
        )

    running = 0
    for count in label_counts:
        label_offsets.append(running)
        running += count
    total_records = running
    declared_size = HEADER_SIZE + total_records * RECORD_SIZE

    # Learn masks from retained geometry, not duplicate frequency inflation.
    for label_id, records in enumerate(records_by_label):
        freq = frequencies[label_id]
        for _, _, _, bits in records:
            for word_index, word in enumerate(bits):
                value = int(word)
                while value:
                    low = value & -value
                    bit = low.bit_length() - 1
                    freq[(word_index << 6) + bit] += 1
                    value ^= low

    with output.open("wb+") as fh:
        fh.write(b"\0" * HEADER_SIZE)
        for records in records_by_label:
            for variant, quality, pop, bits in records:
                fh.write(struct.pack("<HHHH", variant, quality, pop, 0))
                bits.tofile(fh)

        all_freq = array("Q", [0]) * BITS
        for freq in frequencies:
            for i, value in enumerate(freq):
                all_freq[i] += value
        positives: list[array] = []
        negatives: list[array] = []
        for label_id, freq in enumerate(frequencies):
            others = array("Q", (int(all_freq[i]) - int(freq[i]) for i in range(BITS)))
            pos, neg = top_mask(
                freq,
                others,
                label_counts[label_id],
                total_records - label_counts[label_id],
            )
            positives.append(pos)
            negatives.append(neg)

        write_header(
            fh,
            total_records,
            label_offsets,
            label_counts,
            positives,
            negatives,
            corpus_digest.digest(),
            declared_size,
        )
        fh.flush()
        os.fsync(fh.fileno())

    sha256 = hashlib.sha256()
    with output.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            sha256.update(chunk)

    manifest = {
        "name": "Perci Cognitive Weights",
        "version": VERSION,
        "format": "PERCIW02",
        "architecture": "4096-bit sparse associative Bitwork network with signed expert evidence",
        "size_bytes": output.stat().st_size,
        "size_mib": output.stat().st_size / (1024 * 1024),
        "prototype_count": total_records,
        "bits_per_activation": BITS,
        "words_per_activation": WORDS,
        "labels": LABELS,
        "record_size": RECORD_SIZE,
        "generated_record_count": generated_records,
        "deduplicated_record_count": generated_records - total_records,
        "deduplication_ratio": 1.0 - total_records / max(generated_records, 1),
        "label_record_counts": dict(zip(LABELS, label_counts)),
        "positive_mask_bits": 512,
        "negative_mask_bits": 512,
        "sha256": sha256.hexdigest(),
        "corpus_sha256": corpus_digest.hexdigest(),
        "seed": active_seed,
        "adaptive": adaptive_on,
        "adaptive_inject": os.environ.get("PERCI_ADAPTIVE_INJECT", ""),
        "limitations": [
            "Not a transformer or general-purpose pretrained language model.",
            "Open-ended language is template and retrieval based.",
            "Exact arithmetic and geometry are delegated to deterministic tools.",
            "Knowledge is bounded by the generated curriculum, adaptive inject, and local memory.",
            "Adaptive morph changes associative prototypes; it is not gradient fine-tuning of a neural net.",
            "Promotion requires independent held-out evaluation; a successful build is not an acceptance decision.",
        ],
    }
    manifest_path = output.with_suffix(output.suffix + ".json")
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("models/candidates/perci-cognitive-v0.2.pwgt"),
    )
    args = parser.parse_args()
    manifest = build(args.output)
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
