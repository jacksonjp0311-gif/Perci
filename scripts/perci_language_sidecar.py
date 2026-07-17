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
    evidence = req.get("evidence") or []
    # Keep generated prose in the foreground. Governance metadata belongs in
    # Perci's trace; repeating the same header/footer made every response feel
    # like a preset rather than a conversation.
    lead = "In short.\n\n" if "summar" in task else ""
    ev_lines = []
    for e in evidence[:5]:
        if isinstance(e, dict):
            ev_lines.append(
                f"- [{e.get('source_type','src')} · auth={e.get('authority',0):.2f}] "
                f"{(e.get('claim') or '')[:200]}"
            )
    body = (
        f"{lead}{seed}"
        + ("\n\nFor provenance:\n" + "\n".join(ev_lines) if ev_lines else "")
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
