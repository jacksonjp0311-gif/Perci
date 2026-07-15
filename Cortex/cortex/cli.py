from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .activation import activate_repository
from .bootstrap import bootstrap_repository
from .bridge import consolidate
from .config import ensure_home, load_repo_config
from .context import build_context, cortex_context_protocol, nexus_packet
from .learning import record_outcome
from .environment import environment_summary
from .governor import Governor
from .health import health_report
from .benchmark import verify_benchmarks
from .graph import neighborhood, resolve_graph
from .hippocampus import begin_session, remember
from .indexer import index_repository
from .neuron import activate_interlink, neural_graph_state
from .retrieval import query
from .store import Store
from .selftest import run_self_test
from .telemetry import ingest_git
from thalamus import apply_feedback, inhibit, make_request, record_feedback, route
from .verify import verify_repository


def emit(value: Any, as_json: bool = False) -> None:
    if as_json or isinstance(value, (dict, list)):
        print(json.dumps(value, indent=2, default=str))
    else:
        print(value)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="cortex",
        description="Repository assimilation and selective memory for AI coding agents.",
    )
    parser.add_argument("--home", help="Override CORTEX_HOME.")
    sub = parser.add_subparsers(dest="command", required=True)

    init = sub.add_parser("init", help="Initialize the global Cortex home and database.")
    init.add_argument("--json", action="store_true")

    bootstrap = sub.add_parser("bootstrap", help="Assimilate and integrate a repository.")
    bootstrap.add_argument("path", nargs="?", default=".")
    bootstrap.add_argument("--name")
    bootstrap.add_argument("--force", action="store_true")
    bootstrap.add_argument("--json", action="store_true")

    activate = sub.add_parser("activate", help="Refresh memory as needed and emit task context.")
    activate.add_argument("--repo", required=True)
    activate.add_argument("--task", required=True)
    activate.add_argument("--budget", type=int, default=1200)
    activate.add_argument("--refresh", choices=["auto", "always", "never", "packet-fast", "packet-refresh", "bootstrap-full"], default="auto")
    activate.add_argument("--json", action="store_true")

    index = sub.add_parser("index", help="Incrementally index an attached repository.")
    index.add_argument("--repo", required=True)
    index.add_argument("--force", action="store_true")
    index.add_argument("--json", action="store_true")

    migrate = sub.add_parser("migrate-vectors", help="Upgrade stored legacy vectors to versioned float32 BLOBs.")
    migrate.add_argument("--repo")
    migrate.add_argument("--json", action="store_true")

    query_parser = sub.add_parser("query", help="Search repository memory.")
    query_parser.add_argument("query")
    query_parser.add_argument("--repo", required=True)
    query_parser.add_argument("--limit", type=int, default=8)
    query_parser.add_argument("--json", action="store_true")

    context = sub.add_parser("context", help="Emit a bounded context packet without starting a session.")
    context.add_argument("--repo", required=True)
    context.add_argument("--task", required=True)
    context.add_argument("--budget", type=int, default=1200)
    context.add_argument("--json", action="store_true")

    nexus = sub.add_parser("nexus-packet", help="Emit NexusGate Intent/Evidence/Authority/Context shape.")
    nexus.add_argument("--repo", required=True)
    nexus.add_argument("--task", required=True)
    nexus.add_argument("--budget", type=int, default=1200)
    nexus.add_argument("--json", action="store_true")

    protocol = sub.add_parser("protocol", help="Emit the stable Cortex Context Protocol packet.")
    protocol.add_argument("--repo", required=True)
    protocol.add_argument("--task", required=True)
    protocol.add_argument("--budget", type=int, default=1200)
    protocol.add_argument("--json", action="store_true")

    outcome = sub.add_parser("outcome", help="Record a verification outcome and replay-gate bounded learning.")
    outcome.add_argument("--repo", required=True)
    outcome.add_argument("--activation-id", required=True)
    outcome.add_argument("--status", choices=["verified", "diagnosed", "helpful", "unknown", "irrelevant", "failed", "unsafe"], required=True)
    outcome.add_argument("--verification", required=True)
    outcome.add_argument("--reward", type=float)
    outcome.add_argument("--json", action="store_true")

    environment = sub.add_parser("environment", help="Show the learned repository environment profile.")
    environment.add_argument("--repo", required=True)
    environment.add_argument("--json", action="store_true")

    thalamus = sub.add_parser("thalamus", help="Inspect the deterministic retrieval route for a task.")
    thalamus.add_argument("--repo", required=True)
    thalamus.add_argument("--task", required=True)
    thalamus.add_argument("--budget", type=int, default=1200)
    thalamus.add_argument("--json", action="store_true")

    feedback = sub.add_parser("thalamus-feedback", help="Record bounded evidence usefulness feedback.")
    feedback.add_argument("--repo", required=True)
    feedback.add_argument("--memory-id", type=int, required=True)
    feedback.add_argument("--outcome", required=True)
    feedback.add_argument("--json", action="store_true")

    interlink = sub.add_parser("interlink", help="Run sparse neural activation for a task.")
    interlink.add_argument("--repo", required=True)
    interlink.add_argument("--task", required=True)
    interlink.add_argument("--limit", type=int, default=24)
    interlink.add_argument("--learn", action="store_true")
    interlink.add_argument("--json", action="store_true")

    replay = sub.add_parser("neural-replay", help="Replay recent neural interlink ledger events.")
    replay.add_argument("--repo", required=True)
    replay.add_argument("--limit", type=int, default=100)
    replay.add_argument("--json", action="store_true")

    focus = sub.add_parser("focus", help="Start an explicit hippocampal session.")
    focus.add_argument("--repo", required=True)
    focus.add_argument("--task", required=True)
    focus.add_argument("--files", nargs="*")
    focus.add_argument("--json", action="store_true")

    remember_parser = sub.add_parser("remember", help="Record a working-memory event.")
    remember_parser.add_argument("--repo", required=True)
    remember_parser.add_argument("--kind", required=True)
    remember_parser.add_argument("--text", required=True)
    remember_parser.add_argument("--session")
    remember_parser.add_argument("--json", action="store_true")

    consolidate_parser = sub.add_parser("consolidate", help="Consolidate a session into a Discovery Card.")
    consolidate_parser.add_argument("--repo", required=True)
    consolidate_parser.add_argument("--session")
    consolidate_parser.add_argument("--json", action="store_true")

    verify = sub.add_parser("verify", help="Verify assimilation and issue a certificate.")
    verify.add_argument("--repo", required=True)
    verify.add_argument("--json", action="store_true")

    graph = sub.add_parser("graph", help="Rebuild or inspect structural relationships.")
    graph.add_argument("--repo", required=True)
    graph.add_argument("--path")
    graph.add_argument("--rebuild", action="store_true")
    graph.add_argument("--json", action="store_true")

    telemetry = sub.add_parser("telemetry", help="Refresh Git temporal and co-change memory.")
    telemetry.add_argument("--repo", required=True)
    telemetry.add_argument("--json", action="store_true")

    status = sub.add_parser("status", help="Show Cortex state for one repository or all repositories.")
    status.add_argument("--repo")
    status.add_argument("--json", action="store_true")

    doctor = sub.add_parser("doctor", help="Check Python, SQLite, database, and integration readiness.")
    doctor.add_argument("--repo")
    doctor.add_argument("--json", action="store_true")

    health = sub.add_parser("health", help="Emit a compact repository health and next-action packet.")
    health.add_argument("--repo", required=True)
    health.add_argument("--json", action="store_true")

    benchmark = sub.add_parser("benchmark", help="Verify committed controlled-workload benchmark thresholds.")
    benchmark.add_argument("--verify", action="store_true")
    benchmark.add_argument("--json", action="store_true")

    self_test = sub.add_parser("self-test", help="Clone Cortex inside a cloned Cortex host and verify self-hosted activation.")
    self_test.add_argument("--skip-tests", action="store_true")
    self_test.add_argument("--json", action="store_true")

    return parser


def _repo_root(store: Store, repo: str) -> Path:
    row = store.repo(repo)
    if not row:
        raise ValueError(f"Unknown repository: {repo}. Run cortex bootstrap first.")
    return Path(row["path"])


def main(argv: list[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    home = ensure_home(Path(args.home).expanduser().resolve() if args.home else None)
    store = Store(home / "cortex.db")
    governor = Governor(home, store)
    try:
        command = args.command
        if command == "init":
            emit({
                "initialized": True,
                "home": str(home),
                "database": str(home / "cortex.db"),
                "database_integrity": store.integrity_check(),
            }, args.json)

        elif command == "bootstrap":
            result = bootstrap_repository(
                home, store, Path(args.path), args.name, force=args.force
            )
            emit(result, args.json)

        elif command == "activate":
            refresh = {"packet-fast": "never", "packet-refresh": "auto", "bootstrap-full": "always"}.get(args.refresh, args.refresh)
            result = activate_repository(
                home,
                store,
                governor,
                args.repo,
                args.task,
                budget=args.budget,
                refresh=refresh,
            )
            result["requested_mode"] = args.refresh
            emit(result, args.json)

        elif command == "index":
            root = _repo_root(store, args.repo)
            config = load_repo_config(root)
            emit(index_repository(store, args.repo, config, force=args.force), args.json)

        elif command == "migrate-vectors":
            if args.repo and not store.repo(args.repo):
                raise ValueError(f"Unknown repository: {args.repo}. Run cortex bootstrap first.")
            emit(store.migrate_vectors(args.repo), args.json)

        elif command == "query":
            repository = store.repo(args.repo)
            if not repository:
                raise ValueError(f"Unknown repository: {args.repo}. Run cortex bootstrap first.")
            config = load_repo_config(Path(repository["path"]))
            hits = query(store, args.repo, args.query, args.limit, config.semantic_scan_limit)
            if config.thalamus_enabled:
                plan = route(make_request(repository, args.query, config.context_budget))
                hits = apply_feedback(store, args.repo, hits)
                hits = inhibit(
                    hits, plan.lane_weights, min_lane_relevance=config.thalamus_min_lane_relevance
                )
            emit([hit.to_dict() for hit in hits], args.json)

        elif command in {"context", "nexus-packet", "protocol"}:
            packet = build_context(home, store, governor, args.repo, args.task, args.budget)
            value = nexus_packet(packet) if command == "nexus-packet" else cortex_context_protocol(packet) if command == "protocol" else packet
            emit(value, args.json)

        elif command == "outcome":
            if not store.repo(args.repo):
                raise ValueError(f"Unknown repository: {args.repo}. Run cortex bootstrap first.")
            governance = governor.evaluate(args.repo)
            emit(record_outcome(
                store, args.repo, args.activation_id, status=args.status,
                verification_type=args.verification, reward=args.reward,
                governance_mode=governance["mode"],
            ), args.json)

        elif command == "environment":
            emit(environment_summary(store.environment_profile(args.repo)), args.json)

        elif command == "thalamus":
            repository = store.repo(args.repo)
            if not repository:
                raise ValueError(f"Unknown repository: {args.repo}. Run cortex bootstrap first.")
            emit(route(make_request(repository, args.task, args.budget)).to_dict(), args.json)

        elif command == "thalamus-feedback":
            if not store.repo(args.repo):
                raise ValueError(f"Unknown repository: {args.repo}. Run cortex bootstrap first.")
            emit(record_feedback(store, args.repo, args.memory_id, args.outcome), args.json)

        elif command == "interlink":
            root = _repo_root(store, args.repo)
            config = load_repo_config(root)
            repository = store.repo(args.repo)
            hits = query(store, args.repo, args.task, args.limit, config.semantic_scan_limit)
            if config.thalamus_enabled and repository:
                plan = route(make_request(repository, args.task, config.context_budget))
                hits = apply_feedback(store, args.repo, hits)
                hits = inhibit(
                    hits, plan.lane_weights, min_lane_relevance=config.thalamus_min_lane_relevance
                )
            governance = governor.evaluate(args.repo)
            packet = activate_interlink(
                store,
                args.repo,
                args.task,
                hits,
                max_depth=config.neural_activation_depth,
                max_nodes=config.neural_max_nodes,
                learning_rate=config.neural_learning_rate,
                plasticity_enabled=args.learn and config.neural_plasticity_enabled,
                governance_mode=governance["mode"],
            )
            emit(packet.to_dict(), args.json)

        elif command == "neural-replay":
            emit(
                [
                    {
                        "sequence": row["sequence"],
                        "event_type": row["event_type"],
                        "entity_id": row["entity_id"],
                        "payload": json.loads(row["payload"] or "{}"),
                        "created_at": row["created_at"],
                        "previous_hash": row["previous_hash"],
                        "event_hash": row["event_hash"],
                    }
                    for row in reversed(store.neural_events(args.repo, args.limit))
                ],
                args.json,
            )

        elif command == "focus":
            emit(begin_session(home, store, args.repo, args.task, args.files), args.json)

        elif command == "remember":
            emit(
                remember(home, store, args.repo, args.kind, args.text, args.session),
                args.json,
            )

        elif command == "consolidate":
            emit(consolidate(home, store, args.repo, args.session), args.json)

        elif command == "verify":
            root = _repo_root(store, args.repo)
            config = load_repo_config(root)
            emit(verify_repository(home, store, args.repo, config, write_certificate=True), args.json)

        elif command == "graph":
            if args.rebuild:
                result: Any = resolve_graph(store, args.repo)
            elif args.path:
                result = neighborhood(store, args.repo, [args.path], limit=100)
            else:
                edges = store.edges(args.repo, limit=100_000)
                counts: dict[str, int] = {}
                for edge in edges:
                    counts[edge["relation"]] = counts.get(edge["relation"], 0) + 1
                result = {
                    "repo": args.repo,
                    "files": len(store.files(args.repo)),
                    "symbols": len(store.symbols(args.repo)),
                    "edges": len(edges),
                    "relation_counts": counts,
                }
            emit(result, args.json)

        elif command == "telemetry":
            root = _repo_root(store, args.repo)
            config = load_repo_config(root)
            emit(ingest_git(store, args.repo, root, config.git_commit_limit), args.json)

        elif command == "status":
            if args.repo:
                repository = store.repo(args.repo)
                latest = store.latest_bootstrap(args.repo)
                result = {
                    "home": str(home),
                    "repository": dict(repository) if repository else None,
                    "governor": governor.evaluate(args.repo),
                    "latest_bootstrap": dict(latest) if latest else None,
                    "files": len(store.files(args.repo)) if repository else 0,
                    "symbols": len(store.symbols(args.repo)) if repository else 0,
                    "edges": len(store.edges(args.repo, limit=100_000)) if repository else 0,
                    "environment": environment_summary(store.environment_profile(args.repo)) if repository else {"available": False},
                    "neural_interlink": neural_graph_state(store, args.repo) if repository else None,
                }
            else:
                result = {
                    "home": str(home),
                    "database_integrity": store.integrity_check(),
                    "repositories": [dict(row) for row in store.repos()],
                }
            emit(result, args.json)

        elif command == "doctor":
            sqlite_version = store.db.execute("SELECT sqlite_version()").fetchone()[0]
            fts = bool(store.db.execute(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='memories_fts'"
            ).fetchone())
            result = {
                "python": sys.version,
                "sqlite": sqlite_version,
                "fts5_available": fts,
                "database_integrity": store.integrity_check(),
                "home_writable": home.exists() and home.is_dir(),
            }
            if args.repo:
                repository = store.repo(args.repo)
                result["repository"] = dict(repository) if repository else None
                result["governor"] = governor.evaluate(args.repo)
                result["environment"] = environment_summary(store.environment_profile(args.repo))
                result["neural_interlink"] = neural_graph_state(store, args.repo) if repository else None
                result["neural_ledger_integrity"] = store.verify_neural_ledger(args.repo) if repository else False
                result["vector_format"] = store.vector_format_status(args.repo) if repository else None
                if result["vector_format"] and result["vector_format"]["legacy_or_invalid"]:
                    result["vector_migration_recommendation"] = f"cortex migrate-vectors --repo {args.repo} --json"
            emit(result, args.json)

        elif command == "health":
            emit(health_report(home, store, governor, args.repo), args.json)

        elif command == "benchmark":
            result = verify_benchmarks(Path(__file__).resolve().parents[1])
            emit(result, args.json)
            if args.verify and result["status"] != "pass":
                raise RuntimeError("Cortex benchmark threshold regression")

        elif command == "self-test":
            emit(run_self_test(run_tests=not args.skip_tests), args.json)

    except (ValueError, FileNotFoundError, RuntimeError) as exc:
        error = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}
        if getattr(args, "json", False):
            print(json.dumps(error, indent=2), file=sys.stderr)
        else:
            print(error["error"], file=sys.stderr)
        raise SystemExit(2) from exc
    finally:
        store.close()
