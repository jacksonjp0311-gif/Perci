#!/usr/bin/env python3
"""Compare an isolated PERCPHR1 dialogue candidate with the active field."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROBE = ROOT / "scripts" / "native_probe.py"


def load_questions(path: Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        if isinstance(value, dict) and str(value.get("prompt", "")).strip():
            rows.append(value)
    if not rows:
        raise ValueError(f"no held-out questions in {path}")
    return rows


def run_probe(
    questions: Path,
    tag: str,
    phrase: Path | None,
    count: int,
) -> tuple[dict[str, object], Path]:
    transcript = ROOT / "models" / "candidates" / f"native-probe-{tag}.jsonl"
    summary = ROOT / "models" / "candidates" / f"native-probe-{tag}-summary.json"
    command = [
        sys.executable,
        str(PROBE),
        "--tag",
        tag,
        "--count",
        str(count),
        "--questions-file",
        str(questions),
        "--output",
        str(transcript),
        "--summary",
        str(summary),
    ]
    if phrase is not None:
        command.extend(["--phrase-weights", str(phrase)])
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        print(result.stdout, file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        raise SystemExit(result.returncode)
    return json.loads(summary.read_text(encoding="utf-8")), transcript


def expected_metrics(transcript: Path, questions: list[dict[str, object]]) -> dict[str, object]:
    rows = [json.loads(line) for line in transcript.read_text(encoding="utf-8").splitlines() if line.strip()]
    passed = 0
    checked = 0
    for question, row in zip(questions, rows):
        required = [str(item).lower() for item in question.get("required", [])]
        if not required:
            continue
        checked += 1
        answer = str(row.get("response", "")).lower()
        if all(token in answer for token in required):
            passed += 1
    return {
        "required_cases": checked,
        "required_passed": passed,
        "required_rate": round(passed / checked, 4) if checked else None,
        "responses": len(rows),
    }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def evidence_summary(
    baseline_summary: dict[str, object],
    candidate_summary: dict[str, object],
    baseline_required: dict[str, object],
    candidate_required: dict[str, object],
    questions: Path,
) -> dict[str, object]:
    """Attach interpretation metadata without turning metrics into truth.

    The comparison is a paired, single-node probe. This makes the observed
    execution evidence explicit while disclosing that both arms share one
    held-out question file. The summary is descriptive only; promotion remains
    a separate human-authorized operation.
    """
    metrics = {
        "required_followup_rate": (
            0.45,
            baseline_required.get("required_rate") is not None
            and candidate_required.get("required_rate") is not None,
        ),
        "topic_binding_rate": (
            0.35,
            "topic_binding_rate" in baseline_summary
            and "topic_binding_rate" in candidate_summary,
        ),
        "duplicate_response_rate": (
            0.20,
            "duplicate_responses" in baseline_summary
            and "duplicate_responses" in candidate_summary,
        ),
    }
    declared = sum(weight for weight, _ in metrics.values())
    observed = sum(weight for weight, present in metrics.values() if present)
    return {
        "topology": "single_node",
        "evidence_coverage": round(observed / declared, 4) if declared else 0.0,
        "execution_evidence_coverage": 1.0 if observed == declared else 0.0,
        "metrics": {
            name: {"declared_weight": weight, "observed": present}
            for name, (weight, present) in metrics.items()
        },
        "proxy_findings": [
            {
                "kind": "shared_holdout_fixture",
                "source": str(questions),
                "disclosure": "paired baseline/candidate comparison; not independent evidence",
            }
        ],
        "policy": {
            "requested": "hold_until_broader_gates",
            "effective": "hold",
            "reason": "candidate comparison is descriptive; promotion requires separate human authorization",
        },
        "distributed_consensus_claimed": False,
        "coherence_is_not_truth": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--questions",
        type=Path,
        default=ROOT / "training" / "dialogue-continuity-v1-heldout.jsonl",
    )
    parser.add_argument("--phrase-weights", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    questions = args.questions.resolve()
    phrase = args.phrase_weights.resolve()
    if not phrase.is_file():
        raise SystemExit(f"missing phrase candidate: {phrase}")
    rows = load_questions(questions)
    count = len(rows)
    tag_prefix = questions.stem.replace("-heldout", "")
    baseline_summary, baseline_transcript = run_probe(
        questions, f"{tag_prefix}-active", None, count
    )
    candidate_summary, candidate_transcript = run_probe(
        questions, f"{tag_prefix}-candidate", phrase, count
    )
    baseline_expected = expected_metrics(baseline_transcript, rows)
    candidate_expected = expected_metrics(candidate_transcript, rows)
    candidate_not_worse = (
        candidate_expected["required_rate"] is not None
        and baseline_expected["required_rate"] is not None
        and candidate_expected["required_rate"] >= baseline_expected["required_rate"]
        and float(candidate_summary.get("topic_binding_rate", 0.0))
        >= float(baseline_summary.get("topic_binding_rate", 0.0))
        and int(candidate_summary.get("duplicate_responses", 0))
        <= int(baseline_summary.get("duplicate_responses", 0))
    )
    receipt = {
        "schema": "perci.dialogue-candidate-evaluation.v1",
        "questions": str(questions),
        "questions_sha256": sha256(questions),
        "phrase_candidate": str(phrase),
        "phrase_candidate_sha256": sha256(phrase),
        "baseline": {"summary": baseline_summary, "required": baseline_expected},
        "candidate": {"summary": candidate_summary, "required": candidate_expected},
        "candidate_not_worse": candidate_not_worse,
        "evidence": evidence_summary(
            baseline_summary,
            candidate_summary,
            baseline_expected,
            candidate_expected,
            questions,
        ),
        "promotion": "HOLD",
        "promote_recommended": False,
        "reason": "Human authorization and broader gates are still required even when this small held-out set improves.",
    }
    output = (args.output or ROOT / "models" / "candidates" / "evaluation-dialogue-continuity-v1.json").resolve()
    output.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
