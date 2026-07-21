#!/usr/bin/env python3
"""Scaffold candidate pack manifests for modular cognition (never active).

Writes models/candidates/packs/*.pack.json with promotion_status=candidate.
Does not create weight bytes or promote anything.
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "models" / "candidates" / "packs"

PACKS = [
    {
        "magic": "PERCISEM1",
        "pack_id": "percisem1-v0.1-candidate",
        "family": "PERCISEM1",
        "capability_tags": ["semantic", "binding", "intent", "relation"],
        "byte_budget_note": "32-64 MiB engineering budget",
    },
    {
        "magic": "PERCIRSN1",
        "pack_id": "percirsn1-v0.1-candidate",
        "family": "PERCIRSN1",
        "capability_tags": ["reasoning", "transition", "verify", "halt"],
        "byte_budget_note": "96-256 MiB engineering budget",
    },
    {
        "magic": "PERCIDSC1",
        "pack_id": "percidsc1-v0.1-candidate",
        "family": "PERCIDSC1",
        "capability_tags": ["discourse", "rhetorical", "plan"],
        "byte_budget_note": "16-48 MiB engineering budget",
    },
    {
        "magic": "PERCILM1",
        "pack_id": "percilm1-v0.1-candidate",
        "family": "PERCILM1",
        "capability_tags": ["language", "realization", "wording"],
        "byte_budget_note": "128-512 MiB engineering budget",
    },
    {
        "magic": "PERCIFLD1",
        "pack_id": "percifld1-v0.1-candidate",
        "family": "PERCIFLD1",
        "capability_tags": ["fold", "operator", "decode", "experiment"],
        "byte_budget_note": "experiment-dependent",
    },
]


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for p in PACKS:
        manifest = {
            "schema": "perci.pack-manifest.v1",
            "magic": p["magic"],
            "format_version": 1,
            "pack_id": p["pack_id"],
            "family": p["family"],
            "capability_tags": p["capability_tags"],
            "dimensions": 4096,
            "record_count": 0,
            "byte_length": 0,
            "checksum_sha256": "",
            "corpus_hash": "",
            "builder_version": "scaffold_modular_packs.py",
            "evaluation_receipt": "",
            "promotion_status": "candidate",
            "authorization_record": "pending human authorize — never auto-promote",
            "dependency_packs": ["PERCIW03"],
            "compatible_decoders": [p["magic"]],
            "path": "",
            "notes": (
                f"Candidate scaffold only ({p['byte_budget_note']}). "
                "No weight payload. PERCIW03 remains the active reflex field. "
                "See docs/MODULAR_COGNITION_v1.md."
            ),
        }
        path = OUT / f"{p['magic'].lower()}.pack.json"
        path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {path}")
    print("promotion_status=candidate for all · no weight bytes · no auto-promote")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
