#!/usr/bin/env python3
"""Compatibility entry point for the deduplicated Perci v2 builder.

The v1 builder needed fixed-size label chunks to create a 200 MiB artifact.
Perci v2 retains only unique activations and is small enough to build directly,
so chunk assembly would duplicate the canonical build path and risk format drift.
"""
from __future__ import annotations

import sys

import build_weights


def main() -> int:
    print(
        "Perci v2 no longer needs chunk assembly; delegating to the canonical "
        "deduplicating builder.",
        file=sys.stderr,
    )
    build_weights.main()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
