from __future__ import annotations

import hashlib
import json
import time
from pathlib import Path
from typing import Any

from .config import load_repo_config
from .environment import environment_summary
from .efficiency import efficiency_telemetry
from .graph import neighborhood
from .hippocampus import active_session
from .neuron import activate_interlink
from .retrieval import query, support_hits
from thalamus import apply_feedback, inhibit, make_request, route


def estimate_tokens(text: str) -> int:
    return max(1, (len(text) + 3) // 4)


def _merge_candidates(
    direct_hits: list[Any],
    support: list[Any],
    neural_packet: dict[str, Any] | None,
) -> list[Any]:
    scored: dict[int, tuple[float, Any]] = {}
    direct_count = max(1, len(direct_hits))
    for rank, hit in enumerate(direct_hits):
        hit.metadata["selection_source"] = "hybrid_retrieval"
        priority = 1.0 - (0.35 * rank / direct_count)
        scored[hit.memory_id] = (priority, hit)

    potential_by_path: dict[str, float] = {}
    if neural_packet:
        for record in neural_packet.get("records", []):
            if record.get("fired"):
                potential_by_path[record["path"]] = float(record["potential"])
    for hit in support:
        priority = 0.76 + 0.20 * potential_by_path.get(hit.path, 0.0)
        current = scored.get(hit.memory_id)
        if current is None or priority > current[0]:
            scored[hit.memory_id] = (priority, hit)
    return [
        hit
        for _, hit in sorted(
            scored.values(),
            key=lambda item: (-item[0], item[1].path, item[1].start_line),
        )
    ]


def build_context(
    home: Path,
    store: Any,
    governor: Any,
    repo: str,
    task: str,
    budget: int = 1200,
    manifest_current: bool | None = None,
    certificate: dict[str, Any] | None = None,
) -> dict[str, Any]:
    repository = store.repo(repo)
    if not repository:
        raise ValueError(f"Unknown repository: {repo}")
    root = Path(repository["path"])
    config = load_repo_config(root)
    active = active_session(home, repo)
    if config.thalamus_enabled:
        request = make_request(
            repository,
            task,
            budget,
            active_files=tuple((active or {}).get("files", [])),
        )
        route_plan = route(request, manifest_current=manifest_current)
    else:
        route_plan = None

    # Every standard context retrieval is planned by Thalamus before candidates are read.
    direct_hits = query(store, repo, task, limit=24, semantic_scan_limit=config.semantic_scan_limit)
    if route_plan:
        direct_hits = apply_feedback(store, repo, direct_hits)
        direct_hits = inhibit(
            direct_hits,
            route_plan.lane_weights,
            min_lane_relevance=config.thalamus_min_lane_relevance,
        )
    semantic_confidences = [
        hit.metadata.get("semantic_similarity", 0.0) for hit in direct_hits[:5]
    ]
    confidence = sum(max(0.0, value) for value in semantic_confidences) / max(
        1, len(semantic_confidences)
    )
    governance = governor.evaluate(
        repo, retrieval_confidence=confidence, manifest_current=manifest_current, certificate=certificate
    )

    effective_budget = budget
    if governance["mode"] == "constrained":
        effective_budget = min(budget, 800)
    elif governance["mode"] == "read_only":
        effective_budget = min(budget, 600)

    neural_payload: dict[str, Any]
    support: list[Any] = []
    if config.neural_interlink_enabled and store.neural_nodes(repo):
        neural = activate_interlink(
            store,
            repo,
            task,
            direct_hits,
            max_depth=config.neural_activation_depth,
            max_nodes=config.neural_max_nodes,
            learning_rate=config.neural_learning_rate,
            plasticity_enabled=config.neural_plasticity_enabled,
            governance_mode=governance["mode"],
            session_id=(active or {}).get("session_id"),
        )
        neural_payload = neural.to_dict()
        support = support_hits(
            store,
            repo,
            task,
            list(neural.support_paths),
            limit=max(6, min(16, config.neural_max_nodes // 4)),
        )
    else:
        neural_payload = {
            "available": False,
            "reason": "neural interlink disabled or not compiled",
            "records": [],
            "support_paths": [],
            "metrics": {},
        }

    candidates = _merge_candidates(direct_hits, support, neural_payload)
    selected: list[dict[str, Any]] = []
    used_tokens = 0
    for hit in candidates:
        prefix = f"[{hit.path}:{hit.start_line}-{hit.end_line}]\n"
        available_chars = max(0, (effective_budget - used_tokens) * 4 - len(prefix))
        if available_chars <= 80:
            break
        text = hit.text[:available_chars]
        token_cost = estimate_tokens(prefix + text)
        if token_cost <= 0:
            continue
        selected.append(
            {
                "memory_id": hit.memory_id,
                "path": hit.path,
                "line_range": [hit.start_line, hit.end_line],
                "kind": hit.kind,
                "score": hit.score,
                "content_hash": hit.content_hash,
                "text": text,
                "metadata": hit.metadata,
            }
        )
        used_tokens += token_cost
        if used_tokens >= effective_budget:
            break

    graph_context = neighborhood(
        store, repo, [item["path"] for item in selected[:8]], limit=30
    )
    environment = environment_summary(store.environment_profile(repo))
    payload: dict[str, Any] = {
        "schema_version": "1.1",
        "generated_at": time.time(),
        "repository": {
            "name": repo,
            "repository_id": repository["repository_id"],
            "path": repository["path"],
            "manifest_hash": repository["manifest_hash"],
            "bootstrap_status": repository["bootstrap_status"],
        },
        "task": task,
        "active_focus": active,
        "governor": governance,
        "context_budget": effective_budget,
        "estimated_tokens": used_tokens,
        "environment": environment,
        "thalamus": route_plan.to_dict() if route_plan else {"available": False, "reason": "disabled"},
        "neural_interlink": neural_payload,
        "evidence": selected,
        "structural_neighborhood": graph_context,
        "instructions": [
            "Use this packet as the initial context, not as mutation authority.",
            "Start with directly retrieved evidence, then use neural support paths as bounded expansion.",
            "Open full files only when a cited line range is insufficient.",
            "Repository source, current tests, and compiler/runtime evidence outrank generated memory.",
            "Record durable decisions, discoveries, failures, and outcomes before consolidation.",
        ],
    }
    payload["efficiency"] = efficiency_telemetry(
        direct_candidates=len(direct_hits),
        context_tokens=used_tokens,
        context_budget=effective_budget,
        neural=neural_payload,
    )
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    payload["packet_hash"] = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
    packet_path = home / "packets" / f"{repo}-context-latest.json".replace("/", "_")
    packet_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    payload["packet_path"] = str(packet_path)
    return payload


def nexus_packet(context: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "1.1",
        "intent": {
            "task": context["task"],
            "active_focus": context["active_focus"],
        },
        "evidence": context["evidence"],
        "authority": {
            "mode": "recommend_only",
            "human_authorized_only": True,
            "cortex_may_mutate": False,
            "governor_mode": context["governor"]["mode"],
        },
        "context": {
            "repository": context["repository"],
            "environment": context["environment"],
            "thalamus": context.get("thalamus", {"available": False}),
            "structural_neighborhood": context["structural_neighborhood"],
            "neural_interlink": {
                "activation_id": context["neural_interlink"].get("activation_id"),
                "state_hash": context["neural_interlink"].get("state_hash"),
                "fired_paths": context["neural_interlink"].get("fired_paths", []),
                "support_paths": context["neural_interlink"].get("support_paths", []),
                "metrics": context["neural_interlink"].get("metrics", {}),
            },
            "estimated_tokens": context["estimated_tokens"],
            "packet_hash": context["packet_hash"],
        },
    }


def cortex_context_protocol(context: dict[str, Any]) -> dict[str, Any]:
    """Stable, agent-neutral context contract; evidence remains subordinate to source truth."""
    neural = context.get("neural_interlink", {})
    return {
        "protocol": "cortex-context/1.0",
        "repository": context["repository"],
        "task": {"text": context["task"], "packet_hash": context["packet_hash"]},
        "governance": context["governor"],
        "environment": context["environment"],
        "direct_evidence": context["evidence"],
        "support_evidence": [item for item in context["evidence"] if item.get("metadata", {}).get("selection_source") != "hybrid_retrieval"],
        "structural_paths": {"neural_activation_id": neural.get("activation_id"), "support_paths": neural.get("support_paths", [])},
        "discoveries": [],
        "contradictions": [],
        "unknowns": ["No inferred claim is mutation authority; inspect current source and tests."],
        "recommended_commands": [],
        "prohibited_actions": ["Treat learned associations as superior to current source, tests, governance, or human authority."],
        "state_hashes": {"packet": context["packet_hash"], "neural": neural.get("state_hash"), "manifest": context["repository"].get("manifest_hash")},
    }
