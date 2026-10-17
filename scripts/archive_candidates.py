#!/usr/bin/env python3
"""Archive unreferenced files from models/candidates/ (move, never delete).

A candidate file is *referenced* when its repo path or file name appears in
any reference source: README.md, VALIDATION.md, WEIGHTS.md, AGENTS.md,
docs/**, models/*.json(l) (promotion ledger, manifests), src/**, scripts/**,
training/**, Launch-Perci.ps1. Code, scripts and docs that name a receipt
keep it in place. Everything else is moved with `git mv` to
archive/candidates-<date>/, preserving sub-folders, so history and Git LFS
pointers stay intact.

Usage:
  python scripts/archive_candidates.py            # dry run: list what would move
  python scripts/archive_candidates.py --apply    # git mv the unreferenced files
  python scripts/archive_candidates.py --check    # exit 1 if any are unreferenced
"""

from __future__ import annotations

import argparse
import datetime as _dt
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CANDIDATES = "models/candidates/"
REFERENCE_GLOBS = [
    "README.md",
    "VALIDATION.md",
    "WEIGHTS.md",
    "AGENTS.md",
    "Launch-Perci.ps1",
    "docs/**/*",
    "models/*.json",
    "models/*.jsonl",
    "src/**/*",
    "scripts/**/*",
    "training/**/*",
]
TEXT_LIMIT = 8 * 1024 * 1024  # skip giant data files when building the corpus


def git_files(*paths: str) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", *paths],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout.decode("utf-8", "replace")
    return [p for p in out.split("\0") if p]


def reference_corpus() -> str:
    chunks: list[str] = []
    seen: set[Path] = set()
    for pattern in REFERENCE_GLOBS:
        for path in ROOT.glob(pattern):
            if path in seen or not path.is_file():
                continue
            seen.add(path)
            rel = path.relative_to(ROOT).as_posix()
            if rel.startswith(CANDIDATES) or rel == "scripts/archive_candidates.py":
                continue
            try:
                if path.stat().st_size > TEXT_LIMIT:
                    continue
                chunks.append(path.read_text(encoding="utf-8", errors="ignore"))
            except OSError:
                continue
    return "\n".join(chunks)


def classify() -> tuple[list[str], list[str]]:
    corpus = reference_corpus()
    keep: list[str] = []
    move: list[str] = []
    for rel in git_files(CANDIDATES):
        name = rel.rsplit("/", 1)[-1]
        sub = rel[len(CANDIDATES):]
        # Pack folders are discovered by directory scan at runtime.
        if sub.startswith("packs/"):
            keep.append(rel)
            continue
        if rel in corpus or sub in corpus or name in corpus:
            keep.append(rel)
        else:
            move.append(rel)
    return keep, move


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true", help="git mv unreferenced files")
    ap.add_argument("--check", action="store_true", help="exit 1 if unreferenced files exist")
    ap.add_argument("--date", default=_dt.date.today().isoformat())
    args = ap.parse_args()

    keep, move = classify()
    dest_root = f"archive/candidates-{args.date}"
    size = sum((ROOT / p).stat().st_size for p in move if (ROOT / p).exists())
    print(f"referenced: {len(keep)}  unreferenced: {len(move)} ({size / 1e6:.1f} MB)")
    if args.check:
        for p in move:
            print(f"  unreferenced {p}")
        return 1 if move else 0
    for p in move:
        dest = f"{dest_root}/{p[len(CANDIDATES):]}"
        if args.apply:
            (ROOT / dest).parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(["git", "mv", "-k", p, dest], cwd=ROOT, check=True)
        print(f"  {'moved' if args.apply else 'would move'} {p} -> {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
