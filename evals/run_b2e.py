#!/usr/bin/env python3
"""Validate Perci's bug2eval cases and write a receipt.

  python evals/run_b2e.py                    # every evals/b2e/*.b2e
  python evals/run_b2e.py B2E-PERCI-001 ...  # some cases
  python evals/run_b2e.py --receipt evals/receipts/baseline-2026-10-17.json

Each case is checked with `bug2eval validate --json`: its verifier must fail on
the broken snapshot and pass on the fixed one. Exit code 0 only if every case
is valid. Set PERCI_B2E_TARGET to share one cargo target dir across cases
(much faster), and PERCI_AGENTSEARCH to exercise AgentSearch in B2E-PERCI-011.
The same check is available from Rust as perci::b2e::validate_case and on the
CLI as `perci eval validate`.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "evals" / "b2e"


def bug2eval() -> list[str]:
    if os.environ.get("PERCI_BUG2EVAL"):
        return os.environ["PERCI_BUG2EVAL"].split()
    exe = shutil.which("bug2eval")
    return [exe] if exe else [sys.executable, "-m", "bug2eval"]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cases", nargs="*", help="case ids or .b2e paths (default: all)")
    ap.add_argument("--receipt", type=Path, help="receipt path (default evals/receipts/b2e-<utc>.json)")
    ap.add_argument("--timeout", type=int, default=1800, help="seconds per case")
    args = ap.parse_args()

    if args.cases:
        paths = [Path(c) if Path(c).exists() else CASES / f"{c}.b2e" for c in args.cases]
    else:
        paths = sorted(CASES.glob("*.b2e"))
    if not paths:
        print("no cases found", file=sys.stderr)
        return 2
    version = subprocess.run(bug2eval() + ["--version"], capture_output=True, text=True).stdout.strip()
    head = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    results = []
    for path in paths:
        start = time.monotonic()
        try:
            proc = subprocess.run(bug2eval() + ["validate", "--json", str(path)], capture_output=True,
                                  text=True, encoding="utf-8", errors="replace", timeout=args.timeout)
            code, out = proc.returncode, proc.stdout
        except subprocess.TimeoutExpired:
            code, out = None, ""
        elapsed = round(time.monotonic() - start, 1)
        try:
            data = json.loads(out)
        except ValueError:
            data = {"error": "no JSON from bug2eval", "stderr": (proc.stderr if code is not None else "timeout")[-1500:]}
        entry = {
            "case_id": data.get("case_id", path.stem),
            "case": str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path),
            "case_sha256": sha256(path) if path.is_file() else None,
            "valid": bool(data.get("valid")) and code == 0,
            "exit_code": code,
            "before_exit": (data.get("before") or {}).get("exit_code"),
            "after_exit": (data.get("after") or {}).get("exit_code"),
            "verifier_intact": (data.get("verifier_integrity") or {}).get("intact"),
            "reasons": data.get("reasons", []) + ([data["error"]] if "error" in data else []),
            "seconds": elapsed,
        }
        if not entry["valid"]:
            entry["before_stdout_tail"] = ((data.get("before") or {}).get("stdout") or "")[-1200:]
            entry["after_stdout_tail"] = ((data.get("after") or {}).get("stdout") or "")[-1200:]
        results.append(entry)
        mark = "VALID  " if entry["valid"] else "INVALID"
        print(f"{mark} {entry['case_id']:<15} before={entry['before_exit']} after={entry['after_exit']} "
              f"{elapsed:>6}s {'; '.join(map(str, entry['reasons']))}", flush=True)

    valid = sum(r["valid"] for r in results)
    receipt = {
        "schema": "perci.b2e-receipt.v1",
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "git_head": head,
        "bug2eval": version,
        "platform": f"{platform.system()} {platform.release()} / Python {platform.python_version()}",
        "agentsearch": os.environ.get("PERCI_AGENTSEARCH", "default"),
        "valid": valid,
        "total": len(results),
        "results": results,
    }
    out_path = args.receipt or ROOT / "evals" / "receipts" / (
        "b2e-" + dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + ".json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"{valid}/{len(results)} valid · receipt {out_path}")
    return 0 if valid == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
