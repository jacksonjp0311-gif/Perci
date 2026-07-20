#!/usr/bin/env python3
"""Evaluate warm-session cross-domain analysis and evidence transfer."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import socket
import subprocess
import tempfile
import time
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def request(port: int, op: str, text: str | None = None) -> dict[str, object]:
    with socket.create_connection(("127.0.0.1", port), timeout=30) as stream:
        payload: dict[str, object] = {"op": op}
        if text is not None:
            payload["text"] = text
        stream.sendall((json.dumps(payload, ensure_ascii=False) + "\n").encode("utf-8"))
        response = b""
        while not response.endswith(b"\n"):
            block = stream.recv(65536)
            if not block:
                break
            response += block
    value = json.loads(response.decode("utf-8"))
    if not value.get("ok"):
        raise RuntimeError(value.get("error", "daemon request failed"))
    return value


def load_rows(path: Path) -> list[dict[str, object]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not rows:
        raise ValueError(f"no rows in {path}")
    return rows


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--perci-bin", type=Path, default=root / "target/release/perci.exe")
    parser.add_argument(
        "--questions",
        type=Path,
        default=root / "training/dialogue-cross-domain-v1-heldout.jsonl",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=root / "models/candidates/evaluation-cross-domain-v1.json",
    )
    parser.add_argument("--port", type=int, default=17876)
    args = parser.parse_args()
    binary = args.perci_bin.resolve()
    questions = args.questions.resolve()
    rows = load_rows(questions)
    cases: list[dict[str, object]] = []
    process: subprocess.Popen[str] | None = None

    with tempfile.TemporaryDirectory(prefix="perci-cross-domain-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update(
            {
                "PERCI_WEIGHTS": str(root / "models/perci-cognitive-v0.3.pwgt"),
                "PERCI_DAEMON_PORT": str(args.port),
                "PERCI_CORTEX_MODE": "off",
                "PERCI_COLOR": "never",
                "PERCI_SESSION": str(temp_root / "session.jsonl"),
                "PERCI_MEMORY": str(temp_root / "memory.jsonl"),
                "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
                "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
            }
        )
        process = subprocess.Popen(
            [str(binary), "daemon"],
            cwd=root,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            text=True,
        )
        for _ in range(120):
            try:
                request(args.port, "ping")
                break
            except OSError:
                time.sleep(0.05)
        else:
            process.terminate()
            raise RuntimeError("Perci daemon did not become ready")

        try:
            for row in rows:
                prompt = str(row.get("prompt", ""))
                started = time.perf_counter()
                reply = request(args.port, "ask", prompt)
                latency_ms = (time.perf_counter() - started) * 1000.0
                answer = str(reply.get("text", ""))
                lower = answer.casefold()
                required = [str(value) for value in row.get("required", [])]
                forbidden = [str(value) for value in row.get("forbidden", [])]
                required_pass = all(value.casefold() in lower for value in required)
                forbidden_pass = not any(value.casefold() in lower for value in forbidden)
                cases.append(
                    {
                        "id": row.get("id"),
                        "family": row.get("family", "unknown"),
                        "prompt": prompt,
                        "answer": answer,
                        "required": required,
                        "forbidden": forbidden,
                        "required_pass": required_pass,
                        "forbidden_pass": forbidden_pass,
                        "latency_ms": round(latency_ms, 3),
                        "pass": required_pass and forbidden_pass and bool(answer.strip()),
                    }
                )
        finally:
            try:
                request(args.port, "shutdown")
            except (OSError, RuntimeError):
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()

    family_totals: dict[str, dict[str, int]] = defaultdict(lambda: {"passed": 0, "total": 0})
    for case in cases:
        family = str(case["family"])
        family_totals[family]["total"] += 1
        family_totals[family]["passed"] += int(bool(case["pass"]))
    passed = sum(int(bool(case["pass"])) for case in cases)
    receipt: dict[str, object] = {
        "schema": "perci.cross-domain-evaluation.v1",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "runtime_sha256": sha256(binary),
        "questions_sha256": sha256(questions),
        "case_count": len(cases),
        "passed": passed,
        "status": "PASS" if passed == len(cases) else "HOLD",
        "automatic_promotion": False,
        "topology": "single_node_warm_session",
        "families": dict(family_totals),
        "cases": cases,
    }
    canonical = json.dumps(receipt, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    receipt["receipt_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "status": receipt["status"],
                "passed": passed,
                "case_count": len(cases),
                "failed": [case["id"] for case in cases if not case["pass"]],
                "receipt_sha256": receipt["receipt_sha256"],
            },
            indent=2,
        )
    )
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
