from __future__ import annotations

import hashlib
import subprocess
from collections import Counter, defaultdict
from itertools import combinations
from pathlib import Path
from typing import Any

from .embeddings import get_embedder

HEADER = "@@CORTEX_COMMIT@@"
SEP = "\x1f"


def _git(root: Path, args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *args],
        text=True,
        encoding="utf-8",
        errors="replace",
        capture_output=True,
        check=False,
        timeout=60,
    )


def ingest_git(store: Any, repo: str, root: Path, limit: int = 500) -> dict[str, Any]:
    inside = _git(root, ["rev-parse", "--is-inside-work-tree"])
    if inside.returncode != 0 or inside.stdout.strip() != "true":
        return {"available": False, "reason": "not a Git work tree", "commits_ingested": 0}

    git_root = _git(root, ["rev-parse", "--show-toplevel"])
    if git_root.returncode != 0 or Path(git_root.stdout.strip()).resolve() != root.resolve():
        return {
            "available": False,
            "reason": "target directory is not the Git work-tree root; telemetry was not imported",
            "commits_ingested": 0,
        }

    head_result = _git(root, ["rev-parse", "HEAD"])
    head = head_result.stdout.strip() if head_result.returncode == 0 else ""
    log = _git(
        root,
        [
            "log", f"-n{limit}", "--no-renames", "--numstat",
            f"--format={HEADER}%H{SEP}%at{SEP}%an{SEP}%s",
        ],
    )
    if log.returncode != 0:
        return {"available": False, "reason": log.stderr.strip(), "commits_ingested": 0}

    store.clear_edges(repo, ["co_changed"])
    store.db.execute("DELETE FROM git_commits WHERE repo=?", (repo,))
    store.db.execute("DELETE FROM file_telemetry WHERE repo=?", (repo,))

    commits: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    for raw_line in log.stdout.splitlines():
        line = raw_line.rstrip("\n")
        if line.startswith(HEADER):
            if current:
                commits.append(current)
            fields = line[len(HEADER):].split(SEP, 3)
            if len(fields) != 4:
                current = None
                continue
            commit_hash, authored_at, author, subject = fields
            current = {
                "commit_hash": commit_hash,
                "authored_at": float(authored_at),
                "author": author,
                "subject": subject,
                "files": [],
            }
            continue
        if current and line and "\t" in line:
            additions, deletions, path = line.split("\t", 2)
            current["files"].append({
                "path": path,
                "additions": int(additions) if additions.isdigit() else 0,
                "deletions": int(deletions) if deletions.isdigit() else 0,
            })
    if current:
        commits.append(current)

    stats: dict[str, dict[str, Any]] = defaultdict(lambda: {
        "commit_count": 0,
        "additions": 0,
        "deletions": 0,
        "last_changed": None,
    })
    cochange: Counter[tuple[str, str]] = Counter()

    for commit in commits:
        store.add_commit(repo, commit)
        paths = [entry["path"] for entry in commit["files"]]
        for entry in commit["files"]:
            path = entry["path"]
            stat = stats[path]
            stat["commit_count"] += 1
            stat["additions"] += entry["additions"]
            stat["deletions"] += entry["deletions"]
            stat["last_changed"] = max(stat["last_changed"] or 0, commit["authored_at"])
        if len(paths) <= 50:
            for left, right in combinations(sorted(set(paths)), 2):
                cochange[(left, right)] += 1

    degree: Counter[str] = Counter()
    for (left, right), count in cochange.items():
        if count < 2:
            continue
        degree[left] += 1
        degree[right] += 1
        store.add_edge(repo, {
            "source": left,
            "target": right,
            "relation": "co_changed",
            "confidence": min(0.95, 0.45 + count / 20.0),
            "evidence": f"{count} shared commits",
            "metadata": {"shared_commit_count": count},
        })

    for path, stat in stats.items():
        stat["cochange_degree"] = degree[path]
        store.set_file_telemetry(repo, path, stat)

    top_files = sorted(stats.items(), key=lambda item: item[1]["commit_count"], reverse=True)[:25]
    summary_lines = [
        f"Repository Git head: {head}",
        f"Commits analyzed: {len(commits)}",
        "Most frequently changed files:",
    ]
    summary_lines.extend(
        f"- {path}: {stat['commit_count']} commits, +{stat['additions']}/-{stat['deletions']}"
        for path, stat in top_files
    )
    summary = "\n".join(summary_lines)
    summary_hash = hashlib.sha256(summary.encode("utf-8")).hexdigest()
    store.remove_path(repo, ".cortex/telemetry/git-summary")
    embedder = get_embedder()
    store.upsert_memory(
        repo=repo,
        path=".cortex/telemetry/git-summary",
        chunk_index=0,
        start_line=1,
        end_line=len(summary_lines),
        kind="telemetry",
        text=summary,
        content_hash=summary_hash,
        vector=embedder.encode_one(summary),
        embedding_model=embedder.name,
        metadata={"generated": True, "git_head": head, "commit_limit": limit},
    )
    store.commit()
    store.set_setting(f"repo:{repo}:git_head", head)
    return {
        "available": True,
        "git_head": head,
        "commits_ingested": len(commits),
        "files_with_history": len(stats),
        "cochange_links": sum(1 for count in cochange.values() if count >= 2),
        "history_truncated": len(commits) >= limit,
    }


def current_git_head(root: Path) -> str | None:
    result = _git(root, ["rev-parse", "HEAD"])
    return result.stdout.strip() if result.returncode == 0 else None
