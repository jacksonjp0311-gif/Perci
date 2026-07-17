#!/usr/bin/env python3
"""Evidence-bounded Perci v2 candidate evaluation.

This script never promotes a model. It verifies the candidate and sealed dataset,
runs a warm daemon, compares the associative classifier with simple controls,
measures selective-local risk, and writes a hash-bearing evaluation receipt.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import socket
import subprocess
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

LABELS = [
    "greeting", "identity", "english", "logic", "math", "geometry", "memory",
    "code", "governance", "planning", "explanation", "systems", "science",
    "creativity", "comparison", "general",
]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def canonical(payload: object) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def exact_shape(prompt: str) -> bool:
    text = prompt.lower()
    if any(word in text for word in ("integral", "derivative", "explain", "operator", "parser")):
        return False
    numbers = re.findall(r"-?\d+", text)
    arithmetic = len(numbers) >= 2 and any(
        marker in text
        for marker in (" percent of ", " plus ", " minus ", " times ", " multiplied by ",
                       " divided by ", " + ", " - ", " * ", " / ")
    )
    geometry = (
        len(numbers) >= 2
        and (("triangle" in text and "area" in text)
             or ("rectangle" in text and ("area" in text or "perimeter" in text))
             or "pythag" in text)
    ) or (
        len(numbers) >= 1
        and (("square" in text and ("area" in text or "perimeter" in text))
             or ("circle" in text and ("area" in text or "circumference" in text)
                 and ("radius" in text or "diameter" in text)))
    )
    return arithmetic or geometry


def local_shape(prompt: str, label: str) -> bool:
    text = prompt.strip().lower()
    if label == "greeting":
        # A one-token reflex greeting is local; multi-word social conversation
        # belongs to the dialogue layer and should not count as exact/local
        # capability when measuring associative routing precision.
        return text in {"hi", "hello", "hey", "hello perci"}
    if label == "identity":
        normalized = text.strip(".,!?;:")
        return normalized in {
            "who are you", "what are you", "what can you do",
            "tell me about perci", "what is perci",
        } or any(
            marker in text for marker in (
                "your capabilities", "your limitations", "your limits",
            )
        )
    return False


def v2_confident(row: dict) -> bool:
    return (
        row.get("schema") == "perci.classify.v2"
        and row.get("score", 0) >= 40
        and row.get("overlap", 0) >= 8
        and row.get("margin", 0) >= 12
        and row.get("overlap_z", 0.0) >= 4.0
        and row.get("jaccard", 0.0) >= 0.10
        and 0 < row.get("query_popcount", 0) <= 2048
    )


def policy_local(prompt: str, row: dict) -> bool:
    return exact_shape(prompt) or (v2_confident(row) and local_shape(prompt, row.get("label", "")))


def keyword_baseline(prompt: str) -> str:
    text = " " + prompt.lower() + " "
    groups = [
        ("code", (" rust ", " compiler ", " compile", " parser ", " code ", " api ", " fuzz ")),
        ("memory", (" remember ", " recall ", " saved notes", " forget ")),
        ("governance", (" authority", " authorization", " permission", " mutation", " rollback", " evidence gate")),
        ("geometry", (" triangle", " circle", " polygon", " geometry", " pythag")),
        ("math", (" calculate", " compute", " fraction", " equation", " integral", " percent")),
        ("science", (" hypothesis", " falsif", " measurement", " observation")),
        ("planning", (" milestone", " dependencies", " acceptance criteria", " sequence")),
        ("comparison", (" compare", " contrast", " both approaches", " tradeoff")),
        ("creativity", (" invent", " unusual", " concept", " brainstorm")),
        ("systems", (" lumen", " cortex", " provider mesh", " cpu", " cache size")),
        ("english", (" grammar", " grammatical", " revise", " paragraph")),
        ("logic", (" premises", " assumptions", " conclusion follows")),
        ("explanation", (" teach ", " explain ", " example", " edge case")),
        ("identity", ("who are you", "what can you do", "perci can", "your limitations")),
        ("greeting", (" hello ", " hi ", " hey ", " morning perci")),
    ]
    scores = [(sum(marker in text for marker in markers), label) for label, markers in groups]
    score, label = max(scores)
    return label if score else "general"


class Daemon:
    def __init__(self, binary: Path, model: Path, port: int):
        env = os.environ.copy()
        env["PERCI_WEIGHTS"] = str(model.resolve())
        env["PERCI_DAEMON_PORT"] = str(port)
        env["PERCI_CORTEX"] = "0"
        flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
        self.port = port
        self.proc = subprocess.Popen(
            [str(binary.resolve()), "daemon"], cwd=str(binary.resolve().parents[2]),
            env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            creationflags=flags,
        )
        for _ in range(100):
            try:
                self.request("ping")
                break
            except OSError:
                time.sleep(0.05)
        else:
            raise RuntimeError("Perci daemon did not become ready")

    def request(self, op: str, text: str | None = None) -> dict:
        with socket.create_connection(("127.0.0.1", self.port), timeout=30) as stream:
            payload = {"op": op}
            if text is not None:
                payload["text"] = text
            stream.sendall((json.dumps(payload) + "\n").encode())
            response = b""
            while not response.endswith(b"\n"):
                block = stream.recv(65536)
                if not block:
                    break
                response += block
        row = json.loads(response)
        if not row.get("ok"):
            raise RuntimeError(row.get("error", "daemon request failed"))
        return row

    def close(self) -> None:
        try:
            self.request("shutdown")
        except Exception:
            self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", type=Path, default=root / "models/candidates/perci-cognitive-v0.2.pwgt")
    parser.add_argument("--dataset", type=Path, default=root / "training/heldout-v2.jsonl")
    parser.add_argument("--perci-bin", type=Path, default=root / "target/release/perci.exe")
    parser.add_argument("--output", type=Path, default=root / "models/candidates/evaluation-v2.json")
    parser.add_argument("--port", type=int, default=17867)
    parser.add_argument(
        "--policy-source",
        type=Path,
        help="source file implementing the selective-local policy; auto-detected when omitted",
    )
    args = parser.parse_args()

    lumen_bridge = root.parent / "src/perci_bridge.rs"
    if args.policy_source is not None:
        policy_source = args.policy_source.resolve()
        policy_kind = "explicit"
    elif lumen_bridge.is_file():
        policy_source = lumen_bridge
        policy_kind = "lumen-bridge-v2"
    else:
        # Standalone Perci has no Lumen bridge. Its bounded local-decision policy
        # is the exact policy_local/v2_confident implementation in this evaluator.
        policy_source = Path(__file__).resolve()
        policy_kind = "evaluator-embedded-v2"

    manifest_path = args.model.with_suffix(args.model.suffix + ".json")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    model_hash = sha256(args.model)
    integrity = model_hash == manifest.get("sha256") and manifest.get("format") in {"PERCIW02", "PERCIW03"}
    cases = [json.loads(line) for line in args.dataset.read_text(encoding="utf-8").splitlines() if line.strip()]

    daemon = Daemon(args.perci_bin, args.model, args.port)
    results = []
    try:
        for case in cases:
            started = time.perf_counter()
            classified = daemon.request("classify", case["prompt"])["result"]
            latency_ms = (time.perf_counter() - started) * 1000.0
            local = policy_local(case["prompt"], classified)
            results.append({
                "id": case["id"], "suite": case["suite"], "expected_domain": case["expected_domain"],
                "actual_domain": classified.get("label"), "expected_local": case["allow_local"],
                "actual_local": local, "latency_ms": round(latency_ms, 3),
                "margin": classified.get("margin"), "overlap_z": classified.get("overlap_z"),
                "jaccard": classified.get("jaccard"),
            })
    finally:
        daemon.close()

    domain_rows = [r for r in results if r["suite"] == "domain"]
    domain_accuracy = sum(r["actual_domain"] == r["expected_domain"] for r in domain_rows) / len(domain_rows)
    baseline_accuracy = sum(keyword_baseline(c["prompt"]) == c["expected_domain"] for c in cases if c["suite"] == "domain") / len(domain_rows)
    local_rows = [r for r in results if r["actual_local"]]
    false_local = [r for r in local_rows if not r["expected_local"]]
    expected_local_rows = [r for r in results if r["expected_local"]]
    local_recall = sum(r["actual_local"] for r in expected_local_rows) / len(expected_local_rows)
    local_precision = 1.0 - len(false_local) / max(len(local_rows), 1)
    trap_rows = [r for r in results if r["suite"] in {"trap", "ood"}]
    trap_abstention = sum(not r["actual_local"] for r in trap_rows) / len(trap_rows)
    latencies = sorted(r["latency_ms"] for r in results)
    shifted_accuracy = sum(
        LABELS[(LABELS.index(r["actual_domain"]) + 1) % len(LABELS)] == r["expected_domain"]
        for r in domain_rows
    ) / len(domain_rows)

    gates = {
        "model_integrity": integrity,
        "domain_accuracy": domain_accuracy >= 0.85,
        "local_precision": local_precision >= 0.98,
        "local_recall": local_recall >= 0.80,
        "trap_abstention": trap_abstention >= 0.98,
        "negative_control_separation": domain_accuracy >= shifted_accuracy + 0.25,
    }
    operational_candidate = all(gates.values())
    baseline_advantage = domain_accuracy - baseline_accuracy
    # A single bounded holdout can qualify an operational candidate. It cannot
    # establish explanatory geometry without independent replication and a
    # confidence-bound superiority analysis.
    claim = "OPERATIONAL_CANDIDATE" if operational_candidate else "DOWNGRADED"
    receipt = {
        "schema": "perci.evaluation.v2",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "model_sha256": model_hash,
        "runtime_sha256": sha256(args.perci_bin),
        "policy_source": str(policy_source),
        "policy_source_kind": policy_kind,
        "policy_source_sha256": sha256(policy_source),
        "evaluator_sha256": sha256(Path(__file__).resolve()),
        "manifest_sha256": sha256(manifest_path),
        "dataset_sha256": sha256(args.dataset),
        "case_count": len(cases),
        "metrics": {
            "domain_accuracy": domain_accuracy,
            "keyword_baseline_accuracy": baseline_accuracy,
            "baseline_advantage": baseline_advantage,
            "shifted_label_control_accuracy": shifted_accuracy,
            "local_precision": local_precision,
            "local_recall": local_recall,
            "trap_abstention": trap_abstention,
            "local_coverage": len(local_rows) / len(results),
            "false_local_count": len(false_local),
            "latency_p50_ms": latencies[len(latencies) // 2],
            "latency_p95_ms": latencies[min(len(latencies) - 1, int(len(latencies) * 0.95))],
        },
        "gates": gates,
        "status": claim,
        "baseline_supremacy_point_estimate": baseline_advantage >= 0.02,
        "confidence_bound_required": True,
        "independent_replication_required": True,
        "automatic_promotion": False,
        "phenomenology_proven": False,
        "failures": [r for r in results if r["actual_local"] != r["expected_local"] or (r["suite"] == "domain" and r["actual_domain"] != r["expected_domain"])],
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt).encode()).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2, ensure_ascii=False))
    return 0 if operational_candidate else 2


if __name__ == "__main__":
    raise SystemExit(main())
