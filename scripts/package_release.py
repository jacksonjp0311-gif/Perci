#!/usr/bin/env python3
"""Package Perci release assets (used by .github/workflows/release.yml).

    package_release.py binary --target TRIPLE --bin PATH --out DIST
        perci-<TRIPLE>.tar.gz (or .zip for Windows) containing one folder,
        perci/, that is a complete Perci home without the weights:

            perci/bin/perci[.exe]
            perci/models/PACKAGE_MANIFEST.json + *.pwgt.json sidecars
            perci/config/  perci/knowledge/
            perci/docs/{QUICKSTART,TOOLS,PROFILES,BENCHMARKS}.md
            perci/scripts/bench_public.py (+ evals/bench-public-v1/policy_cases.json)
            perci/README.md  LICENSE-MIT  LICENSE-APACHE  VERSION

        `perci` finds its home two levels above the executable, so the
        unpacked folder works from anywhere. `perci fetch-weights` adds the
        weights later.

    package_release.py weights --out DIST [--set default|all]
        Copies the weight files PACKAGE_MANIFEST.json lists into DIST under
        their base names (the names `perci fetch-weights` downloads),
        refusing LFS pointers and anything whose sha256 does not match.

    package_release.py sums DIST
        Writes DIST/SHA256SUMS (sha256sum format) for every file in DIST.

Asset names carry no version, so .../releases/latest/download/<name> works;
the version is inside the archive (VERSION) and in `perci --version`.
"""
import argparse
import hashlib
import io
import json
import os
import re
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS = ["QUICKSTART.md", "TOOLS.md", "PROFILES.md", "BENCHMARKS.md"]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    return re.search(r'^version\s*=\s*"([^"]+)"', text, re.M).group(1)


def home_files(bin_path: Path, windows: bool):
    """(archive path, source path or bytes, executable) for the home folder."""
    exe = "perci.exe" if windows else "perci"
    yield f"bin/{exe}", bin_path, True
    models = ROOT / "models"
    yield "models/PACKAGE_MANIFEST.json", models / "PACKAGE_MANIFEST.json", False
    for p in sorted(models.glob("*.pwgt.json")):
        yield f"models/{p.name}", p, False
    for top in ["config", "knowledge"]:
        for p in sorted((ROOT / top).rglob("*")):
            if p.is_file() and "__pycache__" not in p.parts:
                yield p.relative_to(ROOT).as_posix(), p, False
    for name in DOCS:
        p = ROOT / "docs" / name
        if p.is_file():
            yield f"docs/{name}", p, False
    yield "scripts/bench_public.py", ROOT / "scripts" / "bench_public.py", False
    cases = "evals/bench-public-v1/policy_cases.json"
    yield cases, ROOT / cases, False
    for name in ["README.md", "LICENSE-MIT", "LICENSE-APACHE"]:
        yield name, ROOT / name, False
    yield "VERSION", (version() + "\n").encode(), False


def cmd_binary(a):
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    windows = "windows" in a.target
    bin_path = Path(a.bin)
    if not bin_path.is_file():
        sys.exit(f"binary not found: {bin_path}")
    files = list(home_files(bin_path, windows))
    for arc, src, _ in files:
        if isinstance(src, Path) and not src.is_file():
            sys.exit(f"missing release file: {src}")
    if windows:
        dest = out / f"perci-{a.target}.zip"
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
            for arc, src, _ in files:
                data = src if isinstance(src, bytes) else src.read_bytes()
                z.writestr(f"perci/{arc}", data)
    else:
        dest = out / f"perci-{a.target}.tar.gz"
        with tarfile.open(dest, "w:gz", compresslevel=9) as t:
            for arc, src, exe in files:
                data = src if isinstance(src, bytes) else src.read_bytes()
                info = tarfile.TarInfo(f"perci/{arc}")
                info.size = len(data)
                info.mode = 0o755 if exe else 0o644
                info.mtime = 0
                t.addfile(info, io.BytesIO(data))
            # directories are implied; tar extracts them with default perms
    print(f"{dest.name}  {dest.stat().st_size} bytes  ({len(files)} files)")


def cmd_weights(a):
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((ROOT / "models" / "PACKAGE_MANIFEST.json").read_text(encoding="utf-8"))
    total = 0
    for e in manifest["artifacts"]:
        kind = e.get("kind", "")
        if a.set == "default" and kind.startswith("legacy"):
            continue
        src = ROOT / e["path"]
        with open(src, "rb") as f:
            if f.read(23) == b"version https://git-lfs":
                sys.exit(f"{src} is an LFS pointer; pull the weights first")
        if src.stat().st_size != e["bytes"] or sha256(src) != e["sha256"]:
            sys.exit(f"{src} does not match PACKAGE_MANIFEST.json")
        shutil.copyfile(src, out / src.name)
        total += e["bytes"]
        print(f"{src.name}  {e['bytes']} bytes  sha256 ok")
    print(f"weights total {total} bytes")


def cmd_sums(a):
    dist = Path(a.dist)
    lines = [f"{sha256(p)}  {p.name}" for p in sorted(dist.iterdir()) if p.is_file() and p.name != "SHA256SUMS"]
    (dist / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("binary")
    b.add_argument("--target", required=True)
    b.add_argument("--bin", required=True)
    b.add_argument("--out", required=True)
    w = sub.add_parser("weights")
    w.add_argument("--out", required=True)
    w.add_argument("--set", choices=["default", "all"], default="default")
    s = sub.add_parser("sums")
    s.add_argument("dist")
    sub.add_parser("version")
    a = ap.parse_args()
    if a.cmd == "version":
        print(version())
        return
    {"binary": cmd_binary, "weights": cmd_weights, "sums": cmd_sums}[a.cmd](a)


if __name__ == "__main__":
    os.umask(0o022)
    main()
