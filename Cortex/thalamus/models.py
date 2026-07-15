from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any


@dataclass(frozen=True)
class ThalamicRequest:
    """Normalized, local-only input to the deterministic routing layer."""

    request_id: str
    repository_id: str
    task: str
    timestamp_utc: str
    active_files: tuple[str, ...] = ()
    recent_errors: tuple[str, ...] = ()
    current_branch: str | None = None
    working_tree_dirty: bool = False
    requested_mode: str = "assist"
    token_budget: int = 1200
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class RoutePlan:
    """Auditable retrieval plan. It recommends scope; it grants no authority."""

    request_id: str
    primary_intent: str
    secondary_intents: tuple[str, ...]
    lane_weights: dict[str, float]
    source_weights: dict[str, float]
    inhibition_rules: tuple[str, ...]
    query_expansions: tuple[str, ...]
    evidence_budget: dict[str, int]
    confidence: float
    uncertainty: float
    requires_refresh: bool
    requires_human_review: bool
    explanation: tuple[str, ...]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)
