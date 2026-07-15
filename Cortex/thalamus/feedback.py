from __future__ import annotations

from typing import Any, Iterable


REWARDS = {
    "led_to_test_pass": 1.0, "led_to_patch": 0.7, "helpful": 0.5, "used": 0.2,
    "ignored": -0.05, "misleading": -0.6, "contradicted": -0.8, "led_to_failure": -1.0,
}


def record_feedback(store: Any, repo: str, memory_id: int, outcome: str, *, alpha: float = 0.10) -> dict[str, Any]:
    if outcome not in REWARDS:
        raise ValueError(f"Unsupported feedback outcome: {outcome}")
    key = f"thalamus:feedback:{repo}:{memory_id}"
    previous = float(store.get_setting(key, {"score": 0.0})["score"])
    score = max(-1.0, min(1.0, (1.0 - alpha) * previous + alpha * REWARDS[outcome]))
    payload = {"memory_id": memory_id, "outcome": outcome, "score": round(score, 6), "previous_score": previous}
    store.set_setting(key, payload)
    return payload


def apply_feedback(store: Any, repo: str, hits: Iterable[Any]) -> list[Any]:
    """Apply bounded learned usefulness without overriding retrieval provenance."""
    result = list(hits)
    for hit in result:
        feedback = store.get_setting(f"thalamus:feedback:{repo}:{hit.memory_id}", {"score": 0.0})
        usefulness = max(-1.0, min(1.0, float(feedback.get("score", 0.0))))
        hit.score *= 1.0 + usefulness * 0.15
        hit.metadata["thalamus_feedback"] = round(usefulness, 6)
    return result
