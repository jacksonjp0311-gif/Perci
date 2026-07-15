"""Measure legacy JSON versus versioned float32 vector storage and query cost."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import tempfile
import time
import sys

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from cortex.embeddings import HashingEmbedder  # noqa: E402
from cortex.retrieval import query  # noqa: E402
from cortex.store import Store  # noqa: E402


def measure(store: Store, runs: int) -> float:
    started = time.perf_counter()
    for _ in range(runs):
        query(store, "Vectors", "migration storage retrieval performance", limit=8)
    return round((time.perf_counter() - started) * 1000 / runs, 6)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vectors", type=int, default=1000)
    parser.add_argument("--runs", type=int, default=20)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as temporary:
        path = Path(temporary) / "cortex.db"
        store = Store(path)
        try:
            store.attach("Vectors", "vectors", Path(temporary))
            embedder = HashingEmbedder()
            now = time.time()
            for index in range(args.vectors):
                vector = embedder.encode_one(f"module {index} storage retrieval")
                store.db.execute("INSERT INTO memories(repo,path,chunk_index,start_line,end_line,kind,text,content_hash,vector,embedding_model,metadata,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)", ("Vectors", f"m{index}.py", 0, 1, 1, "source", f"module {index} storage retrieval", str(index), json.dumps(vector), "legacy", "{}", now, now))
            store.commit()
            store.db.execute("PRAGMA wal_checkpoint(TRUNCATE)")
            legacy_bytes, legacy_ms = path.stat().st_size, measure(store, args.runs)
            legacy_payload = store.db.execute("SELECT SUM(length(vector)) FROM memories").fetchone()[0]
            migration = store.migrate_vectors("Vectors")
            store.db.execute("VACUUM")
            blob_bytes, blob_ms = path.stat().st_size, measure(store, args.runs)
            blob_payload = store.db.execute("SELECT SUM(length(vector)) FROM memories").fetchone()[0]
            print(json.dumps({"vectors": args.vectors, "runs": args.runs, "legacy": {"database_bytes": legacy_bytes, "vector_payload_bytes": legacy_payload, "mean_query_ms": legacy_ms}, "versioned_blob": {"database_bytes": blob_bytes, "vector_payload_bytes": blob_payload, "mean_query_ms": blob_ms}, "migration": migration, "vector_payload_reduction_percent": round((1 - blob_payload / legacy_payload) * 100, 2)}, indent=2))
        finally:
            store.close()


if __name__ == "__main__":
    main()
