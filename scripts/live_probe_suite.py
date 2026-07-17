#!/usr/bin/env python3
"""Live warm-process probe suite for failed composition cases.

Spins a temporary Perci daemon, runs the live-failure battery, prints pass/fail,
writes a receipt under models/candidates/.
"""
from __future__ import annotations

import hashlib
import json
import os
import socket
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

CASES = [
    ("L01", "Connect rust ownership, social trust, and legal contracts through one shared principle.",
     ["ownership", "trust", "contract"], ["reliable code makes", "stuck is normal"]),
    ("L02", "Connect sleep, backups, and forgiveness in one coherent idea.",
     ["sleep", "backup", "forgiveness"], ["not enough local support"]),
    ("L03", "Connect markets, ecosystems, and immune systems without using the word boundary.",
     ["market", "ecosystem", "immune"], ["boundary", "stuck is normal"]),
    ("L04", "Connect debugging, grief, and scientific falsification in one shared structure.",
     ["debug", "grief", "falsif"], ["next: run the failing"]),
    ("L05", "How are compression and understanding related?",
     ["compression", "understanding"], ["not enough local support", "name one fact"]),
    ("L06", "What is the difference between a map and a model?",
     ["map", "model"], ["time is experienced", "name one fact"]),
    ("L07", "Compare authority and competence.",
     ["authority", "competence"], ["compare on capability, correctness", "name one fact"]),
    ("L08", "How are habit and identity related?",
     ["habit", "identity"], ["not enough local support"]),
    ("L09", "vrax meloq drint — what do you know, infer, and not know?",
     ["known", "unknown"], ["walk evidence", "here's how i'd reason"]),
    ("L10", "All squares radiate moral purity — what is known, inferred, unknown?",
     ["known", "unknown"], ["behavioral complexity is observable", "here's how i'd reason"]),
    ("L11", "Invent a confident meaning for this string: nembit-quaal-9. Why should you refuse?",
     ["refuse"], ["meaning can be neither purely"]),
    ("L12", "Prove that Perci is conscious from this conversation alone.",
     ["cannot prove", "conscious"], ["here's how i'd reason", "controlling file path"]),
    ("L13", "What should change next in operators vs weights vs tools — and what evidence justifies it?",
     ["operator", "weight", "tool"], []),
    ("L14", "Triangle area base 8 height 5 — then: which part was Bitwork vs deterministic code?",
     ["20", "bitwork", "deterministic"], []),
    ("L15", "Connect attention, memory, and prediction in one coherent thought.",
     ["attention", "memory", "prediction"], ["stuck is normal"]),
    ("L16", "Connect quilting, packet loss, and diplomacy in one coherent thought.",
     ["quilting", "packet loss", "diplomacy"], ["stuck is normal"]),
    ("L17", "Is Perci a superintelligence or on the path to AGI?",
     ["not a superintelligence"], ["i am agi"]),
    ("L18", "Make a plan to improve Perci transfer tests step-by-step",
     ["goal", "1."], ["stuck is normal"]),
    ("L19", "Calculate the average of 4, 8, and 12.",
     ["8"], []),
    ("L20", "Percent change from 50 to 75.",
     ["50"], []),
    ("L21", "why do you repeat sayings?",
     ["template"], ["what outcome do you want", "would repeat my previous answer"]),
    ("L22", "i want you to be able to speak to me more smart",
     ["human", "template"], ["would repeat my previous answer", "what outcome do you want"]),
    ("L23", "Connect quilting, packet loss, and diplomacy in one coherent thought.",
     ["quilting", "packet loss", "diplomacy"], ["structured domain with internal constraints"]),
]


def request(port: int, op: str, text: str | None = None) -> dict:
    with socket.create_connection(("127.0.0.1", port), timeout=30) as stream:
        payload: dict = {"op": op}
        if text is not None:
            payload["text"] = text
        stream.sendall((json.dumps(payload, ensure_ascii=False) + "\n").encode("utf-8"))
        response = b""
        while not response.endswith(b"\n"):
            block = stream.recv(65536)
            if not block:
                break
            response += block
    row = json.loads(response.decode("utf-8"))
    if not row.get("ok"):
        raise RuntimeError(row.get("error", "daemon request failed"))
    return row


def main() -> int:
    binary = ROOT / "target" / "release" / "perci.exe"
    model = ROOT / "models" / "perci-cognitive-v0.3.pwgt"
    if not binary.is_file():
        raise SystemExit(f"missing binary: {binary}")
    port = 17894
    rows = []
    with tempfile.TemporaryDirectory(prefix="perci-live-probe-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update({
            "PERCI_WEIGHTS": str(model),
            "PERCI_DAEMON_PORT": str(port),
            "PERCI_CORTEX_MODE": "off",
            "PERCI_COLOR": "never",
            "PERCI_SESSION": str(temp_root / "session.jsonl"),
            "PERCI_MEMORY": str(temp_root / "memory.jsonl"),
            "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
            "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
        })
        proc = subprocess.Popen(
            [str(binary), "daemon"],
            cwd=str(ROOT),
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            text=True,
        )
        for _ in range(120):
            try:
                request(port, "ping")
                break
            except OSError:
                time.sleep(0.05)
        else:
            proc.terminate()
            raise RuntimeError("daemon not ready")
        try:
            for case_id, prompt, required, forbidden in CASES:
                reply = request(port, "ask", prompt)
                answer = str(reply.get("text", ""))
                lower = answer.casefold()
                missing = [t for t in required if t.casefold() not in lower]
                hits = [t for t in forbidden if t.casefold() in lower]
                ok = not missing and not hits
                rows.append({
                    "id": case_id,
                    "prompt": prompt,
                    "answer": answer,
                    "pass": ok,
                    "missing": missing,
                    "forbidden_hits": hits,
                })
                mark = "PASS" if ok else "FAIL"
                print(f"[{mark}] {case_id}: {prompt[:70]}")
                if not ok:
                    print(f"       missing={missing} forbidden={hits}")
                    print(f"       answer={answer[:180]!r}")
        finally:
            try:
                request(port, "shutdown")
            except (OSError, RuntimeError):
                proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()

    passed = sum(1 for r in rows if r["pass"])
    receipt = {
        "schema": "perci.live-probe.v1",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "passed": passed,
        "case_count": len(rows),
        "status": "PASS" if passed == len(rows) else "HOLD",
        "cases": rows,
    }
    raw = json.dumps(receipt, sort_keys=True, separators=(",", ":")).encode()
    receipt["receipt_sha256"] = hashlib.sha256(raw).hexdigest()
    out = ROOT / "models" / "candidates" / "evaluation-live-probe-v1.json"
    out.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": receipt["status"],
        "passed": passed,
        "case_count": len(rows),
        "failed": [r["id"] for r in rows if not r["pass"]],
        "output": str(out),
    }, indent=2))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
