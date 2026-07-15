from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path
from typing import Any

from .models import NeuralNode, NeuralSynapse


RELATION_PRIORS: dict[str, float] = {
    "resolves_to": 1.00,
    "tested_by": 0.95,
    "co_changed": 0.90,
    "described_by": 0.78,
    "imports": 0.72,
    "references": 0.66,
    "documents": 0.62,
    "calls": 0.56,
}

REVERSE_RELATIONS = {"tested_by", "co_changed", "described_by"}


def _node_threshold(kind: str, authoritative: bool) -> float:
    if authoritative:
        return 0.46
    return {
        "source": 0.52,
        "test": 0.54,
        "documentation": 0.58,
        "configuration": 0.56,
        "runtime_evidence": 0.60,
        "discovery_card": 0.57,
        "telemetry": 0.61,
    }.get(kind, 0.58)


def _node_tags(path: str, kind: str, language: str, authoritative: bool) -> tuple[str, ...]:
    parts = [part.lower() for part in Path(path).parts]
    tags = {kind, language, *parts[:-1]}
    suffix = Path(path).suffix.lower().lstrip(".")
    if suffix:
        tags.add(suffix)
    if authoritative:
        tags.add("authoritative")
    return tuple(sorted(tag for tag in tags if tag))


def _synapse_id(source: str, target: str, relation: str) -> str:
    material = f"{source}|{target}|{relation}"
    return "syn_" + sha256(material.encode("utf-8")).hexdigest()[:24]


def _normalized_endpoint(value: str) -> str:
    return value.split("::", 1)[0].replace("\\", "/")


def compile_interlink(store: Any, repo: str) -> dict[str, Any]:
    """Compile Cortex's existing repository graph into a sparse neural interlink.

    This does not create a second memory store. File records become nodes and existing
    structural/temporal edges become bounded synapses in the same Cortex database.
    """

    file_rows = [row for row in store.files(repo) if row["status"] == "indexed"]
    live_paths = {row["path"] for row in file_rows}
    nodes: list[NeuralNode] = []
    for row in file_rows:
        metadata = json.loads(row["metadata"] or "{}")
        node = NeuralNode(
            node_id=row["path"],
            path=row["path"],
            kind=row["kind"],
            threshold=_node_threshold(row["kind"], bool(row["authoritative"])),
            tags=_node_tags(
                row["path"],
                row["kind"],
                row["language"],
                bool(row["authoritative"]),
            ),
            metadata={
                "language": row["language"],
                "authoritative": bool(row["authoritative"]),
                "content_hash": row["content_hash"],
                **metadata,
            },
        )
        nodes.append(node)

    compiled: dict[tuple[str, str, str], NeuralSynapse] = {}
    for edge in store.edges(repo, limit=200_000):
        relation = edge["relation"]
        if relation not in RELATION_PRIORS:
            continue
        source = _normalized_endpoint(edge["source"])
        target = _normalized_endpoint(edge["target"])
        if source not in live_paths or target not in live_paths or source == target:
            continue
        base = max(0.05, min(0.95, float(edge["confidence"]) * RELATION_PRIORS[relation]))
        key = (source, target, relation)
        existing = compiled.get(key)
        if existing is None or base > existing.base_weight:
            compiled[key] = NeuralSynapse(
                synapse_id=_synapse_id(source, target, relation),
                source_id=source,
                target_id=target,
                relation=relation,
                base_weight=round(base, 6),
                weight=round(base, 6),
                evidence=edge["evidence"],
                metadata=json.loads(edge["metadata"] or "{}"),
            )
        if relation in REVERSE_RELATIONS:
            reverse_relation = f"reverse:{relation}"
            reverse_base = max(0.05, min(0.85, base * 0.82))
            reverse_key = (target, source, reverse_relation)
            if reverse_key not in compiled:
                compiled[reverse_key] = NeuralSynapse(
                    synapse_id=_synapse_id(target, source, reverse_relation),
                    source_id=target,
                    target_id=source,
                    relation=reverse_relation,
                    base_weight=round(reverse_base, 6),
                    weight=round(reverse_base, 6),
                    evidence=f"reverse of {relation}: {edge['evidence']}",
                    metadata={"derived_reverse": True},
                )

    store.sync_neural_graph(repo, nodes, list(compiled.values()))
    state = neural_graph_state(store, repo)
    store.append_neural_event(
        repo,
        event_type="interlink_compiled",
        entity_id=repo,
        payload={
            "nodes": state["nodes"],
            "synapses": state["synapses"],
            "graph_hash": state["graph_hash"],
        },
    )
    return state


def neural_graph_state(store: Any, repo: str) -> dict[str, Any]:
    nodes = store.neural_nodes(repo)
    synapses = store.neural_synapses(repo)
    material = {
        "nodes": [
            {
                "node_id": row["node_id"],
                "path": row["path"],
                "threshold": row["threshold"],
                "kind": row["kind"],
            }
            for row in nodes
        ],
        "synapses": [
            {
                "synapse_id": row["synapse_id"],
                "source_id": row["source_id"],
                "target_id": row["target_id"],
                "relation": row["relation"],
                "base_weight": row["base_weight"],
                "weight": row["weight"],
                "update_count": row["update_count"],
            }
            for row in synapses
        ],
    }
    canonical = json.dumps(material, sort_keys=True, separators=(",", ":"))
    graph_hash = sha256(canonical.encode("utf-8")).hexdigest()
    indexed_count = sum(row["status"] == "indexed" for row in store.files(repo))
    coverage = len(nodes) / indexed_count if indexed_count else 1.0
    return {
        "repo": repo,
        "nodes": len(nodes),
        "synapses": len(synapses),
        "node_coverage": round(coverage, 6),
        "graph_hash": graph_hash,
        "ledger_valid": store.verify_neural_ledger(repo),
    }
