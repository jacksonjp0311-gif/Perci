from __future__ import annotations

import hashlib
import math
from datetime import datetime, timezone
from typing import Any

from .classifier import classify
from .models import RoutePlan, ThalamicRequest


LANES = ("source", "symbols", "structure", "tests", "documentation", "decisions", "episodes", "git", "runtime", "failures", "configuration", "security", "cross_repository")

INTENT_LANES: dict[str, tuple[str, ...]] = {
    "debug": ("source", "tests", "failures", "runtime", "git", "structure"),
    "code_change": ("source", "tests", "structure", "documentation"),
    "architecture": ("documentation", "structure", "symbols", "source", "decisions"),
    "documentation": ("documentation", "source", "structure", "decisions"),
    "testing": ("tests", "source", "failures", "structure"),
    "release": ("configuration", "documentation", "tests", "git"),
    "configuration": ("configuration", "source", "documentation", "tests"),
    "security_review": ("security", "source", "configuration", "tests"),
    "historical_inquiry": ("git", "episodes", "decisions", "runtime", "source"),
    "repository_orientation": ("documentation", "structure", "symbols", "configuration"),
    "memory_maintenance": ("episodes", "decisions", "runtime", "git"),
    "unknown": ("source", "documentation", "structure"),
}


def make_request(repository: Any, task: str, budget: int, *, active_files: tuple[str, ...] = (), recent_errors: tuple[str, ...] = ()) -> ThalamicRequest:
    stamp = datetime.now(timezone.utc).isoformat()
    request_id = "thal_" + hashlib.sha256(f"{repository['repository_id']}|{task}|{budget}".encode()).hexdigest()[:20]
    return ThalamicRequest(request_id, repository["repository_id"], task, stamp, active_files, recent_errors, token_budget=budget)


def route(request: ThalamicRequest, *, manifest_current: bool | None = None) -> RoutePlan:
    primary, secondary, confidence = classify(request.task, recent_errors=request.recent_errors, active_files=request.active_files)
    dominant = INTENT_LANES[primary]
    raw = {lane: 0.04 for lane in LANES}
    for position, lane in enumerate(dominant):
        raw[lane] += 0.80 - position * 0.08
    for intent in secondary:
        for lane in INTENT_LANES[intent][:3]:
            raw[lane] += 0.12
    if request.recent_errors:
        raw["failures"] += 0.18
        raw["runtime"] += 0.10
    if primary == "unknown":
        raw["documentation"] += 0.10
        raw["structure"] += 0.10
    total = sum(raw.values())
    weights = {lane: round(value / total, 6) for lane, value in raw.items()}
    entropy = -sum(weight * math.log(weight) for weight in weights.values()) / math.log(len(weights))
    uncertainty = round(min(1.0, 0.30 * entropy + 0.55 * (1.0 - confidence) + (0.15 if manifest_current is False else 0.0)), 6)
    active = [lane for lane, weight in weights.items() if weight >= 0.07]
    available = max(0, request.token_budget - int(request.token_budget * 0.25))
    allocation_total = sum(weights[lane] for lane in active) or 1.0
    budgets = {lane: max(40, int(available * weights[lane] / allocation_total)) for lane in active}
    return RoutePlan(
        request.request_id, primary, secondary, weights,
        {"lexical": 0.45, "semantic": 0.55},
        ("exclude generated and runtime artifacts", "down-rank duplicate evidence", "retain source provenance"),
        tuple(lane.replace("_", " ") for lane in dominant[:4]), budgets, confidence, uncertainty,
        manifest_current is False, uncertainty >= 0.70,
        (f"Primary intent: {primary}.", f"Dominant lanes: {', '.join(dominant[:4])}.", "Thalamus is advisory and cannot grant mutation authority."),
    )
