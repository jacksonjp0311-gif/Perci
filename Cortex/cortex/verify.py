from __future__ import annotations

import hashlib
import json
import re
import time
from pathlib import Path
from typing import Any

from .config import RepoConfig
from .indexer import current_manifest_hash
from .integration import integration_status
from .neuron import neural_graph_state
from .retrieval import query

HEADING_RE = re.compile(r"^#{1,3}\s+(.+)$", re.MULTILINE)


def _coverage(store: Any, repo: str) -> dict[str, Any]:
    rows = store.files(repo)
    eligible_states = {"indexed", "failed"}
    eligible = [row for row in rows if row["status"] in eligible_states]
    indexed = [row for row in rows if row["status"] == "indexed"]
    authoritative = [row for row in rows if row["authoritative"]]
    authoritative_indexed = [row for row in authoritative if row["status"] == "indexed"]
    return {
        "inventory_count": len(rows),
        "eligible_count": len(eligible),
        "indexed_count": len(indexed),
        "failed_count": sum(row["status"] == "failed" for row in rows),
        "unsupported_count": sum(row["status"] in {"unsupported", "binary", "oversized", "unreadable"} for row in rows),
        "index_coverage": len(indexed) / len(eligible) if eligible else 1.0,
        "authoritative_count": len(authoritative),
        "authoritative_indexed": len(authoritative_indexed),
        "authoritative_coverage": len(authoritative_indexed) / len(authoritative) if authoritative else 1.0,
        "unresolved_files": [
            {"path": row["path"], "status": row["status"], "metadata": json.loads(row["metadata"] or "{}")}
            for row in rows if row["status"] != "indexed"
        ][:200],
    }


def _retrieval_probes(store: Any, repo: str, root: Path) -> dict[str, Any]:
    probes: list[tuple[str, str | None]] = []
    readme = root / "README.md"
    if readme.exists():
        text = readme.read_text(encoding="utf-8", errors="replace")
        headings = [heading.strip("`* ") for heading in HEADING_RE.findall(text)]
        for heading in headings[:3]:
            if len(heading) >= 4:
                probes.append((heading, "README.md"))
    for symbol in store.symbols(repo)[:5]:
        probes.append((symbol["name"], symbol["path"]))
    if not probes:
        probes.append((repo, None))

    results: list[dict[str, Any]] = []
    for text, expected_path in probes[:8]:
        hits = query(store, repo, text, limit=5)
        paths = [hit.path for hit in hits]
        passed = bool(hits) and (expected_path is None or expected_path in paths)
        results.append({
            "query": text,
            "expected_path": expected_path,
            "returned_paths": paths,
            "passed": passed,
        })
    pass_rate = sum(result["passed"] for result in results) / len(results) if results else 0.0
    return {"probe_count": len(results), "pass_rate": pass_rate, "results": results}


def verify_repository(
    home: Path,
    store: Any,
    repo: str,
    config: RepoConfig,
    *,
    write_certificate: bool = True,
) -> dict[str, Any]:
    repository = store.repo(repo)
    if not repository:
        raise ValueError(f"Unknown repository: {repo}")
    root = Path(repository["path"])
    stored_manifest = repository["manifest_hash"] or ""
    observed_manifest = current_manifest_hash(root, config)
    manifest_current = bool(stored_manifest) and stored_manifest == observed_manifest
    coverage = _coverage(store, repo)
    probes = _retrieval_probes(store, repo, root)
    integration = integration_status(root)
    graph_edges = store.edges(repo, limit=100_000)
    relation_counts: dict[str, int] = {}
    for edge in graph_edges:
        relation_counts[edge["relation"]] = relation_counts.get(edge["relation"], 0) + 1
    telemetry = {
        "git_commits": len(store.commits(repo, 100_000)),
        "files_with_history": len(store.file_telemetry(repo)),
        "available": bool(store.commits(repo, 1)),
    }
    environment = store.environment_profile(repo)
    neural = neural_graph_state(store, repo) if config.neural_interlink_enabled else {
        "disabled": True, "nodes": 0, "synapses": 0, "node_coverage": 1.0,
        "graph_hash": None, "ledger_valid": True,
    }

    thresholds = config.bootstrap_thresholds
    checks = {
        "database_integrity": store.integrity_check(),
        "integration_complete": integration["complete"],
        "manifest_integrity": manifest_current,
        "index_coverage": coverage["index_coverage"] >= thresholds.get("index_coverage", 0.98),
        "authoritative_coverage": coverage["authoritative_coverage"] >= 0.98,
        "retrieval_probes": probes["pass_rate"] >= thresholds.get("retrieval_probe_pass_rate", 0.75),
        "environment_profile": (not config.environment_learning_enabled) or bool(environment),
        "neural_node_coverage": (not config.neural_interlink_enabled) or neural["node_coverage"] >= 0.98,
        "neural_ledger_integrity": (not config.neural_interlink_enabled) or neural["ledger_valid"],
    }
    required_pass = all(checks.values())
    if required_pass:
        status = "verified"
    elif checks["database_integrity"] and checks["integration_complete"] and coverage["index_coverage"] >= 0.75:
        status = "degraded"
    else:
        status = "failed"

    certificate: dict[str, Any] = {
        "schema_version": "1.1",
        "certificate_type": "cortex_neural_repository_assimilation",
        "status": status,
        "issued_at": time.time(),
        "repository": {
            "name": repo,
            "repository_id": repository["repository_id"],
            "path": str(root),
        },
        "manifest": {
            "stored_hash": stored_manifest,
            "observed_hash": observed_manifest,
            "current": manifest_current,
        },
        "coverage": coverage,
        "integration": integration,
        "graph": {
            "symbols": len(store.symbols(repo)),
            "edges": len(graph_edges),
            "relation_counts": relation_counts,
        },
        "telemetry": telemetry,
        "environment": {
            "available": bool(environment),
            "profile_hash": (environment or {}).get("profile_hash"),
            "ecosystems": (environment or {}).get("ecosystems", []),
            "frameworks": (environment or {}).get("frameworks", []),
        },
        "neural_interlink": neural,
        "retrieval_validation": probes,
        "checks": checks,
        "thresholds": thresholds,
        "claim_boundary": (
            "This certificate verifies inventory, supported-content indexing, environment profiling, "
            "relationship extraction, neural interlink compilation, integration files, and retrieval probes "
            "at the recorded manifest. It does not prove program "
            "correctness, security, safety, semantic completeness, or authorization to mutate the repository."
        ),
        "retrieval_enabled": status == "verified",
    }
    canonical = json.dumps(certificate, sort_keys=True, separators=(",", ":"))
    certificate["certificate_hash"] = hashlib.sha256(canonical.encode("utf-8")).hexdigest()

    if write_certificate:
        repo_certificate = root / ".cortex" / "bootstrap_certificate.json"
        repo_certificate.parent.mkdir(parents=True, exist_ok=True)
        repo_certificate.write_text(json.dumps(certificate, indent=2) + "\n", encoding="utf-8")
        home_certificate = home / "certificates" / f"{repo}-latest.json".replace("/", "_")
        home_certificate.write_text(json.dumps(certificate, indent=2) + "\n", encoding="utf-8")
        store.update_repo_state(repo, bootstrap_status=status, bootstrapped=True)
        store.set_setting(f"certificate:{repo}", certificate)
    return certificate
