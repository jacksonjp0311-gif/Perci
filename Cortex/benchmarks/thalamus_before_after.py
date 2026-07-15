"""Reproducible before/after retrieval benchmark for the Thalamus routing layer."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
import sys
import tempfile
import time
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from cortex.bootstrap import bootstrap_repository  # noqa: E402
from cortex.config import ensure_home  # noqa: E402
from cortex.retrieval import query  # noqa: E402
from cortex.store import Store  # noqa: E402
from thalamus import inhibit, lane_for_hit, make_request, route  # noqa: E402


def build_repository(root: Path, count: int) -> None:
    (root / "README.md").write_text(
        "# Synthetic Routing Repository\n\nA linked module pipeline for deterministic routing tests.\n",
        encoding="utf-8",
    )
    (root / "docs").mkdir()
    (root / "docs" / "architecture.md").write_text(
        "# Architecture\n\nThe module pipeline is covered by corresponding tests.\n", encoding="utf-8"
    )
    tests = root / "tests"
    tests.mkdir()
    for index in range(count):
        next_index = (index + 1) % count
        (root / f"module_{index:04d}.py").write_text(
            f"from module_{next_index:04d} import step as next_step\n\n"
            f"def step(value: int) -> int:\n    return next_step(value) if value < 0 else value + {index}\n",
            encoding="utf-8",
        )
        if index % 25 == 0:
            (tests / f"test_module_{index:04d}.py").write_text(
                f"from module_{index:04d} import step\n\n"
                f"def test_step_{index:04d}():\n    assert step(1) == {index + 1}\n",
                encoding="utf-8",
            )


def rank_for(hits: list[Any], targets: tuple[str, ...]) -> int | None:
    for rank, hit in enumerate(hits, 1):
        if hit.path in targets:
            return rank
    return None


def summarize(samples: list[dict[str, Any]], name: str) -> dict[str, Any]:
    return {
        "name": name,
        "median_seconds": round(statistics.median(sample["seconds"] for sample in samples), 6),
        "median_target_rank": statistics.median(sample["target_rank"] or 25 for sample in samples),
        "top_3_target_recall": round(
            sum((sample["target_rank"] or 25) <= 3 for sample in samples) / len(samples), 6
        ),
        "mean_out_of_route_candidates": round(
            statistics.mean(sample["out_of_route_candidates"] for sample in samples), 3
        ),
    }


def write_svg(output: Path, baseline: dict[str, Any], routed: dict[str, Any]) -> None:
    metrics = [
        ("Median latency (ms)", baseline["median_seconds"] * 1000, routed["median_seconds"] * 1000),
        ("Target rank (lower is better)", baseline["median_target_rank"], routed["median_target_rank"]),
        ("Out-of-route candidates", baseline["mean_out_of_route_candidates"], routed["mean_out_of_route_candidates"]),
    ]
    lines = [
        '<svg xmlns="http://www.w3.org/2000/svg" width="900" height="430" viewBox="0 0 900 430" role="img" aria-labelledby="title desc">',
        '<title id="title">Thalamus routing before and after benchmark</title>',
        '<desc id="desc">Side-by-side measured results for direct hybrid retrieval and Thalamus-routed hybrid retrieval.</desc>',
        '<rect width="900" height="430" fill="#101418"/>',
        '<text x="42" y="45" fill="#f0f4f8" font-family="Arial" font-size="24">Thalamus routing: measured before / after</text>',
        '<rect x="600" y="23" width="14" height="14" fill="#8b9bb4"/><text x="622" y="35" fill="#d4dce8" font-family="Arial" font-size="13">Direct hybrid</text>',
        '<rect x="733" y="23" width="14" height="14" fill="#55c2a6"/><text x="755" y="35" fill="#d4dce8" font-family="Arial" font-size="13">Thalamus-routed</text>',
    ]
    for index, (label, before, after) in enumerate(metrics):
        origin_x = 42 + index * 288
        max_value = max(before, after, 0.001)
        before_height = 190 * before / max_value
        after_height = 190 * after / max_value
        lines.extend([
            f'<text x="{origin_x}" y="92" fill="#d4dce8" font-family="Arial" font-size="14">{label}</text>',
            f'<line x1="{origin_x}" y1="315" x2="{origin_x + 220}" y2="315" stroke="#607086"/>',
            f'<rect x="{origin_x + 25}" y="{315 - before_height:.1f}" width="68" height="{before_height:.1f}" fill="#8b9bb4"/>',
            f'<rect x="{origin_x + 125}" y="{315 - after_height:.1f}" width="68" height="{after_height:.1f}" fill="#55c2a6"/>',
            f'<text x="{origin_x + 25}" y="340" fill="#d4dce8" font-family="Arial" font-size="12">{before:.3f}</text>',
            f'<text x="{origin_x + 125}" y="340" fill="#d4dce8" font-family="Arial" font-size="12">{after:.3f}</text>',
        ])
    lines.extend([
        '<text x="42" y="390" fill="#aeb9c8" font-family="Arial" font-size="12">Fixed synthetic repository; values are measured locally. Lower is better for target rank and out-of-route candidates.</text>',
        '</svg>',
    ])
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", type=int, default=250)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "benchmarks" / "results")
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    baseline_samples: list[dict[str, Any]] = []
    routed_samples: list[dict[str, Any]] = []
    module = args.files // 2
    target = f"module_{module:04d}.py"
    relevant_paths = (f"tests/test_module_{module:04d}.py", f"module_{(module - 1) % args.files:04d}.py")
    task = f"Fix the failing test in {target} and trace the linked pipeline"

    with tempfile.TemporaryDirectory() as temporary:
        base = Path(temporary)
        repo = base / "repo"
        repo.mkdir()
        build_repository(repo, args.files)
        home = ensure_home(base / "home")
        store = Store(home / "cortex.db")
        try:
            bootstrap_repository(home, store, repo, "RoutingBenchmark")
            repository = store.repo("RoutingBenchmark")
            assert repository is not None
            for _ in range(args.runs):
                started = time.perf_counter()
                baseline_hits = query(store, "RoutingBenchmark", task, limit=24)
                baseline_seconds = time.perf_counter() - started
                baseline_samples.append({
                    "seconds": baseline_seconds,
                    "target_rank": rank_for(baseline_hits, relevant_paths),
                    "out_of_route_candidates": sum(lane_for_hit(hit) not in {"source", "tests", "failures", "runtime", "git", "structure"} for hit in baseline_hits[:8]),
                })

                started = time.perf_counter()
                plan = route(make_request(repository, task, 1200), manifest_current=True)
                routed_hits = inhibit(
                    query(store, "RoutingBenchmark", task, limit=24),
                    plan.lane_weights,
                    min_lane_relevance=0.25,
                )
                routed_seconds = time.perf_counter() - started
                routed_samples.append({
                    "seconds": routed_seconds,
                    "target_rank": rank_for(routed_hits, relevant_paths),
                    "out_of_route_candidates": sum(hit.metadata["thalamus"]["lane"] not in {"source", "tests", "failures", "runtime", "git", "structure"} for hit in routed_hits[:8]),
                })
        finally:
            store.close()

    result = {
        "schema_version": "1.0",
        "workload": {
            "files": args.files,
            "runs": args.runs,
            "task": task,
            "target": target,
            "relevant_paths": relevant_paths,
        },
        "baseline": summarize(baseline_samples, "direct_hybrid_retrieval"),
        "thalamus_routed": summarize(routed_samples, "thalamus_routed_hybrid_retrieval"),
        "samples": {"baseline": baseline_samples, "thalamus_routed": routed_samples},
    }
    json_path = args.output_dir / "thalamus_before_after.json"
    svg_path = args.output_dir / "thalamus_before_after.svg"
    json_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    write_svg(svg_path, result["baseline"], result["thalamus_routed"])
    print(json.dumps({"json": str(json_path), "chart": str(svg_path), **result}, indent=2))


if __name__ == "__main__":
    main()
