from __future__ import annotations

from typing import Any


def efficiency_telemetry(*, direct_candidates: int, context_tokens: int, context_budget: int, neural: dict[str, Any]) -> dict[str, Any]:
    """Report bounded computational work; this is not a biological-energy measurement."""
    metrics = neural.get("metrics", {}) if neural else {}
    nodes_considered = int(metrics.get("nodes_considered", 0))
    total_nodes = int(metrics.get("total_nodes", 0))
    return {
        "kind": "computational_efficiency_proxy",
        "direct_candidates": direct_candidates,
        "nodes_considered": nodes_considered,
        "total_nodes": total_nodes,
        "node_scan_fraction": round(nodes_considered / max(1, total_nodes), 6),
        "context_tokens": context_tokens,
        "context_budget": context_budget,
        "context_budget_fraction": round(context_tokens / max(1, context_budget), 6),
        "claim_boundary": "Work counters are engineering telemetry, not a measurement of biological or physical energy.",
    }
