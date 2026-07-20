#!/usr/bin/env python3
"""Deprecated shim — HYDRA inject is native Rust in Perci.

Use:

  cargo run --release -- hydra status
  cargo run --release -- hydra field
  cargo run --release -- hydra markers --slots-only
  cargo run --release -- hydra plan <spec.json>
  cargo run --release -- hydra apply <spec.json>          # dry-run
  cargo run --release -- hydra apply <spec.json> --write  # after review

See src/hydra_inject.rs. No external hydra-inject install required.
Never auto-promotes .pwgt.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    print(
        "hydra_perci_bridge.py is deprecated.\n"
        "HYDRA inject lives in Rust:  perci hydra <status|markers|field|plan|apply>\n"
        "Forwarding to: cargo run --release -- hydra …\n",
        file=sys.stderr,
    )
    cmd = ["cargo", "run", "--release", "--quiet", "--", "hydra", *sys.argv[1:]]
    return subprocess.call(cmd, cwd=str(ROOT))


if __name__ == "__main__":
    raise SystemExit(main())
