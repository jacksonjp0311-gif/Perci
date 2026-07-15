from __future__ import annotations

import hashlib
import time
from collections import defaultdict
from pathlib import Path
from typing import Any

from .embeddings import get_embedder
from .hippocampus import active_session, clear_active

IMPORTANT_KINDS = {
    "decision", "discovery", "invariant", "wound", "failure", "fix", "outcome",
    "interface", "constraint", "evidence", "lesson", "focus",
}


def summarize_events(events: list[Any]) -> str:
    grouped: dict[str, list[str]] = defaultdict(list)
    for event in events:
        kind = event["kind"]
        text = event["text"].strip()
        if not text or kind not in IMPORTANT_KINDS:
            continue
        if text not in grouped[kind]:
            grouped[kind].append(text)
    sections: list[str] = []
    order = [
        "focus", "decision", "discovery", "invariant", "interface", "constraint",
        "evidence", "wound", "failure", "fix", "outcome", "lesson",
    ]
    for kind in order:
        items = grouped.get(kind, [])
        if not items:
            continue
        sections.append(f"## {kind.replace('_', ' ').title()}")
        sections.extend(f"- {item[:1200]}" for item in items[:20])
        sections.append("")
    return "\n".join(sections).strip()


def consolidate(home: Path, store: Any, repo: str, session_id: str | None = None) -> dict[str, Any]:
    active = active_session(home, repo)
    resolved_session = session_id or (active or {}).get("session_id")
    if not resolved_session:
        latest = store.latest_session(repo)
        resolved_session = latest["session_id"] if latest else None
    if not resolved_session:
        return {"created": False, "reason": "no session available"}

    events = store.events(repo, resolved_session)
    body = summarize_events(events)
    if not body:
        return {"created": False, "reason": "no consolidatable events", "session_id": resolved_session}

    session = store.session(resolved_session)
    task = session["task"] if session else "Unknown task"
    created_at = time.time()
    card = (
        f"# Cortex Discovery Card\n\n"
        f"- Repository: `{repo}`\n"
        f"- Session: `{resolved_session}`\n"
        f"- Task: {task}\n"
        f"- Created: {created_at}\n\n"
        f"{body}\n\n"
        "## Provenance\n"
        "This card is a deterministic consolidation of explicitly recorded session events. "
        "Repository source remains authoritative.\n"
    )
    card_hash = hashlib.sha256(card.encode("utf-8")).hexdigest()
    filename = f"{repo}-{resolved_session}-{card_hash[:10]}.md".replace("/", "_")
    path = home / "cards" / filename
    path.write_text(card, encoding="utf-8")

    embedder = get_embedder()
    memory_path = f".cortex/cards/{filename}"
    store.remove_path(repo, memory_path)
    store.upsert_memory(
        repo=repo,
        path=memory_path,
        chunk_index=0,
        start_line=1,
        end_line=len(card.splitlines()),
        kind="discovery_card",
        text=card,
        content_hash=card_hash,
        vector=embedder.encode_one(card),
        embedding_model=embedder.name,
        metadata={
            "session_id": resolved_session,
            "task": task,
            "generated": True,
            "source_event_count": len(events),
        },
    )
    store.end_session(resolved_session, "consolidated")
    store.commit()
    if active and active.get("session_id") == resolved_session:
        clear_active(home, repo)
    return {
        "created": True,
        "repo": repo,
        "session_id": resolved_session,
        "path": str(path),
        "memory_path": memory_path,
        "hash": card_hash,
        "event_count": len(events),
    }
