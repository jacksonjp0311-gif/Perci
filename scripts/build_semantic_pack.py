#!/usr/bin/env python3
"""Build candidate PERCISEM1 via `perci modular build-sem` (Rust)."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "release" / "perci.exe"


def main() -> int:
    if not BIN.is_file():
        print("building release binary…")
        r = subprocess.run(["cargo", "build", "--release"], cwd=ROOT)
        if r.returncode != 0:
            return r.returncode
    return subprocess.call([str(BIN), "modular", "build-sem"], cwd=ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
