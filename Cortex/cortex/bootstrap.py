from __future__ import annotations

import hashlib
import sys
import time
import uuid
from pathlib import Path
from typing import Any

from .config import RepoConfig, load_repo_config, repo_config_path
from .environment import learn_environment
from .graph import resolve_graph
from .indexer import index_repository
from .integration import install_integration
from .neuron import compile_interlink
from .telemetry import ingest_git
from .verify import verify_repository


def stable_repository_id(root: Path) -> str:
    normalized = str(root.resolve()).replace("\\", "/").lower()
    return hashlib.sha256(normalized.encode("utf-8")).hexdigest()[:24]


def bootstrap_repository(
    home: Path,
    store: Any,
    root: Path,
    name: str | None = None,
    *,
    force: bool = False,
) -> dict[str, Any]:
    root = root.expanduser().resolve()
    if not root.exists() or not root.is_dir():
        raise FileNotFoundError(f"Repository directory not found: {root}")
    repository_name = name or root.name
    repository_id = stable_repository_id(root)
    if repo_config_path(root).exists():
        config = load_repo_config(root)
        config.repository_name = repository_name
        config.repository_id = repository_id
    else:
        config = RepoConfig(
            repository_name=repository_name,
            repository_id=repository_id,
        )

    config.engine_python = str(Path(sys.executable))
    engine_root = Path(__file__).resolve().parent.parent
    config.engine_module_root = str(engine_root)
    config.cortex_home = str(home.resolve())
    try:
        embedded_relative = engine_root.relative_to(root).as_posix()
    except ValueError:
        embedded_relative = ""
    if embedded_relative and embedded_relative != "." and embedded_relative not in config.exclude:
        config.exclude.append(embedded_relative)
    run_id = f"bootstrap-{int(time.time())}-{uuid.uuid4().hex[:8]}"
    store.attach(repository_name, repository_id, root)
    store.begin_bootstrap(run_id, repository_name)

    try:
        integration = install_integration(root, config)
        index = index_repository(store, repository_name, config, force=force)
        graph = resolve_graph(store, repository_name)
        telemetry = ingest_git(store, repository_name, root, config.git_commit_limit)
        environment = (
            learn_environment(root, store, repository_name)
            if config.environment_learning_enabled
            else {"available": False, "disabled": True}
        )
        neural = (
            compile_interlink(store, repository_name)
            if config.neural_interlink_enabled
            else {"available": False, "disabled": True}
        )
        certificate = verify_repository(home, store, repository_name, config, write_certificate=True)
        store.finish_bootstrap(run_id, certificate["status"], index["manifest_hash"], certificate)
        return {
            "run_id": run_id,
            "repo": repository_name,
            "repository_id": repository_id,
            "root": str(root),
            "integration": integration,
            "index": index,
            "graph": graph,
            "telemetry": telemetry,
            "environment": environment,
            "neural_interlink": neural,
            "certificate": certificate,
            "next_command": {
                "powershell": '.cortex\\bin\\cortex.ps1 activate -Task "<current task>"',
                "bash": './.cortex/bin/cortex.sh activate --task "<current task>"',
            },
        }
    except Exception as exc:
        failure = {
            "schema_version": "1.0",
            "status": "failed",
            "repo": repository_name,
            "run_id": run_id,
            "error": f"{type(exc).__name__}: {exc}",
            "failed_at": time.time(),
        }
        store.finish_bootstrap(run_id, "failed", "", failure)
        store.update_repo_state(repository_name, bootstrap_status="failed")
        raise
