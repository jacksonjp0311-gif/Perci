from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .config import load_repo_config
from .context import build_context
from .environment import learn_environment
from .graph import resolve_graph
from .hippocampus import begin_session
from .indexer import current_manifest_hash, index_repository
from .neuron import compile_interlink, neural_graph_state
from .telemetry import ingest_git
from .verify import verify_repository


def activate_repository(
    home: Path,
    store: Any,
    governor: Any,
    repo: str,
    task: str,
    budget: int = 1200,
    refresh: str = "auto",
) -> dict[str, Any]:
    repository = store.repo(repo)
    if not repository:
        raise ValueError(f"Unknown repository: {repo}. Run cortex bootstrap first.")
    root = Path(repository["path"])
    config = load_repo_config(root)
    observed_manifest = current_manifest_hash(root, config)
    manifest_current = observed_manifest == (repository["manifest_hash"] or "")
    refresh_result: dict[str, Any] | None = None

    if refresh == "always" or (refresh == "auto" and not manifest_current):
        refresh_result = index_repository(store, repo, config, force=False)
        resolve_graph(store, repo)
        ingest_git(store, repo, root, config.git_commit_limit)
        environment = (
            learn_environment(root, store, repo)
            if config.environment_learning_enabled
            else {"available": False, "disabled": True}
        )
        neural = (
            compile_interlink(store, repo)
            if config.neural_interlink_enabled
            else {"available": False, "disabled": True}
        )
        certificate = verify_repository(home, store, repo, config, write_certificate=True)
        manifest_current = certificate["manifest"]["current"]
    else:
        environment = store.environment_profile(repo)
        if config.environment_learning_enabled and not environment:
            environment = learn_environment(root, store, repo)
        if not config.environment_learning_enabled:
            environment = {"available": False, "disabled": True}
        if config.neural_interlink_enabled and not store.neural_nodes(repo):
            neural = compile_interlink(store, repo)
        elif config.neural_interlink_enabled:
            neural = neural_graph_state(store, repo)
        else:
            neural = {"available": False, "disabled": True}
        certificate = verify_repository(home, store, repo, config, write_certificate=False)

    context = build_context(
        home,
        store,
        governor,
        repo,
        task,
        budget,
        manifest_current=manifest_current,
        certificate=certificate,
    )
    session = begin_session(home, store, repo, task)
    runtime_path = root / ".cortex" / "runtime" / "context_latest.json"
    runtime_path.parent.mkdir(parents=True, exist_ok=True)
    runtime_path.write_text(json.dumps(context, indent=2) + "\n", encoding="utf-8")

    return {
        "activation": "ready" if certificate["status"] == "verified" else "read_only",
        "repo": repo,
        "task": task,
        "bootstrap_status": certificate["status"],
        "manifest_current": manifest_current,
        "refresh": refresh_result,
        "environment": environment,
        "neural_interlink": neural,
        "session": session,
        "context": context,
        "runtime_packet": str(runtime_path),
    }
