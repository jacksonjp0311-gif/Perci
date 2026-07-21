#!/usr/bin/env python3
"""Multi-turn live probe for continuity (BRPC D) and recovery (BRPC K).

Spins a temporary Perci daemon, runs setup→probe→optional re-ask threads,
writes models/candidates/multi-turn-probe-latest.json.

Never promotes weights.
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
OUT = ROOT / "models" / "candidates" / "multi-turn-probe-latest.json"

# Threads: setup turns, then probe with required/forbidden, optional re-ask.
THREADS: list[dict] = [
    {
        "id": "MT01",
        "setup": ["Why does trust collapse when communication is delayed?"],
        "prompt": "what about timeout and retry though?",
        "want_any": [["timeout", "retry", "idempot", "done", "checkable", "trust", "lag"]],
        "forbid": ["structure under constraint for", "for your point about", "i am conscious"],
    },
    {
        "id": "MT02",
        "setup": ["Explain how boundaries enable repair"],
        "prompt": "same idea but for local order in living systems",
        "want_any": [["order", "life", "boundar", "exchange", "repair", "mainten", "energy"]],
        "forbid": ["structure under constraint for", "i am conscious"],
    },
    {
        "id": "MT03",
        "setup": ["How should interfaces earn trust under lag and retry?"],
        "prompt": "say that again but shorter and without the list",
        "want_any": [["trust", "timeout", "retry", "lag", "idempot", "done", "checkable"]],
        "forbid": ["1.", "2.", "3.", "governance authority"],
    },
    {
        "id": "MT04",
        "setup": ["How are memory and identity related?"],
        "prompt": "I dont agree — which premise should we challenge first?",
        "want_any": [["premise", "claim", "challenge", "memory", "identity", "disagree"]],
        "forbid": ["stuck is normal", "i am conscious"],
    },
    {
        "id": "MT05",
        "setup": [
            "Why does trust collapse when communication is delayed?",
            "what about timeout and retry though?",
        ],
        "prompt": "what should i do next about that",
        "want_any": [
            ["timeout", "retry", "idempot", "contract", "checkable", "trust", "operator", "test"]
        ],
        "forbid": ["for your point about", "i am conscious"],
    },
    {
        "id": "MT06",
        "setup": ["why does life maintain local order"],
        "prompt": "where does that analogy stop transferring?",
        "want_any": [["crystal", "shape", "geometry", "mechanism", "dies", "stop", "boundar", "life"]],
        "forbid": ["i am conscious", "structure under constraint for"],
    },
    {
        "id": "MT07",
        "setup": ["working on improving your system"],
        "prompt": "what should i do next about that",
        "want_any": [["improv", "transfer", "operator", "miss", "patch", "retest", "ticket", "hardness"]],
        "forbid": ["keeping ", "i am conscious"],
    },
    {
        "id": "MT08",
        "setup": ["Give an original comparison between entropy and limits; state the limit of the comparison."],
        "prompt": "sounds cryptic — give one plain sentence about boundary bands vs max coherence",
        "want_any": [["band", "coheren", "boundar", "max", "calibrat", "fail", "transfer"]],
        "forbid": ["composition failure", "concept card got promoted", "i am conscious"],
    },
]


def request(port: int, op: str, text: str | None = None) -> dict:
    with socket.create_connection(("127.0.0.1", port), timeout=45) as stream:
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


def score(answer: str, want_any: list[list[str]], forbid: list[str]) -> tuple[bool, list[str]]:
    low = answer.lower()
    reasons: list[str] = []
    for group in want_any:
        if not any(tok in low for tok in group):
            reasons.append(f"missing_any:{group}")
    for bad in forbid:
        if bad.lower() in low:
            reasons.append(f"forbidden:{bad}")
    return (len(reasons) == 0, reasons)


def main() -> int:
    binary = ROOT / "target" / "release" / "perci.exe"
    model = ROOT / "models" / "perci-cognitive-v0.3.pwgt"
    if not binary.is_file():
        raise SystemExit(f"missing binary: {binary}")
    port = 17911
    rows: list[dict] = []
    with tempfile.TemporaryDirectory(prefix="perci-mt-probe-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update(
            {
                "PERCI_WEIGHTS": str(model),
                "PERCI_DAEMON_PORT": str(port),
                "PERCI_CORTEX_MODE": "off",
                "PERCI_COLOR": "never",
                "PERCI_SESSION": str(temp_root / "session.jsonl"),
                "PERCI_MEMORY": str(temp_root / "memory.jsonl"),
                "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
                "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
                "PERCI_DECISION_TRACE": str(temp_root / "decision-trace.jsonl"),
                "PERCI_DAEMON_ALLOW_OPEN_PING": "1",
            }
        )
        proc = subprocess.Popen(
            [str(binary), "daemon"],
            cwd=str(ROOT),
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
        try:
            for _ in range(120):
                try:
                    request(port, "ping")
                    break
                except Exception:
                    time.sleep(0.05)
            else:
                raise SystemExit("daemon did not start")

            for thread in THREADS:
                # Fresh session per thread for isolation of deictic binding.
                try:
                    request(port, "session_reset")
                except Exception:
                    pass
                for setup in thread.get("setup") or []:
                    request(port, "ask", setup)
                probe = thread["prompt"]
                r1 = request(port, "ask", probe)
                a1 = r1.get("text") or r1.get("answer") or ""
                ok1, reasons1 = score(a1, thread["want_any"], thread.get("forbid") or [])

                # Surgical re-ask once if failed (K recovery signal).
                a2 = a1
                ok2 = ok1
                reasons2 = reasons1
                taught = False
                if not ok1:
                    teach = (
                        f"For the thread about '{thread['setup'][-1] if thread.get('setup') else probe}': "
                        f"answer the follow-up by binding timeout/retry/idempotent/checkable done, "
                        f"or boundary/repair/order mechanisms — never structure-under-constraint shells, "
                        f"never consciousness claims, never auto-promote weights."
                    )
                    try:
                        request(port, "ask", f"Remember only for this session: {teach}")
                        taught = True
                    except Exception:
                        pass
                    r2 = request(port, "ask", probe)
                    a2 = r2.get("text") or r2.get("answer") or ""
                    ok2, reasons2 = score(a2, thread["want_any"], thread.get("forbid") or [])

                rows.append(
                    {
                        "id": thread["id"],
                        "setup": thread.get("setup") or [],
                        "prompt": probe,
                        "pass_first": ok1,
                        "pass_after_reask": ok2,
                        "reasons_first": reasons1,
                        "reasons_reask": reasons2,
                        "taught": taught,
                        "answer_first": a1[:500],
                        "answer_reask": a2[:500] if a2 != a1 else "",
                    }
                )
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except Exception:
                proc.kill()

    first = sum(1 for r in rows if r["pass_first"])
    reask = sum(1 for r in rows if r["pass_after_reask"])
    sticky = [r["id"] for r in rows if not r["pass_after_reask"]]
    receipt = {
        "schema": "perci.multi-turn-probe.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "threads": len(rows),
        "pass_first": first,
        "pass_after_reask": reask,
        "sticky_fail_ids": sticky,
        "recovery_rate": round((reask - first) / max(1, len(rows) - first), 4)
        if first < len(rows)
        else 1.0,
        "rows": rows,
        "claim_boundary": [
            "never auto-promote .pwgt",
            "session teach is not durable weight promote",
            "coherence is not consciousness",
        ],
        "automatic_promotion": False,
    }
    body = json.dumps(receipt, indent=2, ensure_ascii=False) + "\n"
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(body, encoding="utf-8")
    receipt["receipt_sha256"] = hashlib.sha256(body.encode("utf-8")).hexdigest()
    OUT.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(
        json.dumps(
            {
                "pass_first": first,
                "pass_after_reask": reask,
                "total": len(rows),
                "sticky": sticky,
                "output": str(OUT),
            },
            indent=2,
        )
    )
    return 0 if reask == len(rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
