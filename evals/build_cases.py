#!/usr/bin/env python3
"""Build Perci's bug2eval cases (evals/b2e/*.b2e) from evals/cases.json.

For every case this script:
  1. exports a *pruned* source snapshot of the before and after refs with
     `git archive` (Git LFS smudging disabled, so weights stay out of the case),
  2. copies the current evals/verifiers/ into both snapshots (bug2eval requires
     the protected verifier to be byte-identical in both),
  3. runs `bug2eval capture --before-dir --after-dir` (which validates that the
     verifier fails on before and passes on after) and `bug2eval pack`.

Usage:
  python evals/build_cases.py --all                 # rebuild every case
  python evals/build_cases.py B2E-PERCI-001 ...      # rebuild some
  python evals/build_cases.py --export B2E-PERCI-001 --to /tmp/x   # just the two snapshots
  python evals/build_cases.py --all --skip-validate  # package without running verifiers

Requires git and bug2eval (pip install "git+https://github.com/jacksonjp0311-gif/bug2eval.git@v0.1.3").
"""

from __future__ import annotations

import argparse
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVALS = ROOT / "evals"
SPEC = EVALS / "cases.json"
OUT = EVALS / "b2e"

# What a snapshot needs to build and run Perci. Everything else (archive/,
# training/, docs/, Cortex/, weights) stays out to keep cases small.
SNAPSHOT_PATHS = ["Cargo.toml", "Cargo.lock", "build.rs", "src", "tests", "config",
                  "knowledge", "models/PACKAGE_MANIFEST.json"]
LFS_POINTERS = ["models/perci-cognitive-v0.1.pwgt", "models/perci-cognitive-v0.2.pwgt",
                "models/perci-cognitive-v0.3.pwgt"]


def git(*args: str, binary: bool = False):
    env = dict(os.environ, GIT_LFS_SKIP_SMUDGE="1")
    proc = subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, env=env)
    if proc.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {proc.stderr.decode(errors='replace')}")
    return proc.stdout if binary else proc.stdout.decode().strip()


def resolve(ref: str) -> str:
    if ref.startswith("grep:"):
        pattern, _, suffix = ref[5:].partition("^")
        sha = git("log", "-1", "--format=%H", f"--grep={pattern}", "--fixed-strings")
        if not sha:
            raise SystemExit(f"no commit message mentions {pattern!r}")
        return git("rev-parse", sha + ("^" if _ else ""))
    return git("rev-parse", ref)


def existing(ref: str, paths: list[str]) -> list[str]:
    names = set(git("ls-tree", "-r", "--name-only", ref).splitlines())
    keep = []
    for p in paths:
        if p in names or any(n.startswith(p + "/") for n in names):
            keep.append(p)
    return keep


def export(ref_spec: str, dest: Path, with_pointers: bool) -> str:
    base, _, patch = ref_spec.partition("+")
    sha = resolve(base)
    paths = existing(sha, SNAPSHOT_PATHS + (LFS_POINTERS if with_pointers else []))
    # -c filter.lfs.*= : write LFS pointer text, never the 200 MiB payload.
    data = subprocess.run(
        ["git", "-C", str(ROOT), "-c", "filter.lfs.smudge=", "-c", "filter.lfs.process=",
         "-c", "filter.lfs.required=false", "archive", "--format=tar", sha, "--", *paths],
        capture_output=True, check=True, env=dict(os.environ, GIT_LFS_SKIP_SMUDGE="1"),
    ).stdout
    dest.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        tar.extractall(dest, filter="data") if sys.version_info >= (3, 12) else tar.extractall(dest)
    if patch:
        subprocess.run(["git", "apply", str(EVALS / patch)], cwd=dest, check=True)
    shutil.copytree(EVALS / "verifiers", dest / "evals" / "verifiers",
                    ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
    return sha + (f"+{patch}" if patch else "")


def bug2eval_cmd() -> list[str]:
    exe = shutil.which("bug2eval")
    return [exe] if exe else [sys.executable, "-m", "bug2eval"]


def build_case(case: dict, skip_validate: bool) -> dict:
    cid = case["id"]
    work = Path(tempfile.mkdtemp(prefix=f"{cid}-"))
    before, after = work / "before", work / "after"
    with_ptr = bool(case.get("with_lfs_pointers"))
    before_src = export(case["before"], before, with_ptr)
    after_src = export(case["after"], after, with_ptr)
    prompt = work / "prompt.md"
    prompt.write_text(
        f"# {case['title']}\n\n{case['prompt']}\n\n"
        f"Source: Perci repository, before `{before_src}`, after `{after_src}`.\n"
        "The verifier builds Perci with cargo; it needs Rust and Python 3.10+.\n",
        encoding="utf-8",
    )
    case_dir = work / "case"
    cmd = bug2eval_cmd() + [
        "capture", "--id", cid, "--title", case["title"], "--verify", case["verify"],
        "--protect", "evals/verifiers", "--before-dir", str(before), "--after-dir", str(after),
        "--prompt-file", str(prompt), "--tags", case.get("tags", "perci"),
        "--timeout", str(case.get("timeout", 900)), "--output", str(case_dir), "--json", "--force",
    ]
    if skip_validate:
        cmd.append("--skip-validate")
    print(f"== {cid}: capture ({before_src[:12]} -> {after_src[:12]})", flush=True)
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        print(proc.stdout[-6000:], proc.stderr[-3000:])
        return {"id": cid, "ok": False, "stage": "capture", "exit": proc.returncode}
    OUT.mkdir(parents=True, exist_ok=True)
    target = OUT / f"{cid}.b2e"
    pack = subprocess.run(bug2eval_cmd() + ["pack", str(case_dir), "--output", str(target)],
                          capture_output=True, text=True)
    if pack.returncode != 0:
        print(pack.stdout, pack.stderr)
        return {"id": cid, "ok": False, "stage": "pack", "exit": pack.returncode}
    shutil.rmtree(work, ignore_errors=True)
    print(f"   packed {target.relative_to(ROOT)} ({target.stat().st_size // 1024} KiB)", flush=True)
    return {"id": cid, "ok": True, "path": str(target.relative_to(ROOT))}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--skip-validate", action="store_true")
    ap.add_argument("--export", metavar="ID")
    ap.add_argument("--to", type=Path)
    args = ap.parse_args()
    cases = json.loads(SPEC.read_text(encoding="utf-8"))["cases"]
    by_id = {c["id"]: c for c in cases}
    if args.export:
        case = by_id[args.export]
        dest = args.to or Path(tempfile.mkdtemp(prefix=f"{args.export}-"))
        for side in ("before", "after"):
            print(side, export(case[side], dest / side, bool(case.get("with_lfs_pointers"))), dest / side)
        return 0
    chosen = cases if args.all else [by_id[i] for i in args.ids]
    if not chosen:
        ap.error("name case ids or pass --all")
    results = [build_case(c, args.skip_validate) for c in chosen]
    bad = [r for r in results if not r["ok"]]
    print(json.dumps({"built": len(results) - len(bad), "failed": bad}, indent=2))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
