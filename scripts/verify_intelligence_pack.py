#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "knowledge/packs/perci-core-intelligence-v1")
    manifest_path = root / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

    failures: list[str] = []
    for record in manifest["files"]:
        path = root / record["path"]
        if not path.is_file():
            failures.append(f"missing: {record['path']}")
            continue
        actual = sha256(path)
        if actual != record["sha256"]:
            failures.append(f"hash mismatch: {record['path']}")

    if failures:
        print("\n".join(failures))
        return 1

    print(
        f"verified {manifest['pack_id']} v{manifest['version']} "
        f"({len(manifest['files'])} files)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())