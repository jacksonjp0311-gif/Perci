from __future__ import annotations

from collections import defaultdict
from pathlib import Path
from typing import Any


def _python_module_map(paths: set[str]) -> dict[str, str]:
    mapping: dict[str, str] = {}
    for path in paths:
        if not path.endswith((".py", ".pyi")):
            continue
        parts = list(Path(path).with_suffix("").parts)
        if parts and parts[-1] == "__init__":
            parts = parts[:-1]
        if parts:
            mapping[".".join(parts)] = path
    return mapping


def _resolve_relative(source: str, target: str, paths: set[str]) -> str | None:
    source_parent = Path(source).parent
    candidates = [
        (source_parent / target).as_posix(),
        (source_parent / target).with_suffix(".py").as_posix(),
        (source_parent / target).with_suffix(".js").as_posix(),
        (source_parent / target).with_suffix(".ts").as_posix(),
        (source_parent / target / "index.js").as_posix(),
        (source_parent / target / "index.ts").as_posix(),
    ]
    for candidate in candidates:
        normalized = str(Path(candidate)).replace("\\", "/")
        if normalized in paths:
            return normalized
    return None


def resolve_graph(store: Any, repo: str) -> dict[str, Any]:
    files = store.files(repo)
    paths = {row["path"] for row in files if row["status"] == "indexed"}
    module_map = _python_module_map(paths)
    raw_edges = store.edges(repo, limit=100_000)
    resolved = 0
    unresolved = 0

    store.clear_edges(repo, ["resolves_to", "tested_by", "described_by"])

    for row in raw_edges:
        if row["relation"] not in {"imports", "documents", "references"}:
            continue
        source = row["source"].split("::", 1)[0]
        target = row["target"].strip()
        resolved_target: str | None = None

        if target in paths:
            resolved_target = target
        elif row["relation"] == "imports":
            clean = target.lstrip(".").replace("/", ".")
            resolved_target = module_map.get(clean)
            if not resolved_target:
                for module, path in module_map.items():
                    if module.endswith(clean) or clean.endswith(module):
                        resolved_target = path
                        break
            if not resolved_target:
                resolved_target = _resolve_relative(source, target, paths)
        else:
            resolved_target = _resolve_relative(source, target, paths)

        if resolved_target:
            store.add_edge(repo, {
                "source": source,
                "target": resolved_target,
                "relation": "resolves_to",
                "confidence": 0.90,
                "evidence": f"{row['relation']}:{row['evidence']}",
                "metadata": {"original_target": target},
            })
            resolved += 1
        else:
            unresolved += 1

    source_paths = [row["path"] for row in files if row["kind"] == "source"]
    test_paths = [row["path"] for row in files if row["kind"] == "test"]
    for test_path in test_paths:
        test_stem = Path(test_path).stem.lower().removeprefix("test_").removesuffix("_test")
        for source_path in source_paths:
            source_stem = Path(source_path).stem.lower()
            if test_stem and (test_stem == source_stem or test_stem in source_stem or source_stem in test_stem):
                store.add_edge(repo, {
                    "source": source_path,
                    "target": test_path,
                    "relation": "tested_by",
                    "confidence": 0.75,
                    "evidence": "filename affinity",
                })

    docs = [row["path"] for row in files if row["kind"] == "documentation"]
    for doc in docs:
        doc_name = Path(doc).stem.lower()
        for source_path in source_paths:
            source_name = Path(source_path).stem.lower()
            if source_name and source_name != "__init__" and source_name in doc_name:
                store.add_edge(repo, {
                    "source": source_path,
                    "target": doc,
                    "relation": "described_by",
                    "confidence": 0.60,
                    "evidence": "filename affinity",
                })

    store.commit()
    relation_counts: dict[str, int] = defaultdict(int)
    for row in store.edges(repo, limit=100_000):
        relation_counts[row["relation"]] += 1
    return {
        "repo": repo,
        "indexed_nodes": len(paths),
        "symbols": len(store.symbols(repo)),
        "edges": sum(relation_counts.values()),
        "relation_counts": dict(sorted(relation_counts.items())),
        "resolved_references": resolved,
        "unresolved_references": unresolved,
        "resolution_rate": round(resolved / (resolved + unresolved), 6) if resolved + unresolved else 1.0,
    }


def neighborhood(store: Any, repo: str, paths: list[str], limit: int = 30) -> list[dict[str, Any]]:
    seen: set[tuple[str, str, str]] = set()
    output: list[dict[str, Any]] = []
    for path in paths:
        rows = [*store.edges(repo, source=path, limit=limit), *store.edges(repo, target=path, limit=limit)]
        for row in rows:
            key = (row["source"], row["target"], row["relation"])
            if key in seen:
                continue
            seen.add(key)
            output.append({
                "source": row["source"],
                "target": row["target"],
                "relation": row["relation"],
                "confidence": row["confidence"],
                "evidence": row["evidence"],
            })
            if len(output) >= limit:
                return output
    return output
