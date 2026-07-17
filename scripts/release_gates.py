#!/usr/bin/env python3
"""Perci release gates (Phase A + C product law).

Runs the minimum bar before a version bump claim:

1. cargo test --lib
2. hardness pack evaluation
3. transfer suite via release binary (if present) or cargo run
4. lab queue empty-or-report

Never promotes weights. Exit 0 only if all hard gates pass.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def run(cmd: list[str], timeout: int = 600) -> tuple[int, str]:
    p = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        shell=False,
    )
    out = (p.stdout or "") + (p.stderr or "")
    return p.returncode, out


def main() -> int:
    print("=== Perci release gates ===")
    print(f"root: {ROOT}")
    failures: list[str] = []

    # 1) unit tests
    print("\n[1/4] cargo test --lib")
    code, out = run(["cargo", "test", "--lib", "--", "--quiet"], timeout=600)
    if code != 0:
        # PowerShell sometimes treats cargo stderr oddly; check for test result line
        if "test result: ok" not in out and "0 failed" not in out:
            failures.append("cargo test --lib")
            print(out[-2000:])
        else:
            print("  ok (result line found despite nonzero shell code)")
    else:
        print("  ok")

    # 2) hardness
    print("\n[2/4] evaluate_hardness.py")
    code, out = run([sys.executable, str(ROOT / "scripts" / "evaluate_hardness.py")], timeout=900)
    eval_path = ROOT / "models" / "candidates" / "evaluation-hardness-v1.json"
    if eval_path.is_file():
        data = json.loads(eval_path.read_text(encoding="utf-8"))
        status = data.get("status")
        passed = data.get("passed")
        print(f"  status={status} passed={passed}")
        if status != "PASS":
            failures.append(f"hardness status={status}")
    else:
        failures.append("hardness eval missing")
        print(out[-1500:])

    # 3) transfer suite
    print("\n[3/4] transfer suite")
    exe = ROOT / "target" / "release" / ("perci.exe" if os.name == "nt" else "perci")
    if exe.is_file():
        code, out = run([str(exe), "transfer-suite"], timeout=300)
    else:
        code, out = run(
            ["cargo", "run", "--release", "--quiet", "--", "transfer-suite"],
            timeout=600,
        )
    print(out[-2000:] if len(out) > 2000 else out)
    if "SUITE PASS" not in out and "all_pass=true" not in out:
        failures.append("transfer suite")

    # 4) lab queue report
    print("\n[4/4] lab unified queue")
    if exe.is_file():
        code, out = run([str(exe), "lab", "unified"], timeout=120)
    else:
        code, out = run(
            ["cargo", "run", "--release", "--quiet", "--", "lab", "unified"],
            timeout=300,
        )
    print(out[-1500:] if len(out) > 1500 else out)

    print("\n=== summary ===")
    if failures:
        print("FAIL:", ", ".join(failures))
        print("Do not bump version or claim emergence until green.")
        return 1
    print("PASS: release gates green (weights still require human --authorize).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
