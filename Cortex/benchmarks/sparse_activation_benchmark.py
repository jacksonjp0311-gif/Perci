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

from cortex.bootstrap import bootstrap_repository
from cortex.config import ensure_home
from cortex.neuron import activate_interlink
from cortex.retrieval import query
from cortex.store import Store


def build_repository(root: Path, count: int) -> None:
    (root / "README.md").write_text(
        "# Synthetic Agent Repository\n\n## Pipeline\n\nA linked module pipeline for sparse activation benchmarking.\n",
        encoding="utf-8",
    )
    tests = root / "tests"
    tests.mkdir()
    for index in range(count):
        next_index = (index + 1) % count
        (root / f"module_{index:04d}.py").write_text(
            f"from module_{next_index:04d} import step as next_step\n\n"
            f"def step(value: int) -> int:\n"
            f"    return next_step(value) if value < 0 else value + {index}\n",
            encoding="utf-8",
        )
        if index % 25 == 0:
            (tests / f"test_module_{index:04d}.py").write_text(
                f"from module_{index:04d} import step\n\n"
                f"def test_step_{index:04d}():\n"
                f"    assert step(1) == {index + 1}\n",
                encoding="utf-8",
            )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--files", type=int, default=250)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as temp:
        base = Path(temp)
        repo = base / "repo"
        repo.mkdir()
        build_repository(repo, args.files)
        home = ensure_home(base / "home")
        store = Store(home / "cortex.db")
        try:
            start = time.perf_counter()
            bootstrap = bootstrap_repository(home, store, repo, "SyntheticRepo")
            bootstrap_seconds = time.perf_counter() - start

            task = f"Trace module_{args.files // 2:04d} through the linked pipeline"
            hits = query(store, "SyntheticRepo", task, limit=24)
            start = time.perf_counter()
            activation = activate_interlink(
                store,
                "SyntheticRepo",
                task,
                hits,
                max_depth=2,
                max_nodes=64,
                plasticity_enabled=False,
                governance_mode="read_only",
            )
            activation_seconds = time.perf_counter() - start
            print(
                json.dumps(
                    {
                        "files_requested": args.files,
                        "indexed_nodes": bootstrap["neural_interlink"]["nodes"],
                        "synapses": bootstrap["neural_interlink"]["synapses"],
                        "bootstrap_seconds": round(bootstrap_seconds, 6),
                        "activation_seconds": round(activation_seconds, 6),
                        "activation_metrics": activation.metrics,
                        "state_hash": activation.state_hash,
                    },
                    indent=2,
                )
            )
        finally:
            store.close()


if __name__ == "__main__":
    main()
