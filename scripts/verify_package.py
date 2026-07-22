#!/usr/bin/env python3
"""Verify that a complete Perci checkout has its LFS-backed runtime artifacts."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def lfs_filter(path: str) -> str | None:
    result = subprocess.run(
        ["git", "check-attr", "filter", "--", path],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    value = result.stdout.strip().rsplit(":", 1)[-1].strip()
    return value or None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=ROOT / "models" / "PACKAGE_MANIFEST.json",
    )
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    rows = []
    for artifact in manifest.get("artifacts", []):
        relative = artifact["path"]
        path = ROOT / relative
        exists = path.is_file()
        size = path.stat().st_size if exists else None
        actual_hash = sha256(path) if exists else None
        magic = path.read_bytes()[:8].decode("ascii", errors="replace") if exists else None
        row = {
            "path": relative,
            "format": artifact["format"],
            "exists": exists,
            "bytes": size,
            "sha256": actual_hash,
            "hash_matches": actual_hash == artifact["sha256"],
            "size_matches": size == artifact["bytes"],
            "magic": magic,
            "magic_matches": magic == artifact["format"],
            "lfs_filter": lfs_filter(relative),
        }
        row["ok"] = all(
            [
                row["exists"],
                row["hash_matches"],
                row["size_matches"],
                row["magic_matches"],
                row["lfs_filter"] == "lfs",
            ]
        )
        rows.append(row)
    payload = {
        "schema": "perci.package-verification.v1",
        "manifest": str(args.manifest.relative_to(ROOT)),
        "artifact_count": len(rows),
        "ok": bool(rows) and all(row["ok"] for row in rows),
        "artifacts": rows,
    }
    print(json.dumps(payload, indent=2))
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
