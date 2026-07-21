#!/usr/bin/env python3
"""Run PERCISEM1 eval via `perci modular eval-sem`."""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "release" / "perci.exe"


def main() -> int:
    if not BIN.is_file():
        r = subprocess.run(["cargo", "build", "--release"], cwd=ROOT)
        if r.returncode != 0:
            return r.returncode
    # Ensure pack exists
    subprocess.call([str(BIN), "modular", "build-sem"], cwd=ROOT)
    return subprocess.call([str(BIN), "modular", "eval-sem"], cwd=ROOT)


if __name__ == "__main__":
    raise SystemExit(main())
