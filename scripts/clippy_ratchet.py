#!/usr/bin/env python3
"""Fail if a change adds clippy warnings ("no new warnings" ratchet).

Perci has ~95 pre-existing clippy warnings, so `-D warnings` would fail every
build. Instead this counts warnings per (lint, file) on HEAD and on a base
commit, built with the *same* toolchain, and fails if any count went up.
Line numbers are ignored, so moving code around does not trip it.

  python scripts/clippy_ratchet.py --base origin/main
  python scripts/clippy_ratchet.py --base HEAD~1 --json-out clippy.json

The base is checked out into a temporary git worktree (no LFS payloads needed).
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def clippy_counts(src: Path, target: Path) -> collections.Counter:
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    proc = subprocess.run(
        ["cargo", "clippy", "--all-targets", "--message-format=json", "-q"],
        cwd=src, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    counts: collections.Counter = collections.Counter()
    seen = set()
    for line in proc.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        m = msg["message"]
        if m.get("level") == "error":
            print(m.get("rendered", ""), file=sys.stderr)
            raise SystemExit(f"clippy failed to build {src}")
        if m.get("level") != "warning" or not m.get("spans"):
            continue
        span = next((s for s in m["spans"] if s.get("is_primary")), m["spans"][0])
        code = (m.get("code") or {}).get("code") or m.get("message", "")[:60]
        # The same warning is reported once per target (lib, lib test, bin...).
        ident = (code, span["file_name"], span["line_start"], span["column_start"])
        if ident in seen:
            continue
        seen.add(ident)
        counts[(code, span["file_name"].replace("\\", "/"))] += 1
    if proc.returncode != 0 and not counts:
        print(proc.stderr[-4000:], file=sys.stderr)
        raise SystemExit("cargo clippy failed")
    return counts


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="git ref to compare against")
    ap.add_argument("--target-dir", type=Path, default=Path(tempfile.gettempdir()) / "perci-clippy-ratchet")
    ap.add_argument("--json-out", type=Path)
    args = ap.parse_args()

    base_sha = subprocess.run(["git", "-C", str(ROOT), "rev-parse", args.base],
                              capture_output=True, text=True, check=True).stdout.strip()
    head = clippy_counts(ROOT, args.target_dir / "head")
    wt = Path(tempfile.mkdtemp(prefix="perci-clippy-base-"))
    env = dict(os.environ, GIT_LFS_SKIP_SMUDGE="1")
    subprocess.run(["git", "-C", str(ROOT), "worktree", "add", "--detach", str(wt), base_sha],
                   check=True, capture_output=True, env=env)
    try:
        base = clippy_counts(wt, args.target_dir / "base")
    finally:
        subprocess.run(["git", "-C", str(ROOT), "worktree", "remove", "--force", str(wt)], capture_output=True)
        shutil.rmtree(wt, ignore_errors=True)

    new = {f"{k[0]} @ {k[1]}": (base.get(k, 0), v) for k, v in head.items() if v > base.get(k, 0)}
    fixed = sum(max(0, base[k] - head.get(k, 0)) for k in base)
    summary = {"base": base_sha, "base_total": sum(base.values()), "head_total": sum(head.values()),
               "new": new, "fixed": fixed}
    if args.json_out:
        args.json_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"clippy warnings: base {summary['base_total']} -> head {summary['head_total']} (fixed {fixed})")
    for key, (b, h) in sorted(new.items()):
        print(f"  NEW: {key}: {b} -> {h}")
    return 1 if new else 0


if __name__ == "__main__":
    sys.exit(main())
