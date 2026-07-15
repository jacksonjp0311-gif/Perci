from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

from .hippocampus import active_session


class Governor:
    """Negative-feedback controller for repository memory trust and context scope."""

    def __init__(self, home: Path, store: Any) -> None:
        self.home = home
        self.store = store

    def evaluate(
        self,
        repo: str,
        *,
        retrieval_confidence: float = 0.0,
        manifest_current: bool | None = None,
        certificate: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        repository = self.store.repo(repo)
        if not repository:
            return {
                "stability": 0.0,
                "mode": "read_only",
                "reason": "repository is not attached",
                "components": {},
            }

        certificate_row = self.store.latest_bootstrap(repo)
        certificate = certificate or {}
        if not certificate and certificate_row:
            try:
                certificate = json.loads(certificate_row["certificate"] or "{}")
            except json.JSONDecodeError:
                certificate = {}

        integrity = 1.0 if self.store.integrity_check() else 0.0
        certificate_status = certificate.get("status")
        if certificate_status == "verified":
            integrity *= 1.0
        elif certificate_status == "degraded":
            integrity *= 0.65
        else:
            integrity *= 0.25

        focus = 1.0 if active_session(self.home, repo) else 0.35
        if manifest_current is True:
            freshness = 1.0
        elif manifest_current is False:
            freshness = 0.20
        else:
            last_indexed = repository["last_indexed"] or 0
            age_hours = max(0.0, (time.time() - last_indexed) / 3600.0)
            freshness = max(0.25, 1.0 - min(age_hours, 168.0) / 210.0)

        latest_session = self.store.latest_session(repo)
        continuity = 0.80 if latest_session else 0.45
        if latest_session and latest_session["status"] == "active":
            continuity = 1.0

        confidence = max(0.0, min(1.0, retrieval_confidence))
        stability = (
            0.30 * integrity
            + 0.25 * focus
            + 0.20 * freshness
            + 0.15 * confidence
            + 0.10 * continuity
        )
        stability = round(stability, 6)

        if certificate_status != "verified" or manifest_current is False:
            mode = "read_only"
            reason = "bootstrap certificate missing/degraded or repository manifest drifted"
        elif stability >= 0.72:
            mode = "normal"
            reason = "repository memory is certified, current, focused, and sufficiently confident"
        elif stability >= 0.55:
            mode = "constrained"
            reason = "memory is usable but scope should remain narrow and dry-run-first"
        else:
            mode = "read_only"
            reason = "stability is below the mutation-support threshold"

        return {
            "stability": stability,
            "mode": mode,
            "reason": reason,
            "components": {
                "integrity": round(integrity, 6),
                "focus": round(focus, 6),
                "freshness": round(freshness, 6),
                "retrieval_confidence": round(confidence, 6),
                "continuity": round(continuity, 6),
            },
            "authority": {
                "cortex_may_authorize_mutation": False,
                "host_repository_rules_control": True,
                "human_authorization_required": True,
            },
        }
