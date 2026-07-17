#!/usr/bin/env python3
"""Evaluate whether Bitwork v4 transfers reasoning prompts to relevant concepts."""
from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

from evaluate_v2 import Daemon, canonical, sha256

CASES = [
    ("geometry-curvature", "how can local bending determine an entire shape", "geometry", "Curvature"),
    ("geometry-topology", "what properties survive continuous deformation", "geometry", "Topology"),
    ("geometry-symmetry", "what can change while a spatial form stays equivalent", "geometry", "Symmetry"),
    ("language-metaphor", "how does a metaphor carry structure between subjects", "english", "metaphor"),
    ("language-ambiguity", "when does ambiguity become a practical defect", "english", "Ambiguity"),
    ("language-compression", "why can a shorter sentence preserve the important meaning", "english", "compression"),
    ("science-life", "how does an organism keep local order over time", "science", "Life"),
    ("science-death", "what ends when biological integration cannot maintain itself", "science", "death"),
    ("science-evolution", "how can selection accumulate fit without foresight", "science", "Evolution"),
    ("systems-emergence", "how can interactions create a pattern no component contains", "systems", "Emergent"),
    ("systems-feedback", "distinguish amplification from stabilizing correction", "systems", "feedback"),
    ("logic-counterexample", "what defeats a claim that says every case behaves this way", "logic", "counterexample"),
    ("logic-causality", "why does intervention reveal more than prediction alone", "logic", "intervention"),
    ("general-finitude", "why does limited time give choices weight", "general", "Death"),
    ("general-knowledge", "when does stored information become knowledge", "general", "Knowledge"),
    ("memory-learning", "how is preserving a note different from changing performance", "memory", "learning"),
    ("identity-strongest-claim", "what is the strongest honest claim about Perci intelligence", "identity", "strongest honest claim"),
    ("identity-weight-boundary", "how can a fresh process prove weights changed instead of session context", "identity", "weight-change"),
    ("logic-falsification", "what makes a claim falsifiable with a competing prediction", "logic", "disconfirmation"),
    ("logic-argument-structure", "separate observation premise inference conclusion and uncertainty", "logic", "unsupported step"),
    ("planning-acceptance", "what acceptance gate and receipt should justify promoting a change", "planning", "acceptance"),
    ("explanation-depth", "what makes deep reasoning a causal chain with a boundary and a test", "explanation", "causal chain"),
    ("explanation-response-fit", "how should a response fit the requested operation and uncertainty", "explanation", "requested operation"),
    ("systems-routing", "why should routing select an operator instead of a nearby concept", "systems", "routing"),
    ("systems-composition", "how can composition fail when a correct component is rendered in the wrong context", "systems", "composition"),
    ("comparison-ablation", "how does ablation show whether a component causally supports a capability", "comparison", "ablation"),
    ("comparison-regression", "what is a regression after a change and how should it constrain promotion", "comparison", "regression"),
]


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--perci-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--port", type=int, default=17874)
    args = parser.parse_args()

    daemon = Daemon(args.perci_bin, args.model, args.port)
    rows = []
    try:
        for case_id, prompt, expected_label, expected_term in CASES:
            result = daemon.request("classify", prompt)["result"]
            insight = result.get("insight") or ""
            label_ok = result.get("label") == expected_label
            concept_ok = expected_term.lower() in insight.lower()
            rows.append({
                "id": case_id,
                "prompt": prompt,
                "expected_label": expected_label,
                "actual_label": result.get("label"),
                "expected_concept_term": expected_term,
                "concept_id": result.get("concept_id"),
                "insight": insight,
                "label_pass": label_ok,
                "concept_pass": concept_ok,
                "pass": label_ok and concept_ok,
            })
    finally:
        daemon.close()

    passed = sum(row["pass"] for row in rows)
    receipt = {
        "schema": "perci.concept-transfer.v4",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "model_sha256": sha256(args.model),
        "runtime_sha256": sha256(args.perci_bin),
        "evaluator_sha256": sha256(Path(__file__).resolve()),
        "case_count": len(rows),
        "passed": passed,
        "accuracy": passed / len(rows),
        "minimum_accuracy": 0.75,
        "status": "PASS" if passed / len(rows) >= 0.75 else "HOLD",
        "automatic_promotion": False,
        "cases": rows,
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2, ensure_ascii=False))
    return 0 if receipt["status"] == "PASS" else 2


if __name__ == "__main__":
    raise SystemExit(main())
