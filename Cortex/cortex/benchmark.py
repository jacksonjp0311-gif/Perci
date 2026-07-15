from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def verify_benchmarks(root: Path) -> dict[str, Any]:
    thresholds = json.loads((root / "benchmarks" / "thresholds.json").read_text(encoding="utf-8"))
    routing = json.loads((root / "benchmarks" / "results" / "thalamus_before_after.json").read_text(encoding="utf-8"))
    host = json.loads((root / "benchmarks" / "results" / "self_host_before_after.json").read_text(encoding="utf-8"))
    routed = routing["thalamus_routed"]
    nested = host["nested_cloned_engine"]
    checks = {
        "thalamus_target_rank": routed["median_target_rank"] <= thresholds["thalamus"]["maximum_target_rank"],
        "thalamus_top_3_recall": routed["top_3_target_recall"] >= thresholds["thalamus"]["minimum_top_3_recall"],
        "self_host_certificate": nested["certificate_verified"] is thresholds["self_host"]["require_verified_certificate"],
        "nested_engine_excluded": nested["nested_engine_excluded"] is thresholds["self_host"]["require_nested_engine_excluded"],
        "self_host_activation": nested["activation_seconds"] <= thresholds["self_host"]["maximum_activation_seconds"],
    }
    return {"schema_version": "1.0", "status": "pass" if all(checks.values()) else "fail", "checks": checks, "thresholds": thresholds, "claim_boundary": thresholds["claim_boundary"]}
