#!/usr/bin/env python3
"""Persistent JSONL bridge between Perci and Cortex.

The process imports Cortex once and keeps its SQLite store warm. Each request is
one JSON object on stdin; each response is one JSON object on stdout.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import traceback
from pathlib import Path
from typing import Any


def emit(value: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(value, separators=(",", ":"), ensure_ascii=True) + "\n")
    sys.stdout.flush()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine-root", required=True)
    parser.add_argument("--home", required=True)
    parser.add_argument("--repo", required=True)
    args = parser.parse_args()

    engine_root = Path(args.engine_root).resolve()
    home = Path(args.home).resolve()
    repo = args.repo

    sys.path.insert(0, str(engine_root))
    os.environ["PYTHONPATH"] = str(engine_root)
    os.environ["CORTEX_HOME"] = str(home)

    from cortex.config import ensure_home
    from cortex.context import build_context, cortex_context_protocol
    from cortex.governor import Governor
    from cortex.hippocampus import remember
    from cortex.store import Store

    resolved_home = ensure_home(home)
    store = Store(resolved_home / "cortex.db")
    governor = Governor(resolved_home, store)

    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            operation = str(request.get("operation", "")).strip()

            if operation == "ping":
                emit({"ok": True, "result": {"ready": True, "repo": repo}})
                continue

            if operation == "protocol":
                task = str(request.get("task", "")).strip()
                budget = int(request.get("budget", 800))
                context = build_context(
                    resolved_home,
                    store,
                    governor,
                    repo,
                    task,
                    budget=max(200, min(budget, 2400)),
                )
                emit({"ok": True, "result": cortex_context_protocol(context)})
                continue

            if operation == "remember":
                kind = str(request.get("kind", "note")).strip() or "note"
                text = str(request.get("text", "")).strip()
                if not text:
                    raise ValueError("remember requires non-empty text")
                result = remember(resolved_home, store, repo, kind, text)
                emit({"ok": True, "result": result})
                continue

            if operation == "shutdown":
                emit({"ok": True, "result": {"shutdown": True}})
                return 0

            raise ValueError(f"unknown operation: {operation}")
        except Exception as exc:
            emit(
                {
                    "ok": False,
                    "error": f"{type(exc).__name__}: {exc}",
                    "trace": traceback.format_exc(limit=3),
                }
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())