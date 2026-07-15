from __future__ import annotations

import hashlib
import json
import time
import uuid
from pathlib import Path
from typing import Any


def _active_path(home: Path, repo: str) -> Path:
    safe = "".join(character if character.isalnum() or character in "-_" else "_" for character in repo)
    return home / "sessions" / f"{safe}-active.json"


def begin_session(
    home: Path,
    store: Any,
    repo: str,
    task: str,
    files: list[str] | None = None,
) -> dict[str, Any]:
    session_id = f"{int(time.time())}-{uuid.uuid4().hex[:8]}"
    payload = {
        "schema_version": "1.0",
        "session_id": session_id,
        "repo": repo,
        "task": task,
        "focus_files": files or [],
        "started_at": time.time(),
        "updated_at": time.time(),
        "state_hash": hashlib.sha256(f"{repo}|{task}|{session_id}".encode("utf-8")).hexdigest(),
    }
    path = _active_path(home, repo)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    store.start_session(session_id, repo, task, {"focus_files": files or []})
    store.add_event(session_id, repo, "focus", task, {"files": files or []})
    return payload


def active_session(home: Path, repo: str) -> dict[str, Any] | None:
    path = _active_path(home, repo)
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def remember(
    home: Path,
    store: Any,
    repo: str,
    kind: str,
    text: str,
    session_id: str | None = None,
    metadata: dict[str, Any] | None = None,
) -> dict[str, Any]:
    active = active_session(home, repo)
    resolved_session = session_id or (active or {}).get("session_id")
    store.add_event(resolved_session, repo, kind, text, metadata)
    if active:
        active["updated_at"] = time.time()
        active["last_event_kind"] = kind
        active["last_event_hash"] = hashlib.sha256(text.encode("utf-8")).hexdigest()
        _active_path(home, repo).write_text(json.dumps(active, indent=2) + "\n", encoding="utf-8")
    return {
        "recorded": True,
        "repo": repo,
        "session_id": resolved_session,
        "kind": kind,
        "text_hash": hashlib.sha256(text.encode("utf-8")).hexdigest(),
    }


def clear_active(home: Path, repo: str) -> None:
    path = _active_path(home, repo)
    if path.exists():
        path.unlink()
