from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any


@dataclass(frozen=True)
class NeuralNode:
    node_id: str
    path: str
    kind: str
    threshold: float
    tags: tuple[str, ...] = ()
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class NeuralSynapse:
    synapse_id: str
    source_id: str
    target_id: str
    relation: str
    base_weight: float
    weight: float
    minimum_weight: float = 0.05
    maximum_weight: float = 0.98
    plasticity_rule: str = "bounded_hebbian"
    update_count: int = 0
    evidence: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class NeuralActivationRecord:
    node_id: str
    path: str
    potential: float
    threshold: float
    fired: bool
    depth: int
    source_id: str | None
    relation: str | None

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class NeuralActivationPacket:
    activation_id: str
    repo: str
    task_hash: str
    graph_hash: str
    state_hash: str
    seed_paths: tuple[str, ...]
    fired_paths: tuple[str, ...]
    support_paths: tuple[str, ...]
    records: tuple[NeuralActivationRecord, ...]
    metrics: dict[str, Any]
    plasticity_updates: tuple[dict[str, Any], ...] = ()
    traversed_synapses: tuple[str, ...] = ()

    def to_dict(self) -> dict[str, Any]:
        return {
            "activation_id": self.activation_id,
            "repo": self.repo,
            "task_hash": self.task_hash,
            "graph_hash": self.graph_hash,
            "state_hash": self.state_hash,
            "seed_paths": list(self.seed_paths),
            "fired_paths": list(self.fired_paths),
            "support_paths": list(self.support_paths),
            "records": [record.to_dict() for record in self.records],
            "metrics": self.metrics,
            "plasticity_updates": list(self.plasticity_updates),
            "traversed_synapses": list(self.traversed_synapses),
        }
