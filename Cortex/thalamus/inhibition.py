from __future__ import annotations

from collections import Counter
from typing import Any, Iterable


HARD_EXCLUDED_PREFIXES = ("node_modules/", ".venv/", "venv/", "dist/", "build/", ".cortex/runtime/")
GENERATED_SUFFIXES = (".lock", ".min.js", ".map")


def inhibit(
    hits: Iterable[Any], lane_weights: dict[str, float], *, min_lane_relevance: float = 0.0
) -> list[Any]:
    """Apply deterministic reticular-style gating and retain an evidence audit on hits."""

    items = list(hits)
    duplicates = Counter((item.path, item.content_hash) for item in items)
    strongest_lane = max(lane_weights.values(), default=1.0)
    selected: list[Any] = []
    suppressed: list[tuple[float, Any]] = []
    for hit in items:
        path = hit.path.replace("\\", "/").lower()
        hard = path.startswith(HARD_EXCLUDED_PREFIXES)
        duplicate = max(0.0, (duplicates[(hit.path, hit.content_hash)] - 1) / max(1, len(items) - 1))
        generated = 1.0 if path.endswith(GENERATED_SUFFIXES) else 0.0
        lane = lane_for_hit(hit)
        lane_relevance = lane_weights.get(lane, 0.0) / strongest_lane
        out_of_scope = 1.0 - min(1.0, lane_relevance)
        pruned = not hard and lane_relevance < min_lane_relevance
        soft_inhibition = min(1.0, 0.20 * duplicate + 0.30 * out_of_scope + 0.10 * generated)
        inhibition = 1.0 if hard or pruned else soft_inhibition
        hit.metadata["thalamus"] = {
            "lane": lane,
            "inhibition": round(inhibition, 6),
            "hard_excluded": hard,
            "pruned": pruned,
            "gated_score": round(float(hit.score) * (1.0 - inhibition), 8),
        }
        if pruned:
            suppressed.append((float(hit.score) * (1.0 - soft_inhibition), hit))
        elif not hard:
            hit.score = float(hit.metadata["thalamus"]["gated_score"])
            selected.append(hit)
    if not selected and suppressed:
        # A route may be uncertain or partially indexed. Preserve a bounded fallback instead of
        # silently emitting an empty context packet.
        for score, hit in sorted(suppressed, key=lambda item: (-item[0], item[1].path))[:4]:
            hit.score = score
            hit.metadata["thalamus"]["fallback"] = True
            selected.append(hit)
    return sorted(selected, key=lambda hit: (-hit.score, hit.path, hit.start_line))


def lane_for_hit(hit: Any) -> str:
    path = hit.path.replace("\\", "/").lower()
    if hit.kind == "telemetry":
        return "git"
    if hit.kind == "discovery_card":
        return "decisions"
    if path.startswith("tests/") or "/test_" in path or path.startswith("test_"):
        return "tests"
    if path.startswith(("docs/", "examples/")) or path.endswith((".md", ".rst")):
        return "documentation"
    if path.endswith((".toml", ".yaml", ".yml", ".json", ".ini", ".cfg")):
        return "configuration"
    return "source"
