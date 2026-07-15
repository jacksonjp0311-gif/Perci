from __future__ import annotations

from pathlib import Path
from typing import Any

from .config import load_repo_config
from .indexer import current_manifest_hash
from .verify import verify_repository


def health_report(home: Path, store: Any, governor: Any, repo: str) -> dict[str, Any]:
    repository = store.repo(repo)
    if not repository:
        raise ValueError(f"Unknown repository: {repo}. Run cortex bootstrap first.")
    root = Path(repository["path"])
    config = load_repo_config(root)
    current = current_manifest_hash(root, config) == (repository["manifest_hash"] or "")
    certificate = verify_repository(home, store, repo, config, write_certificate=False)
    drift = "current" if current else "source_or_configuration_drift"
    vectors = store.vector_format_status(repo)
    command = "cortex activate --repo {0} --task \"<task>\" --refresh packet-fast --json".format(repo)
    if not current:
        command = "cortex activate --repo {0} --task \"<task>\" --refresh packet-refresh --json".format(repo)
    elif vectors["legacy_or_invalid"]:
        command = f"cortex migrate-vectors --repo {repo} --json"
    return {
        "schema_version": "1.0",
        "repo": repo,
        "certificate_status": certificate["status"],
        "governor": governor.evaluate(repo, manifest_current=current, certificate=certificate),
        "drift": {"classification": drift, "manifest_current": current, "volatile_surfaces_excluded": True},
        "vectors": vectors,
        "recommended_next_command": command,
        "claim_boundary": "Health is local operational telemetry; it grants no mutation authority.",
    }
