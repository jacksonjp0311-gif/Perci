#!/usr/bin/env python3
"""Replay the v0.3.1 concept-leak transcript as a sealed regression suite."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path


CASES = [
    ("sensing", "what are you sensing", ("not sensing anything subjectively", "rotated concept")),
    ("explain_previous", "why do you think this", ("what are you sensing", "association, not knowledge")),
    ("repeat_report", "this is the same answer", ("failed to resolve the new intent", "not deeper reasoning")),
    ("repeat_cause", "why do you keep responding like this", ("retrieval leakage", "not a considered answer")),
    ("malfunction", "something is not working correctly", ("malfunction is in composition", "semantic support")),
    ("diagnosis", "whats going on here?", ("response loop", "block duplicate output")),
]
FORBIDDEN = ("Life is matter organized", "Time is experienced as sequence")


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--perci-bin", type=Path, required=True)
    parser.add_argument("--model", type=Path, default=root / "models/perci-cognitive-v0.3.pwgt")
    parser.add_argument("--output", type=Path, default=root / "models/candidates/evaluation-v3.1-dialogue.json")
    args = parser.parse_args()
    binary = args.perci_bin.resolve()
    model = args.model.resolve()

    rows = []
    outputs: list[str] = []
    with tempfile.TemporaryDirectory(prefix="perci-dialogue-v31-") as temp:
        temp_root = Path(temp)
        env = os.environ.copy()
        env.update({
            "PERCI_WEIGHTS": str(model),
            "PERCI_CORTEX_MODE": "off",
            "PERCI_COLOR": "never",
            "PERCI_SESSION": str(temp_root / "session.jsonl"),
            "PERCI_LEARNING": str(temp_root / "learning.jsonl"),
            "PERCI_DIALOGUE_PROFILE": str(temp_root / "profile.json"),
        })
        for case_id, prompt, required in CASES:
            run = subprocess.run(
                [str(binary), "ask", prompt],
                cwd=root,
                env=env,
                text=True,
                encoding="utf-8",
                capture_output=True,
                check=False,
            )
            answer = run.stdout.strip()
            outputs.append(answer)
            required_pass = all(term.casefold() in answer.casefold() for term in required)
            forbidden_pass = not any(term.casefold() in answer.casefold() for term in FORBIDDEN)
            rows.append({
                "id": case_id,
                "prompt": prompt,
                "answer": answer,
                "required": list(required),
                "required_pass": required_pass,
                "forbidden_pass": forbidden_pass,
                "exit_code": run.returncode,
                "pass": run.returncode == 0 and required_pass and forbidden_pass,
            })

    unique_outputs = len(set(outputs)) == len(outputs)
    passed = sum(1 for row in rows if row["pass"])
    receipt = {
        "schema": "perci.dialogue-regression.v3.1",
        "evaluated_at_utc": datetime.now(timezone.utc).isoformat(),
        "model_sha256": digest(model),
        "runtime_sha256": digest(binary),
        "case_count": len(rows),
        "passed": passed,
        "unique_outputs": unique_outputs,
        "status": "PASS" if passed == len(rows) and unique_outputs else "HOLD",
        "automatic_promotion": False,
        "cases": rows,
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt)).hexdigest()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2, ensure_ascii=False))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
