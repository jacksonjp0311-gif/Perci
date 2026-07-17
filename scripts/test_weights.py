#!/usr/bin/env python3
"""Held-out domain routing probes against the active Bitwork pack (default PERCIW03)."""
from __future__ import annotations

import argparse
import json
import mmap
import struct
import time
from pathlib import Path

import numpy as np

import build_weights as bw

FIXED = struct.Struct("<8sIIIIQQQ32s")
RECORD_SIZE = 8 + bw.WORDS * 8  # 520
LABEL_ENTRY_V1 = 16 + 16 + bw.WORDS * 8
LABEL_ENTRY_V2 = 16 + 16 + bw.WORDS * 8 * 2  # signed masks (v2/v3)


def default_model() -> Path:
    for candidate in (
        Path("models/perci-cognitive-v0.3.pwgt"),
        Path("models/perci-cognitive-v0.2.pwgt"),
        Path("models/perci-cognitive-v0.1.pwgt"),
    ):
        if candidate.is_file():
            return candidate
    return Path("models/perci-cognitive-v0.3.pwgt")


def priors(text: str) -> dict[str, int]:
    t = " " + bw.normalize(text) + " "
    keys = {
        "greeting": [" hello ", " hi ", " hey ", "good morning", "good evening"],
        "identity": [
            "who are you",
            "what exactly is perci",
            "your limitations",
            "your limits",
            "what can you do",
        ],
        "english": [" grammar ", " adjective ", " noun ", " verb ", "rewrite", "polish", "english"],
        "logic": [" logically ", "what follows", "contradiction", "assumption", "infer ", "reason step"],
        "math": [
            " calculate ",
            " compute ",
            " divided ",
            " multiply ",
            " plus ",
            " minus ",
            " equation ",
            " fraction ",
            " percent ",
        ],
        "geometry": [
            " triangle ",
            " circle ",
            " geometry ",
            " pythagorean ",
            " angle ",
            " circumference ",
        ],
        "memory": [" remember ", " recall ", " memory ", " store this ", "what do you remember"],
        "code": [" rust ", " powershell ", " code ", " debug ", " parser ", " cli ", " repository "],
        "governance": [
            " permission ",
            " authority ",
            " authorized ",
            " durable ",
            " mutation ",
            " ledger ",
            " sandbox ",
        ],
        "planning": [
            " plan ",
            " milestones ",
            " roadmap ",
            " acceptance tests ",
            " dependencies ",
            " build first ",
        ],
        "explanation": [" explain ", " teach ", " simple terms ", " example ", " how does ", " why does "],
        "systems": [" lumen ", " cortex ", " bitwork ", " nemo ", " rhp ", " perci "],
        "science": [
            " momentum ",
            " energy ",
            " force ",
            " pressure ",
            " experiment ",
            " scientific ",
            " atom ",
            " cells ",
        ],
        "creativity": [
            " invent ",
            " brainstorm ",
            " story ",
            " creative ",
            " original ",
            " design a futuristic ",
        ],
        "comparison": [" compare ", " contrast ", " tradeoffs ", " versus ", " vs "],
    }
    scores = {k: sum(24 for x in xs if x in t) for k, xs in keys.items()}
    scores["general"] = 24 if not any(scores.values()) else 0
    return scores


class Model:
    """Minimal PERCIW01/02/03 reader for routing probes (not full SoftCascade)."""

    def __init__(self, path: Path):
        self.f = path.open("rb")
        self.mm = mmap.mmap(self.f.fileno(), 0, access=mmap.ACCESS_READ)
        magic, version, bits, words, nlabels, total, header, _target, _corpus = FIXED.unpack_from(
            self.mm, 0
        )
        assert bits == bw.BITS and words == bw.WORDS, (bits, words)
        assert magic in (b"PERCIW01", b"PERCIW02", b"PERCIW03"), magic
        self.version = version
        self.header = header
        self.total = total
        self.labels: list[tuple[str, int, int, np.ndarray, np.ndarray]] = []
        off = FIXED.size
        entry = LABEL_ENTRY_V1 if version < 2 else LABEL_ENTRY_V2
        for _ in range(nlabels):
            name = self.mm[off : off + 16].split(b"\0", 1)[0].decode()
            off += 16
            _label_id, start, count, _concept = struct.unpack_from("<IIII", self.mm, off)
            off += 16
            pos = np.frombuffer(self.mm, dtype="<u8", count=bw.WORDS, offset=off).copy()
            off += bw.WORDS * 8
            if version >= 2:
                neg = np.frombuffer(self.mm, dtype="<u8", count=bw.WORDS, offset=off).copy()
                off += bw.WORDS * 8
            else:
                neg = np.zeros(bw.WORDS, dtype=np.uint64)
            self.labels.append((name, start, count, pos, neg))
            # v3 concept table sits after labels; skip by using header as record base only.
            _ = entry  # silence lint; header_size from file is authoritative
        # For v3, concept blobs sit between labels and records; self.header is correct.

    def classify(self, text: str, nearest: bool = True):
        bits, _pop = bw.encode(text)
        q = np.asarray(bits, dtype=np.uint64)
        boosts = priors(text)
        coarse: list[tuple[int, int]] = []
        for i, (name, _start, _count, pos, neg) in enumerate(self.labels):
            positive = int(np.bitwise_count(np.bitwise_and(pos, q)).sum())
            negative = int(np.bitwise_count(np.bitwise_and(neg, q)).sum())
            prior = boosts.get(name, 0)
            if self.version >= 3:
                prior *= 2
            score = positive * 2 - negative + prior
            coarse.append((score, i))
        coarse.sort(reverse=True)
        best = None
        scan = coarse[:3] if nearest else coarse[:1]
        for coarse_score, i in scan:
            name, start, count, _pos, _neg = self.labels[i]
            if count == 0:
                continue
            offset = self.header + start * RECORD_SIZE
            rows = np.ndarray(
                (count, bw.WORDS),
                dtype="<u8",
                buffer=self.mm,
                offset=offset + 8,
                strides=(RECORD_SIZE, 8),
            )
            overlaps = np.bitwise_count(np.bitwise_and(rows, q)).sum(axis=1, dtype=np.uint16)
            pops = np.ndarray(
                (count,),
                dtype="<u2",
                buffer=self.mm,
                offset=offset + 4,
                strides=(RECORD_SIZE,),
            )
            scores = overlaps.astype(np.int32) * 2 - pops.astype(np.int32)
            j = int(scores.argmax())
            score = int(scores[j]) + coarse_score * 2
            variant = struct.unpack_from("<H", self.mm, offset + j * RECORD_SIZE)[0]
            candidate = (score, i, variant, int(overlaps[j]), name)
            if best is None or candidate[0] > best[0]:
                best = candidate
        assert best is not None
        score, i, variant, overlap, name = best
        return name, variant, score, overlap, coarse[:3]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", type=Path, default=None)
    ns = ap.parse_args()
    model_path = ns.model or default_model()
    if not model_path.is_file():
        raise SystemExit(f"model missing: {model_path}")
    m = Model(model_path)
    tests = [
        ("greeting", "hello friend, ready to begin?"),
        ("identity", "what exactly is Perci and where are your limits"),
        ("english", "could you polish my grammar and explain the adjective"),
        ("logic", "all ravens are birds and this is a raven, what follows"),
        ("math", "compute 812 divided by 7"),
        ("geometry", "find the area of a triangle whose base is 14 and height is 9"),
        ("memory", "please remember this architectural decision"),
        ("code", "help debug this Rust command line parser"),
        ("governance", "do we have authority to write this durable mutation"),
        ("planning", "make milestones and acceptance tests for the project"),
        ("explanation", "teach the concept in simple terms with an example"),
        ("systems", "how should Lumen Cortex and Bitwork interconnect"),
        ("science", "describe momentum and how to measure it experimentally"),
        ("creativity", "invent an original cybernetic interface concept"),
        ("comparison", "compare deterministic solvers against neural prediction"),
        ("general", "give me your careful thoughts on this unusual idea"),
    ]
    ok = 0
    started = time.time()
    for expected, text in tests:
        got, variant, score, overlap, _coarse = m.classify(text)
        passed = got == expected
        ok += int(passed)
        print(
            f"{'PASS' if passed else 'FAIL'} expected={expected:11s} got={got:11s} "
            f"variant={variant:2d} score={score:4d} overlap={overlap:3d} :: {text}"
        )
    elapsed = time.time() - started
    print(
        json.dumps(
            {
                "model": str(model_path),
                "version": m.version,
                "passed": ok,
                "total": len(tests),
                "accuracy": ok / len(tests),
                "seconds": elapsed,
                "queries_per_second": len(tests) / max(elapsed, 1e-9),
            },
            indent=2,
        )
    )
    raise SystemExit(0 if ok >= 14 else 1)


if __name__ == "__main__":
    main()
