#!/usr/bin/env python3
"""Bitwork vs. standard text-classification baselines on Perci's held-out routing cases.

Reproduces the evaluation table in docs/BITWORK_WHITEPAPER.md (section 10).

What it does
  1. Regenerates the exact training corpus of the v0.3 pack
     (models/perci-cognitive-v0.3.pwgt) with scripts/build_weights_v4.py's own
     prompt generator and checks its SHA-256 against the pack manifest
     (`corpus_sha256`). The baselines are trained on exactly the text Bitwork
     was built from: 960,000 generated prompts, 450,526 unique.
  2. Loads the 158 held-out cases that `perci packs calibrate` uses
     (training/heldout-v2.jsonl, heldout-v2.1.jsonl, heldout-v3-transfer.jsonl;
     their concatenated SHA-256 is the `cases_sha256` pinned in
     config/route-calibration/*.json).
  3. Bitwork: asks a real Perci binary over MCP stdio (`perci mcp`, tool
     `route`) for the v0.2 and v0.3 pack labels and records the tool's own
     in-process time (`receipt.elapsed_us`) plus the client round trip.
  4. Baselines, trained on the regenerated corpus (deduplicated text):
       keyword       the hand-written keyword router in scripts/evaluate_v2.py
       tfidf+lr      TF-IDF (word 1-2 grams + char 3-5 grams), logistic regression
       tfidf-1nn     cosine nearest neighbour over the same TF-IDF vectors
       bm25-1nn      Okapi BM25 (k1=1.2, b=0.75) nearest training prompt
  5. Writes a JSON report (and prints a Markdown table).

Usage (from the repository root, after `git lfs pull` and `cargo build --release`):

    python3 -m pip install scikit-learn numpy scipy
    python3 evals/bitwork_baseline/bitwork_baseline.py \
        --perci target/release/perci --out evals/bitwork_baseline/results.json

Nothing is sent over the network. Accuracy is top-1 label accuracy against
`expected_domain` on all 158 cases, the same quantity `perci packs calibrate`
reports. These are small, author-written sets; see the paper's caveats.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import random
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

HELDOUT = ["training/heldout-v2.1.jsonl", "training/heldout-v2.jsonl", "training/heldout-v3-transfer.jsonl"]


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return float("nan")
    return xs[max(0, min(len(xs) - 1, int(round(p / 100.0 * (len(xs) - 1)))))]


def regenerate_corpus():
    import build_weights as bw  # noqa: E402  (repo script)
    import build_weights_v4 as v4  # noqa: E402

    digest = hashlib.sha256()
    texts, labels = [], []
    seen = set()
    for label_id, label in enumerate(bw.LABELS):
        rng = random.Random(v4.SEED ^ (label_id * 0x9E3779B97F4A7C15))
        for index in range(v4.ATTEMPTS_PER_LABEL):
            prompt = v4.prompt_for(label, rng, index)[0]
            digest.update(label.encode("ascii") + b"\0" + prompt.encode("utf-8") + b"\n")
            key = (label, prompt)
            if key not in seen:
                seen.add(key)
                texts.append(prompt)
                labels.append(label)
    manifest = json.loads((ROOT / "models/perci-cognitive-v0.3.pwgt.json").read_text(encoding="utf-8"))
    ok = digest.hexdigest() == manifest["corpus_sha256"]
    return texts, labels, digest.hexdigest(), ok, v4.ATTEMPTS_PER_LABEL * len(bw.LABELS)


def load_cases():
    raw = b""
    cases = []
    for rel in HELDOUT:
        data = (ROOT / rel).read_bytes()
        raw += data
        for line in data.decode("utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                cases.append({"text": row["prompt"], "label": row["expected_domain"],
                              "suite": row.get("suite", "?"), "file": rel})
    return cases, hashlib.sha256(raw).hexdigest()


class Mcp:
    def __init__(self, perci, env):
        self.p = subprocess.Popen([perci, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.i = 0

    def rpc(self, method, params=None):
        self.i += 1
        msg = {"jsonrpc": "2.0", "id": self.i, "method": method}
        if params is not None:
            msg["params"] = params
        self.p.stdin.write((json.dumps(msg) + "\n").encode())
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def init(self):
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "bitwork-baseline", "version": "1"}})
        self.p.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.p.stdin.flush()
        return self

    def call(self, name, args):
        return self.rpc("tools/call", {"name": name, "arguments": args})["result"]["structuredContent"]

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=30)


def nopriors_profile(perci):
    """A temporary profile whose packs reuse the built-in v0.2/v0.3 weight files
    under new names. Non-built-in packs default to `lexical_priors: off`, so this
    measures the binary geometry alone (ablation of the keyword priors)."""
    import shutil

    profile = tempfile.mkdtemp(prefix="perci-bw-nopriors-")
    env = dict(os.environ, PERCI_PROFILE=profile, PERCI_LEDGER="off")
    for name, src in (("v02-nopriors", "models/perci-cognitive-v0.2.pwgt"),
                      ("v03-nopriors", "models/perci-cognitive-v0.3.pwgt")):
        d = Path(profile) / "packs" / name
        d.mkdir(parents=True)
        try:
            os.link(ROOT / src, d / "router.pwgt")
        except OSError:
            shutil.copyfile(ROOT / src, d / "router.pwgt")
        subprocess.run([perci, "packs", "manifest", str(d), "--name", name, "--version", "1",
                        "--entry", "router.pwgt"], env=env, check=True, stdout=subprocess.DEVNULL)
    return profile


def run_bitwork(perci, pack, cases, profile=None):
    profile = profile or tempfile.mkdtemp(prefix="perci-bw-eval-")
    env = dict(os.environ, PERCI_PROFILE=profile, PERCI_LEDGER="off")
    m = Mcp(perci, env).init()
    # Warm-up: the first call loads and verifies the pack (not counted).
    m.call("route", {"text": "warm up the routing pack", "pack": pack})
    preds, work_us, rtt_ms, margins, decisions = [], [], [], [], []
    for c in cases:
        t = time.perf_counter()
        sc = m.call("route", {"text": c["text"], "pack": pack})
        rtt_ms.append((time.perf_counter() - t) * 1000)
        if sc["status"] == "error":
            raise SystemExit(f"route error for pack {pack}: {sc.get('reasons')}")
        bwk = sc["result"]["bitwork"]
        preds.append(bwk["label"])
        margins.append(bwk["margin"])
        decisions.append(bwk["decision"])
        work_us.append(sc["receipt"]["elapsed_us"])
    m.close()
    # Determinism: a second, fresh process must give identical labels.
    m = Mcp(perci, env).init()
    again = [m.call("route", {"text": c["text"], "pack": pack})["result"]["bitwork"]["label"] for c in cases]
    m.close()
    return {"pred": preds, "work_us": work_us, "rtt_ms": rtt_ms, "margin": margins,
            "decision": decisions, "replay_identical": again == preds}


def bm25_matrix(train_texts):
    import numpy as np
    from sklearn.feature_extraction.text import CountVectorizer

    vec = CountVectorizer(lowercase=True, token_pattern=r"(?u)\b\w+\b")
    tf = vec.fit_transform(train_texts).tocsc().astype(np.float32)
    n_docs = tf.shape[0]
    df = np.diff(tf.indptr)
    idf = np.log(1.0 + (n_docs - df + 0.5) / (df + 0.5)).astype(np.float32)
    doc_len = np.asarray(tf.sum(axis=1)).ravel()
    avg = doc_len.mean()
    k1, b = 1.2, 0.75
    tf = tf.tocsr()
    rows = np.repeat(np.arange(n_docs), np.diff(tf.indptr))
    denom = tf.data + k1 * (1 - b + b * doc_len[rows] / avg)
    tf.data = (tf.data * (k1 + 1) / denom) * idf[tf.indices]
    return vec, tf


def prior_overlap(cases, train_texts):
    """How much the built-in keyword-prior table (src/cognitive.rs, fn lexical_priors)
    overlaps the held-out prompts: a contamination check, not a score."""
    import re
    import build_weights as bw

    src = (ROOT / "src/cognitive.rs").read_text(encoding="utf-8")
    start = src.index("fn lexical_priors(text: &str)")
    body = src[start:src.index("let mut scores", start)]
    groups = re.findall(r'\(\s*"(\w+)",\s*&\[(.*?)\]\s*,?\s*\)', body, re.S)
    cues = {g: re.findall(r'"((?:[^"\\]|\\.)*)"', b) for g, b in groups}

    def pad(t):
        return " " + bw.normalize(t) + " "

    held = [(pad(c["text"]), c["label"]) for c in cases]
    train = list({pad(t) for t in train_texts})
    multi = [(g, c) for g, cs in cues.items() for c in cs if len(c.split()) >= 2]
    in_held = [(g, c) for g, c in multi if any(c in p for p, _ in held)]
    held_only = [(g, c) for g, c in in_held if not any(c in t for t in train)]
    return {
        "cue_phrases": sum(len(v) for v in cues.values()),
        "labels_with_cues": len(cues),
        "heldout_with_cue_for_gold_label": sum(any(c in p for c in cues.get(g, [])) for p, g in held),
        "multiword_cues": len(multi),
        "multiword_cues_in_heldout": len(in_held),
        "multiword_cues_in_heldout_never_in_training": len(held_only),
        "examples": [f"{g}: {c.strip()}" for g, c in held_only[:12]],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--perci", default=str(ROOT / "target/release/perci"))
    ap.add_argument("--out", default=str(ROOT / "evals/bitwork_baseline/results.json"))
    ap.add_argument("--packs", default="v0.2,v0.3")
    args = ap.parse_args()

    import numpy as np
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.linear_model import LogisticRegression
    from scipy.sparse import hstack
    import evaluate_v2  # repo script: keyword_baseline

    texts, labels, corpus_sha, corpus_ok, generated = regenerate_corpus()
    print(f"corpus: {generated:,} generated, {len(texts):,} unique; sha256 {corpus_sha} "
          f"({'matches' if corpus_ok else 'DOES NOT match'} the v0.3 manifest)", flush=True)
    cases, cases_sha = load_cases()
    gold = [c["label"] for c in cases]
    print(f"held-out: {len(cases)} cases, sha256 {cases_sha}", flush=True)

    results = {}

    def record(name, pred, per_query_us=None, extra=None):
        correct = [p == g for p, g in zip(pred, gold)]
        by_file = {}
        for c, ok in zip(cases, correct):
            f = by_file.setdefault(c["file"], [0, 0])
            f[0] += ok
            f[1] += 1
        by_suite = {}
        for c, ok in zip(cases, correct):
            s = by_suite.setdefault(c["suite"], [0, 0])
            s[0] += ok
            s[1] += 1
        r = {"correct": sum(correct), "n": len(gold), "accuracy": sum(correct) / len(gold),
             "by_file": by_file, "by_suite": by_suite, "pred": pred}
        if per_query_us is not None:
            r["latency_us_p50"] = pct(per_query_us, 50)
            r["latency_us_p95"] = pct(per_query_us, 95)
        if extra:
            r.update(extra)
        results[name] = r
        print(f"{name:14} {r['correct']}/{r['n']} = {r['accuracy']:.3f}"
              + (f"  p50 {r['latency_us_p50']:.0f} us" if per_query_us is not None else ""), flush=True)

    # Bitwork through the real binary.
    for pack in [p for p in args.packs.split(",") if p]:
        b = run_bitwork(args.perci, pack, cases)
        record(f"bitwork-{pack}", b["pred"], b["work_us"],
               {"route_rtt_ms_p50": pct(b["rtt_ms"], 50), "replay_identical": b["replay_identical"],
                "margin": b["margin"], "decision": b["decision"],
                "accepted_at_default_gate": sum(d == "accept" for d in b["decision"]),
                "accepted_correct": sum(d == "accept" and p == g for d, p, g in zip(b["decision"], b["pred"], gold))})

    # Ablation: same weight files, keyword priors off (geometry only).
    prof = nopriors_profile(args.perci)
    for pack in ("v02-nopriors", "v03-nopriors"):
        b = run_bitwork(args.perci, pack, cases, prof)
        record(f"bitwork-{pack}", b["pred"], b["work_us"],
               {"route_rtt_ms_p50": pct(b["rtt_ms"], 50), "replay_identical": b["replay_identical"]})

    # Keyword baseline (scripts/evaluate_v2.py).
    t = []
    pred = []
    for c in cases:
        s = time.perf_counter()
        pred.append(evaluate_v2.keyword_baseline(c["text"]))
        t.append((time.perf_counter() - s) * 1e6)
    record("keyword", pred, t)

    # TF-IDF features.
    s = time.perf_counter()
    wv = TfidfVectorizer(ngram_range=(1, 2), min_df=2, sublinear_tf=True, token_pattern=r"(?u)\b\w+\b")
    cv = TfidfVectorizer(analyzer="char_wb", ngram_range=(3, 5), min_df=2, sublinear_tf=True, max_features=200_000)
    Xw = wv.fit_transform(texts)
    Xc = cv.fit_transform(texts)
    X = hstack([Xw, Xc]).tocsr()
    fit_vec_s = time.perf_counter() - s

    def feats(q):
        return hstack([wv.transform(q), cv.transform(q)]).tocsr()

    s = time.perf_counter()
    lr = LogisticRegression(max_iter=200, C=4.0, class_weight="balanced", solver="saga", tol=1e-3)
    lr.fit(X, labels)
    fit_lr_s = time.perf_counter() - s
    t, pred = [], []
    for c in cases:
        s = time.perf_counter()
        pred.append(lr.predict(feats([c["text"]]))[0])
        t.append((time.perf_counter() - s) * 1e6)
    record("tfidf+lr", pred, t, {"train_seconds": round(fit_vec_s + fit_lr_s, 1)})

    # TF-IDF cosine 1-NN (rows are L2-normalised per block; the sum is a fair cosine proxy).
    lab = np.array(labels)
    t, pred = [], []
    XT = X.T.tocsr()
    for c in cases:
        s = time.perf_counter()
        sims = (feats([c["text"]]) @ XT).toarray().ravel()
        pred.append(lab[int(sims.argmax())])
        t.append((time.perf_counter() - s) * 1e6)
    record("tfidf-1nn", pred, t)

    # BM25 1-NN.
    bvec, bmat = bm25_matrix(texts)
    BT = bmat.T.tocsr()
    t, pred = [], []
    for c in cases:
        s = time.perf_counter()
        q = bvec.transform([c["text"]])
        q.data[:] = 1.0
        scores = (q @ BT).toarray().ravel()
        pred.append(lab[int(scores.argmax())])
        t.append((time.perf_counter() - s) * 1e6)
    record("bm25-1nn", pred, t)

    overlap = prior_overlap(cases, texts)
    print(f"prior/held-out overlap: {overlap['heldout_with_cue_for_gold_label']}/{len(cases)} held-out prompts "
          f"contain a cue for their gold label; {overlap['multiword_cues_in_heldout_never_in_training']} multi-word "
          f"cues appear in held-out prompts but never in the training corpus", flush=True)

    report = {
        "prior_overlap": overlap,
        "schema": "perci.bitwork-baseline.v1",
        "corpus": {"generated": generated, "unique": len(texts), "sha256": corpus_sha,
                   "matches_v0.3_manifest": corpus_ok},
        "heldout": {"files": HELDOUT, "cases": len(cases), "sha256": cases_sha},
        "machine": {"platform": platform.platform(), "python": platform.python_version(),
                    "cpus": os.cpu_count()},
        "perci": subprocess.run([args.perci, "--version"], capture_output=True, text=True).stdout.strip(),
        "gold": gold,
        "results": results,
    }
    Path(args.out).write_text(json.dumps(report, indent=1) + "\n", encoding="utf-8")
    print("\n| method | correct | accuracy | p50 per query |\n|---|---:|---:|---:|")
    for name, r in results.items():
        print(f"| {name} | {r['correct']}/{r['n']} | {r['accuracy']:.3f} | {r.get('latency_us_p50', float('nan')):.0f} us |")
    print(f"\nwrote {args.out}")


if __name__ == "__main__":
    main()
