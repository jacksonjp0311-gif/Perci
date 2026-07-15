from __future__ import annotations

from collections import defaultdict
from hashlib import sha256
import json
from math import tanh
from typing import Any, Iterable

from .models import NeuralActivationPacket, NeuralActivationRecord


def _task_hash(task: str) -> str:
    return sha256(task.encode("utf-8")).hexdigest()


def _activation_id(repo: str, task_hash: str, state_hash: str) -> str:
    return "act_" + sha256(f"{repo}|{task_hash}|{state_hash}".encode("utf-8")).hexdigest()[:24]


def _seed_strengths(hits: Iterable[Any]) -> dict[str, float]:
    hit_list = list(hits)
    if not hit_list:
        return {}
    maximum = max(float(hit.score) for hit in hit_list) or 1.0
    strengths: dict[str, float] = {}
    for hit in hit_list:
        normalized = max(0.0, min(1.0, float(hit.score) / maximum))
        semantic = max(0.0, float(hit.metadata.get("semantic_similarity", 0.0)))
        authoritative = 0.08 if hit.metadata.get("authoritative") else 0.0
        excitation = min(1.0, 0.46 + 0.38 * normalized + 0.18 * semantic + authoritative)
        strengths[hit.path] = max(strengths.get(hit.path, 0.0), excitation)
    return strengths


def activate_interlink(
    store: Any,
    repo: str,
    task: str,
    hits: Iterable[Any],
    *,
    max_depth: int = 2,
    max_nodes: int = 64,
    learning_rate: float = 0.025,
    plasticity_enabled: bool = True,
    governance_mode: str = "read_only",
    session_id: str | None = None,
) -> NeuralActivationPacket:
    """Run deterministic sparse spreading activation over Cortex's compiled graph."""

    node_rows = store.neural_nodes(repo)
    node_map = {row["node_id"]: row for row in node_rows}
    graph_hash = store.neural_graph_hash(repo)
    seeds = {path: value for path, value in _seed_strengths(hits).items() if path in node_map}
    synapses = store.neural_synapses(repo)
    outgoing: dict[str, list[Any]] = defaultdict(list)
    for row in synapses:
        outgoing[row["source_id"]].append(row)
    for source in outgoing:
        outgoing[source].sort(key=lambda row: (-float(row["weight"]), row["target_id"], row["relation"]))

    potentials: dict[str, float] = dict(seeds)
    best_depth: dict[str, int] = {path: 0 for path in seeds}
    provenance: dict[str, tuple[str | None, str | None]] = {path: (None, None) for path in seeds}
    frontier = sorted(seeds, key=lambda path: (-seeds[path], path))[:max_nodes]
    traversed: set[str] = set()
    steps = 0

    for depth in range(max_depth + 1):
        if not frontier:
            break
        next_contributions: dict[str, tuple[float, str, str]] = {}
        for source in frontier:
            source_potential = potentials.get(source, 0.0)
            node = node_map[source]
            threshold = float(node["threshold"])
            if source_potential < threshold:
                continue
            if depth >= max_depth:
                continue
            for synapse in outgoing.get(source, ()):  # already deterministic
                target = synapse["target_id"]
                if target not in node_map:
                    continue
                contribution = source_potential * float(synapse["weight"]) * (0.84 ** (depth + 1))
                if contribution <= 0.01:
                    continue
                traversed.add(synapse["synapse_id"])
                steps += 1
                prior = next_contributions.get(target)
                if prior is None or contribution > prior[0]:
                    next_contributions[target] = (contribution, source, synapse["relation"])
        ranked = sorted(next_contributions.items(), key=lambda item: (-item[1][0], item[0]))
        frontier = []
        for target, (contribution, source, relation) in ranked[:max_nodes]:
            integrated = tanh(potentials.get(target, 0.0) * 0.35 + contribution)
            if integrated > potentials.get(target, 0.0):
                potentials[target] = integrated
                best_depth[target] = depth + 1
                provenance[target] = (source, relation)
                frontier.append(target)

    ranked_nodes = sorted(potentials, key=lambda path: (-potentials[path], best_depth[path], path))[:max_nodes]
    records: list[NeuralActivationRecord] = []
    fired: list[str] = []
    for path in ranked_nodes:
        node = node_map[path]
        potential = potentials[path]
        threshold = float(node["threshold"])
        did_fire = potential >= threshold
        source_id, relation = provenance.get(path, (None, None))
        records.append(
            NeuralActivationRecord(
                node_id=path,
                path=path,
                potential=round(potential, 8),
                threshold=round(threshold, 8),
                fired=did_fire,
                depth=best_depth.get(path, 0),
                source_id=source_id,
                relation=relation,
            )
        )
        if did_fire:
            fired.append(path)

    seed_paths = tuple(sorted(seeds, key=lambda path: (-seeds[path], path)))
    support_paths = tuple(path for path in fired if path not in seeds)
    state_material = {
        "repo": repo,
        "task_hash": _task_hash(task),
        "graph_hash": graph_hash,
        "seeds": [(path, round(seeds[path], 8)) for path in seed_paths],
        "records": [record.to_dict() for record in records],
    }
    state_hash = sha256(
        json.dumps(state_material, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    activation_id = _activation_id(repo, state_material["task_hash"], state_hash)

    # v2: activation is observational. Persistent graph adaptation happens only after
    # a separately recorded, verification-backed outcome has passed replay gates.
    updates: list[dict[str, Any]] = []

    metrics = {
        "total_nodes": len(node_rows),
        "nodes_considered": len(records),
        "nodes_fired": len(fired),
        "support_nodes": len(support_paths),
        "propagation_steps": steps,
        "sparse_activation_ratio": round(len(fired) / max(1, len(node_rows)), 8),
        "considered_fraction": round(len(records) / max(1, len(node_rows)), 8),
        "max_depth": max((record.depth for record in records), default=0),
    }
    packet = NeuralActivationPacket(
        activation_id=activation_id,
        repo=repo,
        task_hash=state_material["task_hash"],
        graph_hash=graph_hash,
        state_hash=state_hash,
        seed_paths=seed_paths,
        fired_paths=tuple(fired),
        support_paths=support_paths,
        records=tuple(records),
        metrics=metrics,
        plasticity_updates=tuple(updates),
        traversed_synapses=tuple(sorted(traversed)),
    )
    payload = packet.to_dict()
    store.record_neural_activation(repo, session_id, payload)
    store.append_neural_event(
        repo,
        event_type="sparse_activation",
        entity_id=activation_id,
        payload={
            "session_id": session_id,
            "task_hash": packet.task_hash,
            "state_hash": state_hash,
            "fired_paths": list(packet.fired_paths),
            "support_paths": list(packet.support_paths),
            "metrics": metrics,
            "plasticity_updates": updates,
            "traversed_synapses": sorted(traversed),
        },
    )
    return packet
