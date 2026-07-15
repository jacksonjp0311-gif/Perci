from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PlasticityProposal:
    synapse_id: str
    old_weight: float
    proposed_weight: float
    delta: float
    reason: str

    def to_dict(self) -> dict[str, float | str]:
        return {
            "synapse_id": self.synapse_id,
            "old_weight": self.old_weight,
            "proposed_weight": self.proposed_weight,
            "delta": self.delta,
            "reason": self.reason,
        }


def bounded_hebbian(
    *,
    synapse_id: str,
    weight: float,
    minimum_weight: float,
    maximum_weight: float,
    pre: float,
    post: float,
    learning_rate: float,
) -> PlasticityProposal:
    """Move a weight toward its upper bound according to bounded co-activation."""

    raw_delta = learning_rate * max(0.0, pre) * max(0.0, post) * (maximum_weight - weight)
    proposed = min(maximum_weight, max(minimum_weight, weight + raw_delta))
    return PlasticityProposal(
        synapse_id=synapse_id,
        old_weight=weight,
        proposed_weight=proposed,
        delta=proposed - weight,
        reason="bounded_hebbian_coactivation",
    )
