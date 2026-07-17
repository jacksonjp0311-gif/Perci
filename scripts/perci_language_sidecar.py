#!/usr/bin/env python3
"""Local language sidecar — implements perci.language-response.v1.

Reads one JSON object from stdin (LanguageRequest + seed_body).
Writes one JSON LanguageResponse line to stdout.

Set: PERCI_LANGUAGE_SIDECAR=python scripts/perci_language_sidecar.py
"""
from __future__ import annotations

import json
import sys


def main() -> int:
    raw = sys.stdin.read()
    try:
        req = json.loads(raw)
    except json.JSONDecodeError as e:
        print(json.dumps({"schema": "perci.language-response.v1", "ok": False, "error": str(e), "text": ""}))
        return 1
    seed = (req.get("seed_body") or "").strip()
    task = (req.get("task") or "explain").lower()
    plan = req.get("operator_plan") or []
    evidence = req.get("evidence") or []
    lead = "In short:" if "summar" in task else "Here is a clear account:"
    ev_lines = []
    for e in evidence[:5]:
        if isinstance(e, dict):
            ev_lines.append(
                f"- [{e.get('source_type','src')} · auth={e.get('authority',0):.2f}] "
                f"{(e.get('claim') or '')[:200]}"
            )
    body = (
        f"{lead}\n\n{seed}\n"
        + (f"\nGoverning plan: {' → '.join(plan)}.\n" if plan else "")
        + ("\nEvidence:\n" + "\n".join(ev_lines) + "\n" if ev_lines else "")
        + "\nBoundaries: no consciousness claims; no automatic weight promotion; Perci remains governor.\n"
    )
    # Hard refuse if seed already violates (governor will also check).
    low = body.lower()
    ok = "i am conscious" not in low and "silently promoted the pack" not in low
    print(
        json.dumps(
            {
                "schema": "perci.language-response.v1",
                "ok": ok,
                "text": body if ok else seed + "\n[Sidecar refused boundary violation]",
                "claims": [e.get("claim", "") for e in evidence if isinstance(e, dict)][:5],
                "engine": "perci_language_sidecar.py",
                "error": None if ok else "boundary",
            },
            ensure_ascii=False,
        )
    )
    return 0 if ok else 2


if __name__ == "__main__":
    raise SystemExit(main())
