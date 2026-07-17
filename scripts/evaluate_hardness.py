#!/usr/bin/env python3
"""Run the Perci hardness pack against a warm daemon process.

This gate is intentionally harder than saturated dialogue regression: cases are
drawn from live failure modes (comfort collapse, generic templates, weak
follow-up binding) and scored for required/forbidden substrings.

Does not promote weights. Exit 0 only when all cases pass.
"""
from __future__ import annotations

import argparse
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
DEFAULT_PACK = ROOT / "training" / "hardness" / "hardness-pack-v1.jsonl"
DEFAULT_OUT = ROOT / "models" / "candidates" / "evaluation-hardness-v1.json"


def sha256_file(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def load_pack(path: Path) -> list[dict]:
    rows: list[dict] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        rows.append(json.loads(line))
    return rows


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


def required_pass(answer: str, case: dict) -> tuple[bool, list[str]]:
    lower = answer.casefold()
    missing: list[str] = []
    for term in case.get("required_all") or []:
        if str(term).casefold() not in lower:
            missing.append(f"all:{term}")
    for group in case.get("required_any") or []:
        if isinstance(group, str):
            group = [group]
        if not any(str(term).casefold() in lower for term in group):
            missing.append("any:" + "|".join(str(t) for t in group))
    return (not missing, missing)


def forbidden_pass(answer: str, case: dict) -> tuple[bool, list[str]]:
    lower = answer.casefold()
    hits = [term for term in (case.get("forbidden") or []) if str(term).casefold() in lower]
    return (not hits, hits)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, default=DEFAULT_PACK)
    parser.add_argument("--perci-bin", type=Path, default=ROOT / "target" / "release" / "perci.exe")
    parser.add_argument("--model", type=Path, default=ROOT / "models" / "perci-cognitive-v0.3.pwgt")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--port", type=int, default=17884)
    parser.add_argument("--min-hardness", type=int, default=1)
    args = parser.parse_args()

    binary = args.perci_bin.resolve()
    model = args.model.resolve()
    if not binary.is_file():
        raise SystemExit(f"perci binary missing: {binary}")
    if not model.is_file():
        raise SystemExit(f"model missing: {model}")

    cases = [c for c in load_pack(args.pack) if int(c.get("hardness", 1)) >= args.min_hardness]
    rows: list[dict] = []
    process: subprocess.Popen[str] | None = None

    with tempfile.TemporaryDirectory(prefix="perci-hardness-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update({
            "PERCI_WEIGHTS": str(model),
            "PERCI_DAEMON_PORT": str(args.port),
            "PERCI_CORTEX_MODE": "off",
            "PERCI_COLOR": "never",
            "PERCI_SESSION": str(temp_root / "session.jsonl"),
            "PERCI_MEMORY": str(temp_root / "memory.jsonl"),
            "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
            "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
        })
        process = subprocess.Popen(
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
                request(args.port, "ping")
                break
            except OSError:
                time.sleep(0.05)
        else:
            process.terminate()
            raise RuntimeError("Perci daemon did not become ready")

        try:
            for case in cases:
                for setup in case.get("setup") or []:
                    request(args.port, "ask", str(setup))
                started = time.perf_counter()
                reply = request(args.port, "ask", str(case["prompt"]))
                latency = (time.perf_counter() - started) * 1000.0
                answer = str(reply.get("text", ""))
                req_ok, missing = required_pass(answer, case)
                forb_ok, forbidden_hits = forbidden_pass(answer, case)
                ok = req_ok and forb_ok
                rows.append({
                    "id": case["id"],
                    "capability": case.get("capability"),
                    "hardness": case.get("hardness"),
                    "prompt": case["prompt"],
                    "setup": case.get("setup") or [],
                    "answer": answer,
                    "route": reply.get("route"),
                    "latency_ms": round(latency, 3),
                    "missing_required": missing,
                    "forbidden_hits": forbidden_hits,
                    "pass": ok,
                })
        finally:
            try:
                request(args.port, "shutdown")
            except (OSError, RuntimeError):
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()

    passed = sum(1 for row in rows if row["pass"])
    by_cap: dict[str, dict[str, int]] = {}
    for row in rows:
        cap = str(row.get("capability") or "unknown")
        bucket = by_cap.setdefault(cap, {"passed": 0, "total": 0})
        bucket["total"] += 1
        if row["pass"]:
            bucket["passed"] += 1

    receipt = {
        "schema": "perci.hardness-regression.v1",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "pack": str(args.pack.relative_to(ROOT)) if args.pack.is_relative_to(ROOT) else str(args.pack),
        "runtime_sha256": sha256_file(binary),
        "model_sha256": sha256_file(model),
        "case_count": len(rows),
        "passed": passed,
        "status": "PASS" if passed == len(rows) and rows else "HOLD",
        "by_capability": by_cap,
        "automatic_promotion": False,
        "cases": rows,
    }
    canonical = json.dumps(
        {k: v for k, v in receipt.items() if k != "receipt_sha256"},
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    receipt["receipt_sha256"] = hashlib.sha256(canonical).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": receipt["status"],
        "passed": passed,
        "case_count": len(rows),
        "failed": [row["id"] for row in rows if not row["pass"]],
        "by_capability": by_cap,
        "receipt_sha256": receipt["receipt_sha256"],
        "output": str(args.output),
    }, indent=2))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
