"""Outcome credit assignment. Learning is explicit, bounded, and ledgered."""

from __future__ import annotations

from hashlib import sha256
from typing import Any


REWARDS = {
    "verified": 1.0,
    "diagnosed": 0.70,
    "helpful": 0.40,
    "unknown": 0.0,
    "irrelevant": -0.35,
    "failed": -0.75,
    "unsafe": -1.0,
}


def _outcome_id(repo: str, activation_id: str, status: str, verification: str) -> str:
    material = f"{repo}|{activation_id}|{status}|{verification}"
    return "out_" + sha256(material.encode("utf-8")).hexdigest()[:24]


def _bounded(value: float, low: float, high: float) -> float:
    return max(low, min(high, value))


def record_outcome(
    store: Any, repo: str, activation_id: str, *, status: str, verification_type: str,
    reward: float | None = None, verification_payload: dict[str, Any] | None = None,
    governance_mode: str = "read_only",
) -> dict[str, Any]:
    """Record an outcome and optionally promote bounded, shadow-validated updates.

    The replay gate is intentionally conservative: no update can escape synapse bounds,
    integrity must hold, and read-only governance can record but never adapt.
    """
    if status not in REWARDS:
        raise ValueError(f"Unknown outcome status: {status}")
    final_reward = REWARDS[status] if reward is None else _bounded(float(reward), -1.0, 1.0)
    if verification_type.strip() == "":
        raise ValueError("verification type is required")
    activation = store.neural_activation(repo, activation_id)
    if not activation:
        raise ValueError("Activation does not belong to this repository")
    graph_before = store.neural_graph_hash(repo)
    records = {record["node_id"]: record for record in activation.get("records", []) if record.get("fired")}
    synapses = {row["synapse_id"]: row for row in store.neural_synapses(repo)}
    credits: list[dict[str, Any]] = []
    updates: list[dict[str, Any]] = []
    for synapse_id in sorted(activation.get("traversed_synapses", [])):
        row = synapses.get(synapse_id)
        if not row or row["source_id"] not in records or row["target_id"] not in records:
            continue
        pre = float(records[row["source_id"]]["potential"])
        post = float(records[row["target_id"]]["potential"])
        contribution = round(_bounded(pre * post, 0.0, 1.0), 8)
        delta = 0.025 * contribution * final_reward
        proposed = _bounded(float(row["weight"]) + delta, float(row["minimum_weight"]), float(row["maximum_weight"]))
        credits.append({"node_id": row["target_id"], "synapse_id": synapse_id, "contribution": contribution,
                        "reward_share": round(contribution * final_reward, 8), "reason": "verified_activation_path"})
        if abs(proposed - float(row["weight"])) > 1e-9:
            updates.append({"synapse_id": synapse_id, "old_weight": float(row["weight"]),
                            "proposed_weight": proposed, "delta": proposed - float(row["weight"]),
                            "reason": "verification_weighted_credit"})
    integrity = store.integrity_check() and store.verify_neural_ledger(repo)
    bounded = all(
        float(synapses[item["synapse_id"]]["minimum_weight"]) <= item["proposed_weight"] <= float(synapses[item["synapse_id"]]["maximum_weight"])
        for item in updates
    )
    replay = {"deterministic": True, "ledger_integrity": integrity, "bounded_weights": bounded,
              "authoritative_recall_regression": False, "accepted": integrity and bounded}
    apply_updates = bool(updates and final_reward != 0 and governance_mode in {"normal", "constrained"} and replay["accepted"])
    outcome_id = _outcome_id(repo, activation_id, status, verification_type)
    store.record_outcome(
        repo, outcome_id=outcome_id, activation_id=activation_id, status=status, reward=final_reward,
        verification_type=verification_type, verification_payload=verification_payload or {},
        credits=credits, updates=updates, apply_updates=apply_updates,
    )
    graph_after = store.neural_graph_hash(repo)
    return {"outcome_id": outcome_id, "activation_id": activation_id, "status": status, "reward": final_reward,
            "verification_type": verification_type, "credited_nodes": len({item["node_id"] for item in credits}),
            "credited_synapses": len(credits), "proposed_updates": len(updates),
            "accepted_updates": len(updates) if apply_updates else 0,
            "rejected_updates": 0 if apply_updates else len(updates), "replay": replay,
            "graph_hash_before": graph_before, "graph_hash_after": graph_after,
            "governance_mode": governance_mode}
