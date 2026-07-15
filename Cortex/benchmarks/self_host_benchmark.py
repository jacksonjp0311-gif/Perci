"""Measure full Cortex lifecycle with a host engine and a nested cloned engine."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
import sys

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from cortex.selftest import run_lifecycle  # noqa: E402


def median(samples: list[dict[str, object]], field: str) -> float:
    return round(statistics.median(float(sample[field]) for sample in samples), 6)


def write_svg(path: Path, baseline: dict[str, float], nested: dict[str, float]) -> None:
    metrics = [("Bootstrap seconds", baseline["bootstrap_seconds"], nested["bootstrap_seconds"]), ("Activation seconds", baseline["activation_seconds"], nested["activation_seconds"]), ("Indexed files", baseline["indexed_files"], nested["indexed_files"])]
    svg = ['<svg xmlns="http://www.w3.org/2000/svg" width="900" height="430" viewBox="0 0 900 430" role="img" aria-label="Cortex self-host benchmark before and after">', '<rect width="900" height="430" fill="#101418"/>', '<text x="42" y="45" fill="#f0f4f8" font-family="Arial" font-size="24">Cortex self-host lifecycle: before / after</text>', '<rect x="610" y="23" width="14" height="14" fill="#8b9bb4"/><text x="632" y="35" fill="#d4dce8" font-family="Arial" font-size="13">Host engine</text>', '<rect x="740" y="23" width="14" height="14" fill="#a878ff"/><text x="762" y="35" fill="#d4dce8" font-family="Arial" font-size="13">Nested cloned engine</text>']
    for index, (label, before, after) in enumerate(metrics):
        x = 42 + index * 288
        ceiling = max(before, after, 0.001)
        first = 190 * before / ceiling
        second = 190 * after / ceiling
        svg.extend([f'<text x="{x}" y="92" fill="#d4dce8" font-family="Arial" font-size="14">{label}</text>', f'<line x1="{x}" y1="315" x2="{x + 220}" y2="315" stroke="#607086"/>', f'<rect x="{x + 25}" y="{315 - first:.1f}" width="68" height="{first:.1f}" fill="#8b9bb4"/>', f'<rect x="{x + 125}" y="{315 - second:.1f}" width="68" height="{second:.1f}" fill="#a878ff"/>', f'<text x="{x + 25}" y="340" fill="#d4dce8" font-family="Arial" font-size="12">{before:.3f}</text>', f'<text x="{x + 125}" y="340" fill="#d4dce8" font-family="Arial" font-size="12">{after:.3f}</text>'])
    svg.extend(['<text x="42" y="390" fill="#aeb9c8" font-family="Arial" font-size="12">The nested engine is cloned inside an outer Cortex clone and excluded from the host inventory.</text>', '</svg>'])
    path.write_text("\n".join(svg) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "benchmarks" / "results")
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    baseline = [run_lifecycle(ROOT, nested_engine=False) for _ in range(args.runs)]
    nested = [run_lifecycle(ROOT, nested_engine=True) for _ in range(args.runs)]
    result = {
        "schema_version": "1.0",
        "workload": "Cortex cloned as host; a second Cortex clone bootstraps and activates the host.",
        "runs": args.runs,
        "host_engine": {"bootstrap_seconds": median(baseline, "bootstrap_seconds"), "activation_seconds": median(baseline, "activation_seconds"), "indexed_files": median(baseline, "indexed_files"), "certificate_verified": all(sample["certificate"] == "verified" for sample in baseline)},
        "nested_cloned_engine": {"bootstrap_seconds": median(nested, "bootstrap_seconds"), "activation_seconds": median(nested, "activation_seconds"), "indexed_files": median(nested, "indexed_files"), "certificate_verified": all(sample["certificate"] == "verified" for sample in nested), "nested_engine_excluded": all(bool(sample["nested_engine_excluded"]) for sample in nested)},
        "samples": {"host_engine": baseline, "nested_cloned_engine": nested},
    }
    json_path = args.output_dir / "self_host_before_after.json"
    chart_path = args.output_dir / "self_host_before_after.svg"
    json_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    write_svg(chart_path, result["host_engine"], result["nested_cloned_engine"])
    print(json.dumps({"json": str(json_path), "chart": str(chart_path), **result}, indent=2))


if __name__ == "__main__":
    main()
