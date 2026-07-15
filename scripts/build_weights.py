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

MAGIC = b"PERCIW01"
VERSION = 1
BITS = 4096
WORDS = BITS // 64
HEADER_SIZE = 16 * 1024
RECORD_SIZE = 8 + WORDS * 8  # variant, quality, popcount, reserved + 4096 bits
TARGET_SIZE = 200 * 1024 * 1024
LABEL_ENTRY_SIZE = 16 + WORDS * 8
SEED = 0x50455243495F5631

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


def prompt_for(label: str, rng: random.Random, index: int) -> tuple[str, int, int]:
    a, b, c = rng.randint(1, 999), rng.randint(1, 999), rng.randint(1, 99)
    noun, noun2 = pick(rng, NOUNS), pick(rng, NOUNS)
    topic = pick(rng, TOPICS)
    adjective = pick(rng, ADJECTIVES)
    variant = index % 32
    quality = 500 + (index % 500)

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


def top_mask(own: array, others: array, count: int = 512) -> tuple[array, array]:
    # Positive bits maximize within-class frequency relative to all other labels.
    scores = [(int(own[i]) * 4 - int(others[i]), i) for i in range(BITS)]
    scores.sort(reverse=True)
    positive = array("Q", [0]) * WORDS
    for _, bit in scores[:count]:
        positive[bit >> 6] |= 1 << (bit & 63)

    neg_scores = [(int(others[i]) * 4 - int(own[i]), i) for i in range(BITS)]
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
        TARGET_SIZE,
        corpus_sha256,
    )
    fh.write(fixed)
    for label_id, label in enumerate(LABELS):
        name = label.encode("ascii")[:15] + b"\0"
        name = name.ljust(16, b"\0")
        fh.write(name)
        fh.write(struct.pack("<IIII", label_id, label_offsets[label_id], label_counts[label_id], 0))
        positive_masks[label_id].tofile(fh)
    position = fh.tell()
    if position > HEADER_SIZE:
        raise RuntimeError(f"header overflow: {position} > {HEADER_SIZE}")
    fh.write(b"\0" * (HEADER_SIZE - position))


def build(output: Path) -> dict:
    output.parent.mkdir(parents=True, exist_ok=True)
    total_records = (TARGET_SIZE - HEADER_SIZE) // RECORD_SIZE
    pad_bytes = TARGET_SIZE - HEADER_SIZE - total_records * RECORD_SIZE
    base = total_records // len(LABELS)
    remainder = total_records % len(LABELS)
    label_counts = [base + (1 if i < remainder else 0) for i in range(len(LABELS))]
    label_offsets: list[int] = []
    running = 0
    for count in label_counts:
        label_offsets.append(running)
        running += count

    frequencies = [array("I", [0]) * BITS for _ in LABELS]
    corpus_digest = hashlib.sha256()
    started = time.time()

    with output.open("wb+") as fh:
        fh.write(b"\0" * HEADER_SIZE)
        global_index = 0
        for label_id, (label, count) in enumerate(zip(LABELS, label_counts)):
            rng = random.Random(SEED ^ (label_id * 0x9E3779B97F4A7C15))
            freq = frequencies[label_id]
            for local_index in range(count):
                prompt, variant, quality = prompt_for(label, rng, local_index)
                bits, pop = encode(prompt)
                corpus_digest.update(label.encode("ascii"))
                corpus_digest.update(b"\0")
                corpus_digest.update(prompt.encode("utf-8"))
                corpus_digest.update(b"\n")
                for word_index, word in enumerate(bits):
                    value = int(word)
                    while value:
                        low = value & -value
                        bit = low.bit_length() - 1
                        freq[(word_index << 6) + bit] += 1
                        value ^= low
                fh.write(struct.pack("<HHHH", variant, quality, pop, 0))
                bits.tofile(fh)
                global_index += 1
                if global_index % 50000 == 0:
                    elapsed = time.time() - started
                    rate = global_index / max(elapsed, 1e-9)
                    print(f"built {global_index:,}/{total_records:,} prototypes ({rate:,.0f}/s)", flush=True)
        if pad_bytes:
            fh.write(b"\0" * pad_bytes)

        all_freq = array("Q", [0]) * BITS
        for freq in frequencies:
            for i, value in enumerate(freq):
                all_freq[i] += value
        positives: list[array] = []
        negatives: list[array] = []
        for freq in frequencies:
            others = array("Q", (int(all_freq[i]) - int(freq[i]) for i in range(BITS)))
            pos, neg = top_mask(freq, others)
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
        "format": "PERCIW01",
        "architecture": "4096-bit sparse associative Bitwork network",
        "size_bytes": output.stat().st_size,
        "size_mib": output.stat().st_size / (1024 * 1024),
        "prototype_count": total_records,
        "bits_per_activation": BITS,
        "words_per_activation": WORDS,
        "labels": LABELS,
        "record_size": RECORD_SIZE,
        "sha256": sha256.hexdigest(),
        "corpus_sha256": corpus_digest.hexdigest(),
        "seed": SEED,
        "limitations": [
            "Not a transformer or general-purpose pretrained language model.",
            "Open-ended language is template and retrieval based.",
            "Exact arithmetic and geometry are delegated to deterministic tools.",
            "Knowledge is bounded by the generated curriculum and local memory.",
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
        default=Path("models/perci-cognitive-v0.1.pwgt"),
    )
    args = parser.parse_args()
    manifest = build(args.output)
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
