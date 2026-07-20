#!/usr/bin/env python3
"""HYDRA Injector ↔ Perci evolve bridge.

Closes the weak link in probe → analyze → inject:

  probe (hardness / adversarial / BRPC)
    → analyze (weak factors + failed cases)
    → HYDRA anchor (marker slots)
    → inject plan (reviewable diff / residual seal)
    → retract unsafe scope (HYDRA policy)
    → seal (session ledger + receipt)
    → re-gate (hardness / transfer / BRPC)

HYDRA does **not** replace Perci operators or auto-promote `.pwgt`.
It makes **code / script / pack-side injections** governed:

  No anchor, no injection.
  No boundary, no promotion.
  No seal, no trust.

Requires: `hydra-inject` on PATH (pip install -e path/to/hydra-injector).

Usage:
  python scripts/hydra_perci_bridge.py status
  python scripts/hydra_perci_bridge.py plan
  python scripts/hydra_perci_bridge.py field
  python scripts/hydra_perci_bridge.py evolve --dry-run
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "models" / "candidates" / "hydra-bridge"
SPECS_DIR = OUT_DIR / "specs"
LEDGER = OUT_DIR / "hydra_sessions.jsonl"
RECEIPT = OUT_DIR / "hydra-perci-bridge-latest.json"

HARDNESS = ROOT / "models" / "candidates" / "evaluation-hardness-v1.json"
PROBE = ROOT / "models" / "candidates" / "adversarial-probe-brpc-latest.json"
BRPC = ROOT / "models" / "candidates" / "brpc-perci-receipt-latest.json"
SURGICAL = ROOT / "models" / "candidates" / "interact-evolve-loop-latest.json"

# Factor → preferred HYDRA slot / target (Python surfaces first — .rs needs allow_extensions).
FACTOR_SLOTS: dict[str, dict[str, str]] = {
    "P": {
        "slot": "hardness_seed",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=hardness_seed profile=library",
        "layer": "hardness / predictive",
    },
    "M": {
        "slot": "geometry_note",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=geometry_note profile=library",
        "layer": "manifold / SoftCascade geometry",
    },
    "B": {
        "slot": "boundary_lock",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=boundary_lock profile=library",
        "layer": "refuse / authorize / no silent promote",
    },
    "R": {
        "slot": "coord_note",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=coord_note profile=library",
        "layer": "multi-engine coordination",
    },
    "K": {
        "slot": "recovery_note",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=recovery_note profile=library",
        "layer": "fail→repair recovery",
    },
    "U": {
        "slot": "latency_note",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=latency_note profile=library",
        "layer": "resource / latency (code path, not teach)",
    },
    "D": {
        "slot": "continuity_note",
        "target": "scripts/hydra_perci_bridge.py",
        "marker": "# HYDRA-INJECT:slot name=continuity_note profile=library",
        "layer": "follow-up / setup continuity",
    },
}

CLAIM_BOUNDARY = [
    "HYDRA bridge is governance for injection, not consciousness",
    "never auto-promote .pwgt from HYDRA seal",
    "code-apply only after review + tests",
    "residual field metrics are engineering telemetry, not physics claims",
]


def read_json(path: Path) -> dict | list | None:
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def hydra_bin() -> str | None:
    return shutil.which("hydra-inject")


def run_hydra(args: list[str], timeout: float = 120.0) -> dict[str, Any]:
    bin_path = hydra_bin()
    if not bin_path:
        return {"ok": False, "error": "hydra-inject not on PATH"}
    try:
        proc = subprocess.run(
            [bin_path, *args],
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
        )
        out = (proc.stdout or "") + (proc.stderr or "")
        return {
            "ok": proc.returncode == 0,
            "returncode": proc.returncode,
            "stdout": proc.stdout or "",
            "stderr": proc.stderr or "",
            "tail": out[-2000:],
        }
    except Exception as exc:  # noqa: BLE001
        return {"ok": False, "error": str(exc)}


def collect_failures() -> list[dict[str, Any]]:
    fails: list[dict[str, Any]] = []
    hardness = read_json(HARDNESS)
    if isinstance(hardness, dict):
        for c in hardness.get("cases") or []:
            if not c.get("pass"):
                fails.append(
                    {
                        "source": "hardness",
                        "id": c.get("id"),
                        "prompt": c.get("prompt"),
                        "missing": c.get("missing_required"),
                        "forbidden": c.get("forbidden_hits"),
                        "factor": map_case_to_factor(str(c.get("capability") or ""), str(c.get("prompt") or "")),
                    }
                )
    probe = read_json(PROBE)
    if isinstance(probe, dict):
        for c in probe.get("cases") or []:
            if not c.get("pass"):
                fails.append(
                    {
                        "source": "adversarial_probe",
                        "id": c.get("id"),
                        "prompt": c.get("prompt"),
                        "missing": c.get("missing"),
                        "forbidden": c.get("forbidden_hits"),
                        "factor": map_case_to_factor(str(c.get("capability") or ""), str(c.get("prompt") or "")),
                    }
                )
    surgical = read_json(SURGICAL)
    if isinstance(surgical, dict):
        for p in surgical.get("persistent_failures") or []:
            fails.append(
                {
                    "source": "surgical",
                    "id": p.get("kind"),
                    "prompt": p.get("prompt"),
                    "missing": (p.get("analysis") or {}).get("issues"),
                    "forbidden": [],
                    "factor": "K",
                }
            )
    return fails


def map_case_to_factor(capability: str, prompt: str) -> str:
    cap = capability.lower()
    p = prompt.lower()
    if "geometry" in cap or "geometry" in p or "band" in p or "manifold" in p:
        return "M"
    if "govern" in cap or "promot" in p or "authorize" in p:
        return "B"
    if "abstent" in cap or "conscious" in p or "refuse" in p or "invent" in p:
        return "B"
    if "followup" in cap or "cryptic" in p or "plain sentence" in p:
        return "D"
    if "transfer" in cap or "entity" in p or "trust" in p and "lag" in p:
        return "P"
    if "recover" in p or "hardness fail" in p:
        return "K"
    if "latency" in p or "resource" in p:
        return "U"
    return "R"


def soft_brpc_factors() -> list[tuple[str, float]]:
    brpc = read_json(BRPC)
    if not isinstance(brpc, dict):
        return []
    factors = brpc.get("factors") or {}
    soft: list[tuple[str, float]] = []
    for k, row in factors.items():
        v = row.get("value")
        if v is None:
            soft.append((k, 0.0))
        elif float(v) < 0.92:
            soft.append((k, float(v)))
    soft.sort(key=lambda kv: kv[1])
    return soft


def build_codeweave_spec(
    factor: str,
    *,
    fail: dict | None = None,
    value: float | None = None,
) -> dict[str, Any]:
    meta = FACTOR_SLOTS[factor]
    rationale_parts = [
        f"BRPC factor {factor} ({meta['layer']}) needs governed injection.",
        "HYDRA: anchor→inject→retract→seal. Never auto-promote .pwgt.",
    ]
    if value is not None:
        rationale_parts.append(f"Observed factor value={value:.3f}.")
    if fail:
        rationale_parts.append(
            f"From {fail.get('source')} {fail.get('id')}: {str(fail.get('prompt') or '')[:120]}"
        )
    # Injection is a reviewable note block — human/AI replaces with real operator patch later.
    note = (
        f"\n# hydra-evolve: factor={factor} layer={meta['layer']}\n"
        f"# status=candidate_patch_anchor\n"
        f"# action=repair_owning_engine_not_densify_bitwork\n"
        f"# claim_boundary=no_auto_promote_pwgt\n"
    )
    return {
        "root": str(ROOT),
        "target_file": meta["target"],
        "marker": meta["marker"],
        "name": meta["slot"],
        "profile": "library",
        "mode": "after",
        "code": note,
        "allow_extensions": [".py", ".md", ".json", ".toml", ".yml", ".yaml", ".txt", ".rs"],
        "max_bytes": 16000,
        "rationale": " ".join(rationale_parts),
        "non_claims": CLAIM_BOUNDARY,
    }


def brpc_to_field_spec() -> dict[str, Any]:
    """Map BRPC factors to a 3x3 residual field for HYDRA seal telemetry."""
    order = ["P", "M", "B", "R", "K", "U", "D"]
    brpc = read_json(BRPC) if BRPC.is_file() else {}
    factors = (brpc or {}).get("factors") or {}
    # Inject residual stress: weak factors → higher amplitude (1 − v).
    # Strong factors stay near zero so the seal does not over-smooth to trivial.
    vals = []
    for k in order:
        v = (factors.get(k) or {}).get("value")
        v = 0.5 if v is None else max(0.0, min(1.0, float(v)))
        vals.append(max(0.02, 1.0 - v))
    # pad to 9 cells with mean residual
    while len(vals) < 9:
        vals.append(sum(vals) / max(1, len(vals)))
    field = [
        [vals[0], vals[1], vals[2]],
        [vals[3], vals[4], vals[5]],
        [vals[6], vals[7], vals[8]],
    ]
    # admissible everywhere
    mask = [[1, 1, 1], [1, 1, 1], [1, 1, 1]]
    return {
        "mask": mask,
        "field": field,
        "config": {
            "target_volume": 1.0,
            "retract_fraction": 0.20,
            "pin_strength": 0.35,
            "boundary_band": 1,
            "seal_steps": 8,
            "seal_alpha": 0.35,
        },
        "robustness": True,
        "meta": {
            "source": "perci.brpc-receipt.v0.1",
            "factor_order": order + ["pad", "pad"],
            "claim_boundary": CLAIM_BOUNDARY,
        },
    }


def ensure_dirs() -> None:
    SPECS_DIR.mkdir(parents=True, exist_ok=True)
    OUT_DIR.mkdir(parents=True, exist_ok=True)


def cmd_status(_: argparse.Namespace) -> int:
    bin_path = hydra_bin()
    fails = collect_failures()
    soft = soft_brpc_factors()
    print("HYDRA ↔ Perci bridge status")
    print(f"  hydra-inject: {bin_path or 'NOT FOUND'}")
    print(f"  hardness fails: {sum(1 for f in fails if f['source']=='hardness')}")
    print(f"  probe fails: {sum(1 for f in fails if f['source']=='adversarial_probe')}")
    print(f"  surgical persistent: {sum(1 for f in fails if f['source']=='surgical')}")
    print(f"  soft BRPC factors: {soft[:5]}")
    markers = run_hydra(["markers", str(ROOT), "--slots-only", "--format", "json"])
    if markers.get("ok"):
        try:
            data = json.loads(markers.get("stdout") or "[]")
            print(f"  HYDRA slots in Perci: {len(data)}")
        except json.JSONDecodeError:
            print("  HYDRA slots: (see markers output)")
            print((markers.get("stdout") or "")[:400])
    else:
        print(f"  markers scan: {markers.get('error') or markers.get('tail', '')[:200]}")
    print("  claim: no auto-promote .pwgt; code-apply is explicit")
    return 0 if bin_path else 2


def cmd_field(_: argparse.Namespace) -> int:
    ensure_dirs()
    if not hydra_bin():
        print("hydra-inject not found", file=sys.stderr)
        return 2
    spec = brpc_to_field_spec()
    path = SPECS_DIR / "brpc_field_spec.json"
    path.write_text(json.dumps(spec, indent=2), encoding="utf-8")
    print(f"wrote {path}")
    result = run_hydra(["run", str(path), "--format", "json"])
    print(result.get("stdout") or result.get("tail") or result.get("error"))
    if result.get("ok"):
        # try robustness lightly
        rob = run_hydra(
            ["robustness", str(path), "--trials", "6", "--noise-scale", "0.03"],
            timeout=180,
        )
        print("--- robustness ---")
        print((rob.get("stdout") or rob.get("tail") or "")[:1500])
    receipt = {
        "schema": "perci.hydra-bridge.field.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "spec": str(path.relative_to(ROOT)),
        "run_ok": result.get("ok"),
        "claim_boundary": CLAIM_BOUNDARY,
    }
    (OUT_DIR / "field-run-latest.json").write_text(
        json.dumps(receipt, indent=2) + "\n", encoding="utf-8"
    )
    return 0 if result.get("ok") else 1


def cmd_plan(args: argparse.Namespace) -> int:
    ensure_dirs()
    if not hydra_bin():
        print("hydra-inject not found — install HYDRA-Injector (pip install -e …)", file=sys.stderr)
        return 2

    fails = collect_failures()
    soft = soft_brpc_factors()
    planned: list[dict[str, Any]] = []

    # Prefer hard fails; else soft BRPC factors
    work: list[tuple[str, dict | None, float | None]] = []
    for f in fails[: args.limit]:
        work.append((f["factor"], f, None))
    if not work:
        for k, v in soft[: args.limit]:
            work.append((k, None, v))
    if not work:
        # still plan a boundary lock note so the pipeline is exercised
        work.append(("B", None, 1.0))

    for factor, fail, value in work:
        if factor not in FACTOR_SLOTS:
            factor = "R"
        spec = build_codeweave_spec(factor, fail=fail, value=value)
        spec_path = SPECS_DIR / f"codeweave_{factor}_{len(planned)+1}.json"
        spec_path.write_text(json.dumps(spec, indent=2), encoding="utf-8")
        verify = run_hydra(["code-verify", str(spec_path)])
        plan = run_hydra(["code-plan", str(spec_path), "--format", "json", "--ledger", str(LEDGER)])
        planned.append(
            {
                "factor": factor,
                "spec": str(spec_path.relative_to(ROOT)),
                "verify_ok": verify.get("ok"),
                "plan_ok": plan.get("ok"),
                "verify_tail": (verify.get("tail") or "")[-400:],
                "plan_tail": (plan.get("tail") or "")[-600:],
                "fail": fail,
                "value": value,
            }
        )
        mark = "ok" if plan.get("ok") else "fail"
        print(f"[{mark}] factor={factor} verify={verify.get('ok')} plan={plan.get('ok')} → {spec_path.name}")

    receipt = {
        "schema": "perci.hydra-bridge.plan.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "mode": "plan_only",
        "failures_seen": len(fails),
        "soft_brpc": soft,
        "planned": planned,
        "claim_boundary": CLAIM_BOUNDARY,
        "automatic_promotion": False,
        "next": [
            "Review diffs under models/candidates/hydra-bridge/",
            "hydra-inject code-apply <spec> --dry-run",
            "Only after review: code-apply + cargo test / hardness (never weight promote)",
        ],
    }
    RECEIPT.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"receipt: {RECEIPT}")
    return 0 if all(p.get("plan_ok") or p.get("verify_ok") for p in planned) or planned else 1


def cmd_evolve(args: argparse.Namespace) -> int:
    """plan → dry-run apply (default) → optional re-gate."""
    rc = cmd_plan(args)
    if not hydra_bin():
        return 2
    ensure_dirs()
    specs = sorted(SPECS_DIR.glob("codeweave_*.json"))
    apply_rows = []
    for spec_path in specs[-args.limit :]:
        cmd = ["code-apply", str(spec_path)]
        if args.dry_run:
            cmd.append("--dry-run")
        if args.test:
            cmd.extend(["--test", args.test])
            if args.rollback_on_test_fail:
                cmd.append("--rollback-on-test-fail")
        result = run_hydra(cmd, timeout=300)
        apply_rows.append(
            {
                "spec": str(spec_path.relative_to(ROOT)),
                "dry_run": args.dry_run,
                "ok": result.get("ok"),
                "tail": (result.get("tail") or "")[-500:],
            }
        )
        print(f"apply[{'dry' if args.dry_run else 'WRITE'}] {spec_path.name} ok={result.get('ok')}")

    regate = None
    if args.regate:
        print("=== re-gate: hardness ===")
        h = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "evaluate_hardness.py")],
            cwd=str(ROOT),
        )
        print("=== re-gate: BRPC ===")
        b = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "brpc_perci_receipt.py")],
            cwd=str(ROOT),
        )
        regate = {"hardness_rc": h.returncode, "brpc_rc": b.returncode}

    receipt = {
        "schema": "perci.hydra-bridge.evolve.v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "dry_run": args.dry_run,
        "plan_rc": rc,
        "apply": apply_rows,
        "regate": regate,
        "claim_boundary": CLAIM_BOUNDARY,
        "automatic_promotion": False,
    }
    path = OUT_DIR / "evolve-latest.json"
    path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(f"evolve receipt: {path}")
    if args.dry_run:
        print("dry-run only — no files written; re-run without --dry-run after human review")
    return 0 if all(r.get("ok") for r in apply_rows) or not apply_rows else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status", help="Show HYDRA availability and Perci fail mass")
    sub.add_parser("field", help="Map BRPC factors → HYDRA residual field run")

    p_plan = sub.add_parser("plan", help="Build codeweave specs from fails/soft BRPC; plan diffs")
    p_plan.add_argument("--limit", type=int, default=3)

    p_ev = sub.add_parser("evolve", help="Plan + code-apply (default dry-run)")
    p_ev.add_argument("--limit", type=int, default=3)
    p_ev.add_argument(
        "--dry-run",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Default true: never write without --no-dry-run",
    )
    p_ev.add_argument("--test", type=str, default="", help="Optional post-apply test command")
    p_ev.add_argument("--rollback-on-test-fail", action="store_true")
    p_ev.add_argument("--regate", action="store_true", help="Re-run hardness + BRPC after apply")

    args = parser.parse_args()
    if args.cmd == "status":
        return cmd_status(args)
    if args.cmd == "field":
        return cmd_field(args)
    if args.cmd == "plan":
        return cmd_plan(args)
    if args.cmd == "evolve":
        return cmd_evolve(args)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())


# ---------------------------------------------------------------------------
# HYDRA Codeweave anchors (library profile). Bridge injects notes after these.
# Real operator patches should target Rust; these slots prove governed inject.
# ---------------------------------------------------------------------------
# HYDRA-INJECT:slot name=hardness_seed profile=library
# HYDRA-INJECT:slot name=geometry_note profile=library
# HYDRA-INJECT:slot name=boundary_lock profile=library
# HYDRA-INJECT:slot name=coord_note profile=library
# HYDRA-INJECT:slot name=recovery_note profile=library
# HYDRA-INJECT:slot name=latency_note profile=library
# HYDRA-INJECT:slot name=continuity_note profile=library
