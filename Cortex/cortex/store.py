from __future__ import annotations

from contextlib import contextmanager
from hashlib import sha256
import json
import sqlite3
import time
from pathlib import Path
from typing import Any, Iterable

from .embeddings import VECTOR_MAGIC, deserialize_vector, vector_to_bytes

SCHEMA = """
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS repositories(
    name TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    path TEXT NOT NULL,
    attached_at REAL NOT NULL,
    last_indexed REAL,
    last_bootstrap REAL,
    manifest_hash TEXT,
    bootstrap_status TEXT NOT NULL DEFAULT 'uninitialized',
    metadata TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS files(
    repo TEXT NOT NULL,
    path TEXT NOT NULL,
    kind TEXT NOT NULL,
    language TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    mtime_ns INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    authoritative INTEGER NOT NULL DEFAULT 0,
    indexed_at REAL NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY(repo, path),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_files_repo_kind ON files(repo, kind);
CREATE INDEX IF NOT EXISTS idx_files_repo_status ON files(repo, status);

CREATE TABLE IF NOT EXISTS memories(
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    path TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    vector TEXT,
    embedding_model TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at REAL NOT NULL,
    updated_at REAL NOT NULL,
    UNIQUE(repo, path, chunk_index, content_hash),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_mem_repo_path ON memories(repo, path);
CREATE INDEX IF NOT EXISTS idx_mem_repo_kind ON memories(repo, kind);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    text, path, kind, content='memories', content_rowid='id'
);
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, text, path, kind)
    VALUES(new.id, new.text, new.path, new.kind);
END;
CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, text, path, kind)
    VALUES('delete', old.id, old.text, old.path, old.kind);
END;
CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, text, path, kind)
    VALUES('delete', old.id, old.text, old.path, old.kind);
    INSERT INTO memories_fts(rowid, text, path, kind)
    VALUES(new.id, new.text, new.path, new.kind);
END;

CREATE TABLE IF NOT EXISTS edges(
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    source TEXT NOT NULL,
    target TEXT NOT NULL,
    relation TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    UNIQUE(repo, source, target, relation, evidence),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_edges_repo_source ON edges(repo, source);
CREATE INDEX IF NOT EXISTS idx_edges_repo_target ON edges(repo, target);
CREATE INDEX IF NOT EXISTS idx_edges_repo_relation ON edges(repo, relation);

CREATE TABLE IF NOT EXISTS symbols(
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    symbol_kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    signature TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    UNIQUE(repo, path, qualified_name, start_line),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_symbols_repo_name ON symbols(repo, name);
CREATE INDEX IF NOT EXISTS idx_symbols_repo_path ON symbols(repo, path);

CREATE TABLE IF NOT EXISTS git_commits(
    repo TEXT NOT NULL,
    commit_hash TEXT NOT NULL,
    authored_at REAL,
    author TEXT,
    subject TEXT,
    files TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY(repo, commit_hash),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS file_telemetry(
    repo TEXT NOT NULL,
    path TEXT NOT NULL,
    commit_count INTEGER NOT NULL DEFAULT 0,
    additions INTEGER NOT NULL DEFAULT 0,
    deletions INTEGER NOT NULL DEFAULT 0,
    last_changed REAL,
    cochange_degree INTEGER NOT NULL DEFAULT 0,
    metadata TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY(repo, path),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sessions(
    session_id TEXT PRIMARY KEY,
    repo TEXT NOT NULL,
    task TEXT NOT NULL,
    started_at REAL NOT NULL,
    ended_at REAL,
    status TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS events(
    id INTEGER PRIMARY KEY,
    session_id TEXT,
    repo TEXT NOT NULL,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at REAL NOT NULL,
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_events_repo_session ON events(repo, session_id);

CREATE TABLE IF NOT EXISTS bootstrap_runs(
    run_id TEXT PRIMARY KEY,
    repo TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at REAL NOT NULL,
    completed_at REAL,
    manifest_hash TEXT,
    certificate TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS environment_profiles(
    repo TEXT PRIMARY KEY,
    profile_json TEXT NOT NULL,
    profile_hash TEXT NOT NULL,
    observed_at REAL NOT NULL,
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS neural_nodes(
    repo TEXT NOT NULL,
    node_id TEXT NOT NULL,
    path TEXT NOT NULL,
    kind TEXT NOT NULL,
    threshold REAL NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    updated_at REAL NOT NULL,
    PRIMARY KEY(repo, node_id),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_neural_nodes_repo_path ON neural_nodes(repo, path);

CREATE TABLE IF NOT EXISTS neural_synapses(
    repo TEXT NOT NULL,
    synapse_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    base_weight REAL NOT NULL,
    weight REAL NOT NULL,
    minimum_weight REAL NOT NULL,
    maximum_weight REAL NOT NULL,
    plasticity_rule TEXT NOT NULL,
    update_count INTEGER NOT NULL DEFAULT 0,
    evidence TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    updated_at REAL NOT NULL,
    PRIMARY KEY(repo, synapse_id),
    UNIQUE(repo, source_id, target_id, relation),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_neural_synapses_repo_source ON neural_synapses(repo, source_id);
CREATE INDEX IF NOT EXISTS idx_neural_synapses_repo_target ON neural_synapses(repo, target_id);

CREATE TABLE IF NOT EXISTS neural_ledger(
    id INTEGER PRIMARY KEY,
    repo TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at REAL NOT NULL,
    previous_hash TEXT NOT NULL,
    event_hash TEXT NOT NULL,
    UNIQUE(repo, sequence),
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_neural_ledger_repo_sequence ON neural_ledger(repo, sequence);

CREATE TABLE IF NOT EXISTS neural_activations(
    activation_id TEXT PRIMARY KEY,
    repo TEXT NOT NULL,
    session_id TEXT,
    task_hash TEXT NOT NULL,
    state_hash TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at REAL NOT NULL,
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_neural_activations_repo_created ON neural_activations(repo, created_at);

CREATE TABLE IF NOT EXISTS task_outcomes(
    outcome_id TEXT PRIMARY KEY,
    repo TEXT NOT NULL,
    session_id TEXT,
    activation_id TEXT NOT NULL,
    status TEXT NOT NULL,
    reward REAL NOT NULL,
    verification_type TEXT NOT NULL,
    verification_payload_json TEXT NOT NULL DEFAULT '{}',
    created_at REAL NOT NULL,
    FOREIGN KEY(repo) REFERENCES repositories(name) ON DELETE CASCADE,
    FOREIGN KEY(activation_id) REFERENCES neural_activations(activation_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_task_outcomes_repo_created ON task_outcomes(repo, created_at);

CREATE TABLE IF NOT EXISTS evidence_credit(
    outcome_id TEXT NOT NULL,
    memory_id INTEGER,
    node_id TEXT,
    synapse_id TEXT,
    contribution REAL NOT NULL,
    reward_share REAL NOT NULL,
    reason TEXT NOT NULL,
    PRIMARY KEY(outcome_id, node_id, synapse_id),
    FOREIGN KEY(outcome_id) REFERENCES task_outcomes(outcome_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings(
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"""


class Store:
    def __init__(self, path: Path) -> None:
        self.path = path
        path.parent.mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(path)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA busy_timeout=5000")
        self.db.executescript(SCHEMA)

    def close(self) -> None:
        self.db.close()

    def commit(self) -> None:
        self.db.commit()

    @contextmanager
    def transaction(self):
        try:
            self.db.execute("BEGIN IMMEDIATE")
            yield self.db
            self.db.commit()
        except Exception:
            self.db.rollback()
            raise

    def integrity_check(self) -> bool:
        row = self.db.execute("PRAGMA integrity_check").fetchone()
        return bool(row and row[0] == "ok")

    def attach(self, name: str, repository_id: str, path: Path) -> None:
        now = time.time()
        existing = self.repo(name)
        resolved_path = str(path.resolve())
        if existing and (existing["repository_id"] != repository_id or existing["path"] != resolved_path):
            self.db.execute("DELETE FROM repositories WHERE name=?", (name,))
            self.db.commit()
        self.db.execute(
            """
            INSERT INTO repositories(name, repository_id, path, attached_at)
            VALUES(?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
              repository_id=excluded.repository_id,
              path=excluded.path
            """,
            (name, repository_id, resolved_path, now),
        )
        self.db.commit()

    def repo(self, name: str) -> sqlite3.Row | None:
        return self.db.execute("SELECT * FROM repositories WHERE name=?", (name,)).fetchone()

    def repo_by_path(self, path: Path) -> sqlite3.Row | None:
        return self.db.execute(
            "SELECT * FROM repositories WHERE path=?", (str(path.resolve()),)
        ).fetchone()

    def repos(self) -> list[sqlite3.Row]:
        return self.db.execute("SELECT * FROM repositories ORDER BY name").fetchall()

    def update_repo_state(
        self,
        repo: str,
        *,
        manifest_hash: str | None = None,
        bootstrap_status: str | None = None,
        indexed: bool = False,
        bootstrapped: bool = False,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        fields: list[str] = []
        values: list[Any] = []
        if manifest_hash is not None:
            fields.append("manifest_hash=?")
            values.append(manifest_hash)
        if bootstrap_status is not None:
            fields.append("bootstrap_status=?")
            values.append(bootstrap_status)
        if indexed:
            fields.append("last_indexed=?")
            values.append(time.time())
        if bootstrapped:
            fields.append("last_bootstrap=?")
            values.append(time.time())
        if metadata is not None:
            fields.append("metadata=?")
            values.append(json.dumps(metadata, sort_keys=True))
        if not fields:
            return
        values.append(repo)
        self.db.execute(f"UPDATE repositories SET {', '.join(fields)} WHERE name=?", values)
        self.db.commit()

    def file(self, repo: str, path: str) -> sqlite3.Row | None:
        return self.db.execute(
            "SELECT * FROM files WHERE repo=? AND path=?", (repo, path)
        ).fetchone()

    def files(self, repo: str, status: str | None = None) -> list[sqlite3.Row]:
        if status:
            return self.db.execute(
                "SELECT * FROM files WHERE repo=? AND status=? ORDER BY path", (repo, status)
            ).fetchall()
        return self.db.execute("SELECT * FROM files WHERE repo=? ORDER BY path", (repo,)).fetchall()

    def upsert_file(self, record: dict[str, Any]) -> None:
        self.db.execute(
            """
            INSERT INTO files(
              repo, path, kind, language, size_bytes, mtime_ns, content_hash,
              status, authoritative, indexed_at, metadata
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(repo, path) DO UPDATE SET
              kind=excluded.kind,
              language=excluded.language,
              size_bytes=excluded.size_bytes,
              mtime_ns=excluded.mtime_ns,
              content_hash=excluded.content_hash,
              status=excluded.status,
              authoritative=excluded.authoritative,
              indexed_at=excluded.indexed_at,
              metadata=excluded.metadata
            """,
            (
                record["repo"], record["path"], record["kind"], record["language"],
                record["size_bytes"], record["mtime_ns"], record["content_hash"],
                record["status"], int(record.get("authoritative", False)), time.time(),
                json.dumps(record.get("metadata", {}), sort_keys=True),
            ),
        )

    def delete_missing_files(self, repo: str, live_paths: set[str]) -> list[str]:
        existing = {row["path"] for row in self.files(repo)}
        missing = sorted(existing - live_paths)
        for path in missing:
            self.remove_path(repo, path)
            self.db.execute("DELETE FROM files WHERE repo=? AND path=?", (repo, path))
        return missing

    def remove_path(self, repo: str, path: str) -> None:
        self.db.execute("DELETE FROM memories WHERE repo=? AND path=?", (repo, path))
        self.db.execute("DELETE FROM symbols WHERE repo=? AND path=?", (repo, path))
        self.db.execute(
            "DELETE FROM edges WHERE repo=? AND (source=? OR target=?)", (repo, path, path)
        )

    def upsert_memory(self, **memory: Any) -> None:
        now = time.time()
        self.db.execute(
            """
            INSERT INTO memories(
              repo, path, chunk_index, start_line, end_line, kind, text, content_hash,
              vector, embedding_model, metadata, created_at, updated_at
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(repo, path, chunk_index, content_hash) DO UPDATE SET
              start_line=excluded.start_line,
              end_line=excluded.end_line,
              kind=excluded.kind,
              text=excluded.text,
              vector=excluded.vector,
              embedding_model=excluded.embedding_model,
              metadata=excluded.metadata,
              updated_at=excluded.updated_at
            """,
            (
                memory["repo"], memory["path"], memory["chunk_index"],
                memory["start_line"], memory["end_line"], memory["kind"],
                memory["text"], memory["content_hash"],
                vector_to_bytes(memory.get("vector")), memory.get("embedding_model"),
                json.dumps(memory.get("metadata", {}), sort_keys=True), now, now,
            ),
        )

    def memory(self, memory_id: int) -> sqlite3.Row | None:
        return self.db.execute("SELECT * FROM memories WHERE id=?", (memory_id,)).fetchone()

    def memories_for_path(self, repo: str, path: str) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM memories WHERE repo=? AND path=? ORDER BY chunk_index",
            (repo, path),
        ).fetchall()

    def all_vectors(self, repo: str) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM memories WHERE repo=? AND vector IS NOT NULL", (repo,)
        ).fetchall()

    def vector_candidates(
        self, repo: str, preferred_ids: list[int], limit: int, seed: int
    ) -> list[sqlite3.Row]:
        output: dict[int, sqlite3.Row] = {}
        if preferred_ids:
            marks = ",".join("?" for _ in preferred_ids)
            rows = self.db.execute(
                f"SELECT * FROM memories WHERE repo=? AND id IN ({marks}) AND vector IS NOT NULL",
                [repo, *preferred_ids],
            ).fetchall()
            output.update({row["id"]: row for row in rows})
        remaining = max(0, limit - len(output))
        if remaining == 0:
            return list(output.values())
        bounds = self.db.execute(
            "SELECT MIN(id), MAX(id), COUNT(*) FROM memories WHERE repo=? AND vector IS NOT NULL",
            (repo,),
        ).fetchone()
        if not bounds or not bounds[2]:
            return list(output.values())
        minimum, maximum, count = int(bounds[0]), int(bounds[1]), int(bounds[2])
        if count <= remaining:
            rows = self.all_vectors(repo)
        else:
            span = max(1, maximum - minimum + 1)
            pivot = minimum + (seed % span)
            rows = self.db.execute(
                "SELECT * FROM memories WHERE repo=? AND vector IS NOT NULL AND id>=? ORDER BY id LIMIT ?",
                (repo, pivot, remaining),
            ).fetchall()
            if len(rows) < remaining:
                rows += self.db.execute(
                    "SELECT * FROM memories WHERE repo=? AND vector IS NOT NULL AND id<? ORDER BY id LIMIT ?",
                    (repo, pivot, remaining - len(rows)),
                ).fetchall()
        output.update({row["id"]: row for row in rows})
        return list(output.values())

    def lexical(self, repo: str, query: str, limit: int = 40) -> list[sqlite3.Row]:
        tokens = [token for token in query.replace('"', " ").split() if token]
        if not tokens:
            return []
        safe = " OR ".join(f'"{token}"' for token in tokens[:24])
        try:
            return self.db.execute(
                """
                SELECT m.*, bm25(memories_fts) AS bm
                FROM memories_fts
                JOIN memories m ON m.id=memories_fts.rowid
                WHERE m.repo=? AND memories_fts MATCH ?
                ORDER BY bm LIMIT ?
                """,
                (repo, safe, limit),
            ).fetchall()
        except sqlite3.OperationalError:
            return []

    def add_symbol(self, repo: str, path: str, symbol: dict[str, Any]) -> None:
        self.db.execute(
            """
            INSERT OR REPLACE INTO symbols(
              repo, path, name, qualified_name, symbol_kind, start_line, end_line,
              signature, metadata
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                repo, path, symbol["name"], symbol["qualified_name"], symbol["symbol_kind"],
                symbol["start_line"], symbol["end_line"], symbol.get("signature", ""),
                json.dumps(symbol.get("metadata", {}), sort_keys=True),
            ),
        )

    def migrate_vectors(self, repo: str | None = None) -> dict[str, int]:
        """Upgrade legacy JSON or unversioned BLOB vectors without re-indexing source."""
        where = "WHERE vector IS NOT NULL"
        args: list[Any] = []
        if repo:
            where += " AND repo=?"
            args.append(repo)
        rows = self.db.execute(f"SELECT id, vector FROM memories {where}", args).fetchall()
        migrated = 0
        skipped = 0
        for row in rows:
            raw = row["vector"]
            if isinstance(raw, bytes) and raw.startswith(VECTOR_MAGIC):
                skipped += 1
                continue
            vector = deserialize_vector(raw)
            if not vector:
                skipped += 1
                continue
            self.db.execute("UPDATE memories SET vector=?, updated_at=? WHERE id=?", (vector_to_bytes(vector), time.time(), row["id"]))
            migrated += 1
        self.db.commit()
        result = {"scanned": len(rows), "migrated": migrated, "already_current_or_invalid": skipped}
        self.set_setting(f"vector_migration:{repo or 'all'}", {"completed_at": time.time(), **result})
        return result

    def vector_format_status(self, repo: str | None = None) -> dict[str, int]:
        where = "WHERE vector IS NOT NULL"
        args: list[Any] = []
        if repo:
            where += " AND repo=?"
            args.append(repo)
        rows = self.db.execute(f"SELECT vector FROM memories {where}", args).fetchall()
        current = sum(isinstance(row["vector"], bytes) and row["vector"].startswith(VECTOR_MAGIC) for row in rows)
        return {"total": len(rows), "current_versioned_blob": current, "legacy_or_invalid": len(rows) - current}

    def symbols(self, repo: str, path: str | None = None) -> list[sqlite3.Row]:
        if path:
            return self.db.execute(
                "SELECT * FROM symbols WHERE repo=? AND path=? ORDER BY start_line", (repo, path)
            ).fetchall()
        return self.db.execute(
            "SELECT * FROM symbols WHERE repo=? ORDER BY path, start_line", (repo,)
        ).fetchall()

    def add_edge(self, repo: str, edge: dict[str, Any]) -> None:
        self.db.execute(
            """
            INSERT OR IGNORE INTO edges(
              repo, source, target, relation, confidence, evidence, metadata
            ) VALUES(?, ?, ?, ?, ?, ?, ?)
            """,
            (
                repo, edge["source"], edge["target"], edge["relation"],
                float(edge["confidence"]), edge.get("evidence", ""),
                json.dumps(edge.get("metadata", {}), sort_keys=True),
            ),
        )

    def edges(
        self, repo: str, *, source: str | None = None, target: str | None = None,
        relation: str | None = None, limit: int = 500
    ) -> list[sqlite3.Row]:
        clauses = ["repo=?"]
        values: list[Any] = [repo]
        if source:
            clauses.append("source=?")
            values.append(source)
        if target:
            clauses.append("target=?")
            values.append(target)
        if relation:
            clauses.append("relation=?")
            values.append(relation)
        values.append(limit)
        return self.db.execute(
            f"SELECT * FROM edges WHERE {' AND '.join(clauses)} ORDER BY confidence DESC LIMIT ?",
            values,
        ).fetchall()

    def clear_edges(self, repo: str, relations: Iterable[str] | None = None) -> None:
        if relations:
            marks = ",".join("?" for _ in relations)
            values = [repo, *relations]
            self.db.execute(
                f"DELETE FROM edges WHERE repo=? AND relation IN ({marks})", values
            )
        else:
            self.db.execute("DELETE FROM edges WHERE repo=?", (repo,))

    def add_commit(self, repo: str, commit: dict[str, Any]) -> None:
        self.db.execute(
            """
            INSERT OR REPLACE INTO git_commits(
              repo, commit_hash, authored_at, author, subject, files
            ) VALUES(?, ?, ?, ?, ?, ?)
            """,
            (
                repo, commit["commit_hash"], commit.get("authored_at"), commit.get("author"),
                commit.get("subject"), json.dumps(commit.get("files", [])),
            ),
        )

    def commits(self, repo: str, limit: int = 100) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM git_commits WHERE repo=? ORDER BY authored_at DESC LIMIT ?",
            (repo, limit),
        ).fetchall()

    def set_file_telemetry(self, repo: str, path: str, telemetry: dict[str, Any]) -> None:
        self.db.execute(
            """
            INSERT INTO file_telemetry(
              repo, path, commit_count, additions, deletions, last_changed,
              cochange_degree, metadata
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(repo, path) DO UPDATE SET
              commit_count=excluded.commit_count,
              additions=excluded.additions,
              deletions=excluded.deletions,
              last_changed=excluded.last_changed,
              cochange_degree=excluded.cochange_degree,
              metadata=excluded.metadata
            """,
            (
                repo, path, telemetry.get("commit_count", 0), telemetry.get("additions", 0),
                telemetry.get("deletions", 0), telemetry.get("last_changed"),
                telemetry.get("cochange_degree", 0),
                json.dumps(telemetry.get("metadata", {}), sort_keys=True),
            ),
        )

    def file_telemetry(self, repo: str, path: str | None = None) -> list[sqlite3.Row]:
        if path:
            return self.db.execute(
                "SELECT * FROM file_telemetry WHERE repo=? AND path=?", (repo, path)
            ).fetchall()
        return self.db.execute(
            "SELECT * FROM file_telemetry WHERE repo=? ORDER BY commit_count DESC", (repo,)
        ).fetchall()

    def begin_bootstrap(self, run_id: str, repo: str) -> None:
        self.db.execute(
            "INSERT INTO bootstrap_runs(run_id, repo, status, started_at) VALUES(?, ?, ?, ?)",
            (run_id, repo, "running", time.time()),
        )
        self.db.commit()

    def finish_bootstrap(
        self, run_id: str, status: str, manifest_hash: str, certificate: dict[str, Any]
    ) -> None:
        self.db.execute(
            """
            UPDATE bootstrap_runs
            SET status=?, completed_at=?, manifest_hash=?, certificate=?
            WHERE run_id=?
            """,
            (status, time.time(), manifest_hash, json.dumps(certificate, sort_keys=True), run_id),
        )
        self.db.commit()

    def latest_bootstrap(self, repo: str) -> sqlite3.Row | None:
        return self.db.execute(
            "SELECT * FROM bootstrap_runs WHERE repo=? ORDER BY started_at DESC LIMIT 1", (repo,)
        ).fetchone()

    def start_session(
        self, session_id: str, repo: str, task: str, metadata: dict[str, Any] | None = None
    ) -> None:
        self.db.execute(
            """
            INSERT OR REPLACE INTO sessions(
              session_id, repo, task, started_at, status, metadata
            ) VALUES(?, ?, ?, ?, ?, ?)
            """,
            (session_id, repo, task, time.time(), "active", json.dumps(metadata or {})),
        )
        self.db.commit()

    def end_session(self, session_id: str, status: str = "consolidated") -> None:
        self.db.execute(
            "UPDATE sessions SET ended_at=?, status=? WHERE session_id=?",
            (time.time(), status, session_id),
        )
        self.db.commit()

    def session(self, session_id: str) -> sqlite3.Row | None:
        return self.db.execute(
            "SELECT * FROM sessions WHERE session_id=?", (session_id,)
        ).fetchone()

    def latest_session(self, repo: str) -> sqlite3.Row | None:
        return self.db.execute(
            "SELECT * FROM sessions WHERE repo=? ORDER BY started_at DESC LIMIT 1", (repo,)
        ).fetchone()

    def add_event(
        self,
        session_id: str | None,
        repo: str,
        kind: str,
        text: str,
        metadata: dict[str, Any] | None = None,
    ) -> None:
        self.db.execute(
            """
            INSERT INTO events(session_id, repo, kind, text, metadata, created_at)
            VALUES(?, ?, ?, ?, ?, ?)
            """,
            (session_id, repo, kind, text, json.dumps(metadata or {}), time.time()),
        )
        self.db.commit()

    def events(self, repo: str, session_id: str | None = None) -> list[sqlite3.Row]:
        if session_id:
            return self.db.execute(
                "SELECT * FROM events WHERE repo=? AND session_id=? ORDER BY id",
                (repo, session_id),
            ).fetchall()
        return self.db.execute(
            "SELECT * FROM events WHERE repo=? ORDER BY id DESC LIMIT 500", (repo,)
        ).fetchall()

    def set_environment_profile(self, repo: str, profile: dict[str, Any]) -> None:
        profile_hash = str(profile.get("profile_hash", ""))
        self.db.execute(
            """
            INSERT INTO environment_profiles(repo, profile_json, profile_hash, observed_at)
            VALUES(?, ?, ?, ?)
            ON CONFLICT(repo) DO UPDATE SET
              profile_json=excluded.profile_json,
              profile_hash=excluded.profile_hash,
              observed_at=excluded.observed_at
            """,
            (repo, json.dumps(profile, sort_keys=True), profile_hash, time.time()),
        )
        self.db.commit()

    def environment_profile(self, repo: str) -> dict[str, Any] | None:
        row = self.db.execute(
            "SELECT profile_json FROM environment_profiles WHERE repo=?", (repo,)
        ).fetchone()
        return json.loads(row[0]) if row else None

    def sync_neural_graph(self, repo: str, nodes: list[Any], synapses: list[Any]) -> None:
        live_nodes = {node.node_id for node in nodes}
        live_synapses = {synapse.synapse_id for synapse in synapses}
        now = time.time()
        with self.transaction() as conn:
            for node in nodes:
                conn.execute(
                    """
                    INSERT INTO neural_nodes(
                      repo, node_id, path, kind, threshold, tags_json, metadata, updated_at
                    ) VALUES(?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(repo, node_id) DO UPDATE SET
                      path=excluded.path, kind=excluded.kind, threshold=excluded.threshold,
                      tags_json=excluded.tags_json, metadata=excluded.metadata, updated_at=excluded.updated_at
                    """,
                    (
                        repo, node.node_id, node.path, node.kind, node.threshold,
                        json.dumps(node.tags), json.dumps(node.metadata, sort_keys=True), now,
                    ),
                )
            for synapse in synapses:
                conn.execute(
                    """
                    INSERT INTO neural_synapses(
                      repo, synapse_id, source_id, target_id, relation, base_weight, weight,
                      minimum_weight, maximum_weight, plasticity_rule, update_count,
                      evidence, metadata, updated_at
                    ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(repo, synapse_id) DO UPDATE SET
                      source_id=excluded.source_id, target_id=excluded.target_id,
                      relation=excluded.relation, base_weight=excluded.base_weight,
                      weight=MIN(excluded.maximum_weight, MAX(excluded.minimum_weight, neural_synapses.weight)),
                      minimum_weight=excluded.minimum_weight, maximum_weight=excluded.maximum_weight,
                      plasticity_rule=excluded.plasticity_rule, evidence=excluded.evidence,
                      metadata=excluded.metadata, updated_at=excluded.updated_at
                    """,
                    (
                        repo, synapse.synapse_id, synapse.source_id, synapse.target_id,
                        synapse.relation, synapse.base_weight, synapse.weight,
                        synapse.minimum_weight, synapse.maximum_weight, synapse.plasticity_rule,
                        synapse.update_count, synapse.evidence,
                        json.dumps(synapse.metadata, sort_keys=True), now,
                    ),
                )
            if live_nodes:
                marks = ",".join("?" for _ in live_nodes)
                conn.execute(
                    f"DELETE FROM neural_nodes WHERE repo=? AND node_id NOT IN ({marks})",
                    [repo, *sorted(live_nodes)],
                )
            else:
                conn.execute("DELETE FROM neural_nodes WHERE repo=?", (repo,))
            if live_synapses:
                marks = ",".join("?" for _ in live_synapses)
                conn.execute(
                    f"DELETE FROM neural_synapses WHERE repo=? AND synapse_id NOT IN ({marks})",
                    [repo, *sorted(live_synapses)],
                )
            else:
                conn.execute("DELETE FROM neural_synapses WHERE repo=?", (repo,))

    def neural_nodes(self, repo: str) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM neural_nodes WHERE repo=? ORDER BY node_id", (repo,)
        ).fetchall()

    def neural_synapses(self, repo: str) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM neural_synapses WHERE repo=? ORDER BY synapse_id", (repo,)
        ).fetchall()

    def neural_graph_hash(self, repo: str) -> str:
        material = {
            "nodes": [
                [row["node_id"], row["path"], row["kind"], row["threshold"]]
                for row in self.neural_nodes(repo)
            ],
            "synapses": [
                [
                    row["synapse_id"], row["source_id"], row["target_id"],
                    row["relation"], row["base_weight"], row["weight"], row["update_count"],
                ]
                for row in self.neural_synapses(repo)
            ],
        }
        canonical = json.dumps(material, sort_keys=True, separators=(",", ":"))
        return sha256(canonical.encode("utf-8")).hexdigest()

    def update_neural_synapse_weight(self, repo: str, synapse_id: str, weight: float) -> None:
        self.db.execute(
            """
            UPDATE neural_synapses
            SET weight=MIN(maximum_weight, MAX(minimum_weight, ?)),
                update_count=update_count+1, updated_at=?
            WHERE repo=? AND synapse_id=?
            """,
            (float(weight), time.time(), repo, synapse_id),
        )
        self.db.commit()

    @staticmethod
    def _neural_event_hash(record: dict[str, Any]) -> str:
        canonical = json.dumps(record, sort_keys=True, separators=(",", ":"))
        return sha256(canonical.encode("utf-8")).hexdigest()

    def _append_neural_event_conn(
        self, conn: sqlite3.Connection, repo: str, *, event_type: str,
        entity_id: str, payload: dict[str, Any]
    ) -> str:
        tail = conn.execute(
            "SELECT sequence, event_hash FROM neural_ledger WHERE repo=? ORDER BY sequence DESC LIMIT 1",
            (repo,),
        ).fetchone()
        sequence = int(tail["sequence"]) + 1 if tail else 1
        previous_hash = str(tail["event_hash"]) if tail else "0" * 64
        created_at = time.time()
        record = {
            "repo": repo,
            "sequence": sequence,
            "event_type": event_type,
            "entity_id": entity_id,
            "payload": payload,
            "created_at": created_at,
            "previous_hash": previous_hash,
        }
        event_hash = self._neural_event_hash(record)
        conn.execute(
            """
            INSERT INTO neural_ledger(
              repo, sequence, event_type, entity_id, payload, created_at, previous_hash, event_hash
            ) VALUES(?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                repo, sequence, event_type, entity_id,
                json.dumps(payload, sort_keys=True), created_at, previous_hash, event_hash,
            ),
        )
        return event_hash

    def append_neural_event(
        self, repo: str, *, event_type: str, entity_id: str, payload: dict[str, Any]
    ) -> str:
        with self.transaction() as conn:
            return self._append_neural_event_conn(
                conn, repo, event_type=event_type, entity_id=entity_id, payload=payload
            )

    def apply_neural_plasticity(
        self, repo: str, activation_id: str, updates: list[dict[str, Any]]
    ) -> str | None:
        if not updates:
            return None
        with self.transaction() as conn:
            for update in updates:
                conn.execute(
                    """
                    UPDATE neural_synapses
                    SET weight=MIN(maximum_weight, MAX(minimum_weight, ?)),
                        update_count=update_count+1, updated_at=?
                    WHERE repo=? AND synapse_id=?
                    """,
                    (
                        float(update["proposed_weight"]), time.time(), repo,
                        str(update["synapse_id"]),
                    ),
                )
            return self._append_neural_event_conn(
                conn,
                repo,
                event_type="plasticity_applied",
                entity_id=activation_id,
                payload={"updates": updates},
            )

    def neural_events(self, repo: str, limit: int = 100) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM neural_ledger WHERE repo=? ORDER BY sequence DESC LIMIT ?",
            (repo, limit),
        ).fetchall()

    def verify_neural_ledger(self, repo: str) -> bool:
        previous = "0" * 64
        expected = 1
        rows = self.db.execute(
            "SELECT * FROM neural_ledger WHERE repo=? ORDER BY sequence", (repo,)
        ).fetchall()
        for row in rows:
            if int(row["sequence"]) != expected or row["previous_hash"] != previous:
                return False
            payload = json.loads(row["payload"] or "{}")
            record = {
                "repo": repo,
                "sequence": int(row["sequence"]),
                "event_type": row["event_type"],
                "entity_id": row["entity_id"],
                "payload": payload,
                "created_at": float(row["created_at"]),
                "previous_hash": row["previous_hash"],
            }
            if self._neural_event_hash(record) != row["event_hash"]:
                return False
            previous = row["event_hash"]
            expected += 1
        return True

    def record_neural_activation(
        self, repo: str, session_id: str | None, payload: dict[str, Any]
    ) -> None:
        self.db.execute(
            """
            INSERT OR REPLACE INTO neural_activations(
              activation_id, repo, session_id, task_hash, state_hash, payload, created_at
            ) VALUES(?, ?, ?, ?, ?, ?, ?)
            """,
            (
                payload["activation_id"], repo, session_id, payload["task_hash"],
                payload["state_hash"], json.dumps(payload, sort_keys=True), time.time(),
            ),
        )
        self.db.commit()

    def neural_activations(self, repo: str, limit: int = 20) -> list[dict[str, Any]]:
        rows = self.db.execute(
            "SELECT payload FROM neural_activations WHERE repo=? ORDER BY created_at DESC LIMIT ?",
            (repo, limit),
        ).fetchall()
        return [json.loads(row["payload"]) for row in rows]

    def neural_activation(self, repo: str, activation_id: str) -> dict[str, Any] | None:
        row = self.db.execute(
            "SELECT payload FROM neural_activations WHERE repo=? AND activation_id=?", (repo, activation_id)
        ).fetchone()
        return json.loads(row["payload"]) if row else None

    def record_outcome(
        self, repo: str, *, outcome_id: str, activation_id: str, status: str, reward: float,
        verification_type: str, verification_payload: dict[str, Any], credits: list[dict[str, Any]],
        updates: list[dict[str, Any]], apply_updates: bool,
    ) -> None:
        activation = self.db.execute(
            "SELECT session_id FROM neural_activations WHERE repo=? AND activation_id=?", (repo, activation_id)
        ).fetchone()
        if not activation:
            raise ValueError("Activation does not belong to this repository")
        now = time.time()
        with self.transaction() as conn:
            conn.execute(
                """INSERT INTO task_outcomes(outcome_id, repo, session_id, activation_id, status, reward,
                   verification_type, verification_payload_json, created_at)
                   VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (outcome_id, repo, activation["session_id"], activation_id, status, reward,
                 verification_type, json.dumps(verification_payload, sort_keys=True), now),
            )
            for credit in credits:
                conn.execute(
                    """INSERT INTO evidence_credit(outcome_id, memory_id, node_id, synapse_id, contribution,
                       reward_share, reason) VALUES(?, ?, ?, ?, ?, ?, ?)""",
                    (outcome_id, credit.get("memory_id"), credit.get("node_id"), credit.get("synapse_id"),
                     credit["contribution"], credit["reward_share"], credit["reason"]),
                )
            if apply_updates:
                for update in updates:
                    conn.execute(
                        """UPDATE neural_synapses SET weight=MIN(maximum_weight, MAX(minimum_weight, ?)),
                           update_count=update_count+1, updated_at=? WHERE repo=? AND synapse_id=?""",
                        (float(update["proposed_weight"]), now, repo, update["synapse_id"]),
                    )
            self._append_neural_event_conn(
                conn, repo, event_type="verified_outcome", entity_id=outcome_id,
                payload={"activation_id": activation_id, "status": status, "reward": reward,
                         "verification_type": verification_type, "credits": len(credits),
                         "updates": updates, "applied": apply_updates},
            )

    def outcomes(self, repo: str, limit: int = 100) -> list[sqlite3.Row]:
        return self.db.execute(
            "SELECT * FROM task_outcomes WHERE repo=? ORDER BY created_at DESC LIMIT ?", (repo, limit)
        ).fetchall()

    def set_setting(self, key: str, value: Any) -> None:
        self.db.execute(
            """
            INSERT INTO settings(key, value) VALUES(?, ?)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value
            """,
            (key, json.dumps(value, sort_keys=True)),
        )
        self.db.commit()

    def get_setting(self, key: str, default: Any = None) -> Any:
        row = self.db.execute("SELECT value FROM settings WHERE key=?", (key,)).fetchone()
        return json.loads(row[0]) if row else default
