#!/usr/bin/env python3
"""Verify Perci's active cognitive weight file (default: PERCIW03)."""
from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

MAGICS = {1: b"PERCIW01", 2: b"PERCIW02", 3: b"PERCIW03"}
FIXED = struct.Struct("<8sIIIIQQQ32s")


def default_model() -> Path:
    """Prefer active promoted pack, then env-compatible fallbacks."""
    for candidate in (
        Path("models/perci-cognitive-v0.3.pwgt"),
        Path("models/perci-cognitive-v0.2.pwgt"),
        Path("models/perci-cognitive-v0.1.pwgt"),
        Path("models/candidates/perci-cognitive-v0.3.pwgt"),
        Path("models/candidates/perci-cognitive-v0.2.pwgt"),
    ):
        if candidate.is_file():
            return candidate
    return Path("models/perci-cognitive-v0.3.pwgt")


def main() -> None:
    ap = argparse.ArgumentParser(description="Verify Perci .pwgt against sidecar JSON")
    ap.add_argument("--model", type=Path, default=None, help="Path to .pwgt (default: active v0.3)")
    ns = ap.parse_args()
    model = ns.model or default_model()
    if not model.is_file():
        raise SystemExit(f"model missing: {model} (build or place PERCIW03 under models/)")

    manifest_path = model.with_suffix(model.suffix + ".json")
    if not manifest_path.is_file():
        raise SystemExit(f"manifest missing: {manifest_path}")

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    size = model.stat().st_size
    digest = hashlib.sha256()
    with model.open("rb") as fh:
        header = fh.read(FIXED.size)
        if len(header) != FIXED.size:
            raise SystemExit("truncated header")
        values = FIXED.unpack(header)
        fh.seek(0)
        for chunk in iter(lambda: fh.read(4 * 1024 * 1024), b""):
            digest.update(chunk)

    magic, version, bits, words, labels, records, header_size, target_size, _corpus = values
    expected_magic = MAGICS.get(version)
    checks = {
        "file_exists": True,
        "magic": magic == expected_magic,
        "version_matches_manifest": version == int(manifest.get("version", -1)),
        "bits": bits == int(manifest.get("bits_per_activation", 0)),
        "words": words == int(manifest.get("words_per_activation", 0)),
        "label_count": labels == len(manifest.get("labels", [])),
        "records": records == int(manifest.get("prototype_count", -1)),
        "size_file": size == int(manifest.get("size_bytes", -1)),
        "size_header_target": size == int(target_size),
        "sha256": digest.hexdigest() == manifest.get("sha256"),
        "is_perciw03_preferred": version == 3 or not Path("models/perci-cognitive-v0.3.pwgt").is_file(),
    }
    report = {
        "model": str(model),
        "manifest": str(manifest_path),
        "version": version,
        "magic": magic.decode("ascii", errors="replace"),
        "size_bytes": size,
        "prototype_count": records,
        "sha256": digest.hexdigest(),
        "checks": checks,
        "ok": all(checks.values()),
    }
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["ok"] else 1)


if __name__ == "__main__":
    main()
