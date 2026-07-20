#!/usr/bin/env python3
"""BRPC v0.1 → Perci gate receipt (candidate control telemetry).

Maps existing Perci evaluation artifacts onto Boundary-Regulated Predictive
Coherence factors and emits a governed receipt. Does **not** promote weights.

Core (candidate, engineering-only):

    C_BRPC  = ∏_i  v_i ^ w_i^{eff}
    ΔΦ_BRPC = −log(C_BRPC) = Σ_i δφ_i
    Ω_BRPC  = 1 / (1 + ΔΦ_BRPC)

Factors {P, M, B, R, K, U, D}:
  P  predictive adequacy     ← hardness + transfer pass rates
  M  manifold adherence      ← cross-domain / transfer caps + SoftCascade align
  B  boundary suitability    ← abstention + governance + forbidden-hit inverse
  R  coordination            ← capability balance + transfer suite
  K  recovery margin         ← surgical re-ask / still-fail inverse
  U  resource efficiency     ← latency distribution
  D  continuity              ← follow-up binding + setup threads + dialogue gate

Claim boundary (locked):
  · candidate control theory for adaptive systems — not consciousness
  · not a universal field; not an established cross-domain law
  · C without evidence coverage is incomplete
  · never auto-promote .pwgt from this score

Usage:
  python scripts/brpc_perci_receipt.py
  python scripts/brpc_perci_receipt.py --run-gates
  python scripts/brpc_perci_receipt.py --json-out models/candidates/brpc-perci-receipt-latest.json
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_HARDNESS = ROOT / "models" / "candidates" / "evaluation-hardness-v1.json"
DEFAULT_SURGICAL = ROOT / "models" / "candidates" / "interact-evolve-loop-latest.json"
DEFAULT_DIALOGUE = ROOT / "models" / "candidates" / "evaluation-v4-dialogue.json"
DEFAULT_OUT = ROOT / "models" / "candidates" / "brpc-perci-receipt-latest.json"
DEFAULT_BINARY = ROOT / "target" / "release" / "perci.exe"

# Effective weights (sum ≈ 1). Tunable; disclosed in receipt.
DEFAULT_WEIGHTS: dict[str, float] = {
    "P": 0.18,  # predictive
    "M": 0.14,  # manifold
    "B": 0.18,  # boundary
    "R": 0.12,  # coordination
    "K": 0.14,  # recovery
    "U": 0.10,  # resource
    "D": 0.14,  # continuity
}

FACTOR_NAMES = {
    "P": "predictive_adequacy",
    "M": "manifold_adherence",
    "B": "boundary_suitability",
    "R": "coordination",
    "K": "recovery_margin",
    "U": "resource_efficiency",
    "D": "continuity",
}

# Floor so a single missing factor does not force C→0 when partially observed.
V_FLOOR = 0.05
# H7-style band (dynamic horizon proxy for software-agent domain).
H7_MU = 0.82
H7_EPS = 0.08


def clamp01(x: float) -> float:
    try:
        return max(0.0, min(1.0, float(x)))
    except (TypeError, ValueError):
        return 0.0


def read_json(path: Path) -> dict | list | None:
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def pass_rate(passed: int | None, total: int | None) -> float | None:
    if total is None or total <= 0 or passed is None:
        return None
    return clamp01(passed / total)


def cap_rate(by_cap: dict | None, name: str) -> float | None:
    if not by_cap or name not in by_cap:
        return None
    row = by_cap[name] or {}
    return pass_rate(row.get("passed"), row.get("total"))


def mean_present(values: list[float | None]) -> float | None:
    xs = [v for v in values if v is not None]
    if not xs:
        return None
    return clamp01(sum(xs) / len(xs))


def latencies_from_hardness(cases: list[dict]) -> list[float]:
    out: list[float] = []
    for c in cases:
        ms = c.get("latency_ms")
        if isinstance(ms, (int, float)) and ms >= 0:
            out.append(float(ms))
    return out


def latencies_from_surgical(rows: list[dict]) -> list[float]:
    out: list[float] = []
    for r in rows:
        for key in ("ask1", "ask2"):
            ms = (r.get(key) or {}).get("ms")
            if isinstance(ms, (int, float)) and ms >= 0:
                out.append(float(ms))
    return out


def efficiency_from_latencies(ms_list: list[float]) -> float | None:
    """Map median latency → [0,1].

    Calibrated for warm daemon SoftCascade turns:
      ~350–450ms → ≈0.90–0.92
      ~1000ms     → ≈0.62
      ~1600ms     → ≈0.35
    Uses median so rare cold outliers do not dominate.
    """
    if not ms_list:
        return None
    med = statistics.median(ms_list)
    # Center near 1.2s so sub-500ms warm path sits in the healthy band.
    score = 1.0 / (1.0 + math.exp((med - 1200.0) / 400.0))
    # Mild p90 penalty when tail is heavy (cold starts / checklist rewrite spikes).
    if len(ms_list) >= 8:
        ordered = sorted(ms_list)
        p90 = ordered[int(0.9 * (len(ordered) - 1))]
        if p90 > 1500:
            score *= 0.92
        if p90 > 2500:
            score *= 0.85
    return clamp01(score)


def forbidden_rate(cases: list[dict]) -> float | None:
    if not cases:
        return None
    hits = sum(1 for c in cases if c.get("forbidden_hits"))
    return clamp01(1.0 - hits / len(cases))


def softcascade_align_score(transfer_checks: list[dict]) -> float | None:
    if not transfer_checks:
        return None
    last = transfer_checks[-1]
    if not last.get("ok"):
        return 0.35
    tail = str(last.get("tail") or "").lower()
    if "softcascade" in tail and "pass" in tail:
        return 0.95
    if "all_pass=true" in tail or "pass=" in tail:
        return 0.85
    return 0.70 if last.get("ok") else 0.35


def capability_balance(by_cap: dict | None) -> float | None:
    """Coordination proxy: 1 − normalized spread of capability pass rates."""
    if not by_cap:
        return None
    rates: list[float] = []
    for row in by_cap.values():
        r = pass_rate(row.get("passed"), row.get("total"))
        if r is not None:
            rates.append(r)
    if len(rates) < 2:
        return rates[0] if rates else None
    spread = max(rates) - min(rates)
    mean_r = sum(rates) / len(rates)
    return clamp01(mean_r * (1.0 - 0.5 * spread))


def continuity_from_surgical(rows: list[dict]) -> float | None:
    """Setup-thread cases + follow-up-ish kinds that re-ask pass."""
    if not rows:
        return None
    scored: list[float] = []
    for r in rows:
        kind = str(r.get("kind") or "")
        setup_ish = kind in {"H104", "H105", "H110", "H112"} or bool(
            (r.get("prompt") or "").lower().startswith("what should i do next")
            or "say that again" in (r.get("prompt") or "").lower()
            or "sounds cyptic" in (r.get("prompt") or "").lower()
            or "dont agree" in (r.get("prompt") or "").lower()
            or "don't agree" in (r.get("prompt") or "").lower()
        )
        if not setup_ish:
            continue
        ok2 = bool((r.get("ask2") or {}).get("ok"))
        ok1 = bool((r.get("ask1") or {}).get("ok"))
        scored.append(1.0 if ok2 else (0.4 if ok1 else 0.0))
    if not scored:
        # fall back: second_pass rate is continuity of repair under re-ask
        return None
    return clamp01(sum(scored) / len(scored))


def factor_metric(
    value: float | None,
    *,
    status: str,
    source: str,
    notes: str = "",
) -> dict[str, Any]:
    if value is None:
        return {
            "value": None,
            "status": status if status != "observed" else "missing",
            "source": source,
            "notes": notes or "no evidence",
        }
    return {
        "value": clamp01(value),
        "status": status,
        "source": source,
        "notes": notes,
    }


def build_factors(
    hardness: dict | None,
    surgical: dict | None,
    dialogue: dict | None,
) -> dict[str, dict[str, Any]]:
    h_cases = list((hardness or {}).get("cases") or [])
    by_cap = (hardness or {}).get("by_capability") or {}
    h_rate = pass_rate((hardness or {}).get("passed"), (hardness or {}).get("case_count"))
    if h_rate is None and h_cases:
        h_rate = pass_rate(sum(1 for c in h_cases if c.get("pass")), len(h_cases))

    s_first = (surgical or {}).get("first_pass_rate")
    s_second = (surgical or {}).get("second_pass_rate")
    if s_first is None and surgical:
        cyc = surgical.get("cycles") or 0
        s_first = pass_rate(surgical.get("first_pass"), cyc) if cyc else None
    if s_second is None and surgical:
        cyc = surgical.get("cycles") or 0
        s_second = pass_rate(surgical.get("second_pass"), cyc) if cyc else None

    still_fail = (surgical or {}).get("still_fail_after_reask")
    cycles = (surgical or {}).get("cycles") or 0
    still_rate = (still_fail / cycles) if cycles and still_fail is not None else None
    improved = (surgical or {}).get("improved_after_teach")
    improve_rate = (improved / cycles) if cycles and improved is not None else None

    transfer_ok = softcascade_align_score(list((surgical or {}).get("transfer_checks") or []))
    d_rate = pass_rate((dialogue or {}).get("passed"), (dialogue or {}).get("case_count"))

    # --- P predictive adequacy ---
    p = mean_present([h_rate, s_second if s_second is not None else s_first, transfer_ok])
    p_status = "observed" if p is not None else "missing"
    p_src = "hardness+surgical+transfer"

    # --- M manifold adherence ---
    m = mean_present(
        [
            cap_rate(by_cap, "cross_domain_synthesis"),
            cap_rate(by_cap, "transfer_vs_template"),
            cap_rate(by_cap, "relational_inquiry"),
            transfer_ok,
        ]
    )
    m_status = "observed" if m is not None else "missing"

    # --- B boundary suitability ---
    b = mean_present(
        [
            cap_rate(by_cap, "honest_abstention"),
            cap_rate(by_cap, "governed_learning_loop"),
            forbidden_rate(h_cases),
            # never auto-promote is good boundary health when promote_recommended is false
            (
                1.0
                if surgical and surgical.get("promote_recommended") is False
                else (0.7 if surgical else None)
            ),
        ]
    )
    b_status = "observed" if b is not None else "missing"

    # --- R coordination ---
    r = mean_present([capability_balance(by_cap), transfer_ok, s_second])
    r_status = "observed" if r is not None else "missing"

    # --- K recovery margin ---
    if still_rate is not None:
        k_core = clamp01(1.0 - still_rate)
    else:
        k_core = s_second
    # Bonus if teach ever improved fails (when there were fails); neutral when already green.
    if improve_rate is not None and still_rate is not None and still_rate > 0:
        k = clamp01(0.7 * (k_core or 0.0) + 0.3 * improve_rate)
    else:
        k = k_core
    k_status = "observed" if k is not None else "missing"

    # --- U resource efficiency ---
    ms = latencies_from_hardness(h_cases) + latencies_from_surgical(
        list((surgical or {}).get("cycle_rows") or [])
    )
    u = efficiency_from_latencies(ms)
    u_status = "observed" if u is not None else "missing"

    # --- D continuity ---
    d_surg = continuity_from_surgical(list((surgical or {}).get("cycle_rows") or []))
    d = mean_present(
        [
            cap_rate(by_cap, "followup_binding"),
            d_surg,
            d_rate,
        ]
    )
    d_status = "observed" if d is not None else "missing"

    return {
        "P": factor_metric(p, status=p_status, source=p_src, notes="gate predictive pass mass"),
        "M": factor_metric(
            m,
            status=m_status,
            source="cross_domain+transfer+softcascade",
            notes="multipartite / transfer manifold",
        ),
        "B": factor_metric(
            b,
            status=b_status,
            source="abstention+governance+forbidden",
            notes="refuse / authorize / no silent promote",
        ),
        "R": factor_metric(
            r,
            status=r_status,
            source="capability_balance+transfer",
            notes="engine coordination under distribution",
        ),
        "K": factor_metric(
            k,
            status=k_status,
            source="surgical_reask+still_fail",
            notes="fail→teach→re-ask recovery",
        ),
        "U": factor_metric(
            u,
            status=u_status,
            source="median_latency_ms",
            notes=f"n_lat={len(ms)}; med={statistics.median(ms):.0f}ms" if ms else "no latency",
        ),
        "D": factor_metric(
            d,
            status=d_status,
            source="followup+setup_threads+dialogue",
            notes="continuity under setup / rephrase",
        ),
    }


def effective_weights(
    factors: dict[str, dict[str, Any]],
    base: dict[str, float],
) -> tuple[dict[str, float], list[dict[str, str]]]:
    """Down-weight non-observed factors (proxy decorrelation / evidence honesty)."""
    warnings: list[dict[str, str]] = []
    w = dict(base)
    for k, m in factors.items():
        st = m.get("status")
        if st in {"missing", "not_run"}:
            w[k] = w.get(k, 0.0) * 0.35
            warnings.append(
                {
                    "kind": "low_evidence_weight",
                    "factor": k,
                    "detail": f"{k} status={st}; effective weight reduced",
                }
            )
        elif st in {"partial", "inferred"}:
            w[k] = w.get(k, 0.0) * 0.7
            warnings.append(
                {
                    "kind": "partial_evidence_weight",
                    "factor": k,
                    "detail": f"{k} status={st}; effective weight reduced",
                }
            )
    # renorm
    s = sum(w.values()) or 1.0
    w = {k: v / s for k, v in w.items()}
    return w, warnings


def compute_brpc(
    factors: dict[str, dict[str, Any]],
    weights: dict[str, float],
) -> dict[str, Any]:
    """Multiplicative coherence + additive mismatch in ΔΦ-space."""
    delta_parts: dict[str, float] = {}
    log_c = 0.0
    used: list[str] = []
    for k, w in weights.items():
        raw = factors[k].get("value")
        if raw is None:
            v = V_FLOOR  # conservative missing
            status_note = "floor_for_missing"
        else:
            v = max(V_FLOOR, min(1.0, float(raw)))
            status_note = "observed_value"
        # δφ_i = −w_i log v_i
        dphi_i = -w * math.log(v)
        delta_parts[k] = dphi_i
        log_c += w * math.log(v)
        used.append(f"{k}:{status_note}")

    c = math.exp(log_c)
    c = clamp01(c)
    # Guard tiny numerical issues
    if c <= 0:
        c = V_FLOOR**sum(weights.values())
        c = clamp01(c)
    delta_phi = -math.log(max(c, 1e-12))
    omega = 1.0 / (1.0 + delta_phi)

    observed = [
        k
        for k, m in factors.items()
        if m.get("status") == "observed" and m.get("value") is not None
    ]
    e_cov = sum(weights.get(k, 0.0) for k in observed)
    # If weights renormed over all factors, coverage = mass on observed
    e_cov = clamp01(e_cov)
    omega_eb = clamp01(omega * e_cov)

    # Boundary band on C (H7-style)
    lower = clamp01(H7_MU - H7_EPS)
    upper = clamp01(H7_MU + H7_EPS)
    if c < lower:
        h7 = "below_band"
    elif c > upper:
        h7 = "above_band"
    else:
        h7 = "within_band"

    weak = sorted(delta_parts.items(), key=lambda kv: kv[1], reverse=True)
    return {
        "C_BRPC": round(c, 6),
        "DeltaPhi_BRPC": round(delta_phi, 6),
        "Omega_BRPC": round(omega, 6),
        "Omega_evidence_backed": round(omega_eb, 6),
        "evidence_coverage": round(e_cov, 6),
        "delta_phi_parts": {k: round(v, 6) for k, v in delta_parts.items()},
        "weakest_factors": [k for k, _ in weak[:3]],
        "H7": {
            "mu": H7_MU,
            "epsilon": H7_EPS,
            "lower": lower,
            "upper": upper,
            "state": h7,
            "accepted": h7 in {"within_band", "above_band"} and e_cov >= 0.5,
        },
        "composition_notes": used,
    }


def recommendations(factors: dict, brpc: dict) -> list[dict[str, str]]:
    """Actionable correction injects from weak ΔΦ parts — not weight promote."""
    recs: list[dict[str, str]] = []
    parts = brpc.get("delta_phi_parts") or {}
    # Largest mismatch first
    ordered = sorted(parts.items(), key=lambda kv: kv[1], reverse=True)
    playbook = {
        "P": (
            "predictive",
            "Re-run hardness + transfer; add hardness seed for live fail; "
            "repair owning operator (not densify Bitwork).",
        ),
        "M": (
            "manifold",
            "SoftCascade geometry/planning pack-align; reduce geometry_blind; "
            "strengthen multipartite residual hops under open asks.",
        ),
        "B": (
            "boundary",
            "Harden refuse/authorize paths; audit forbidden hits; "
            "never auto-promote; extend honest_abstention / governed_learning cases.",
        ),
        "R": (
            "coordination",
            "Balance weak capability vs strong; fabric plan multi-engine; "
            "operator_program multi-hop end-to-end.",
        ),
        "K": (
            "recovery",
            "Surgical ask→analyze→teach→re-ask on persistent fails; "
            "code-path repair when teach does not move re-ask.",
        ),
        "U": (
            "resource",
            "Profile daemon warm path; avoid cold start; trim checklist fluency rewrite cost.",
        ),
        "D": (
            "continuity",
            "Expand followup_binding hardness; dialogue workspace binding; "
            "setup-thread surgical seeds (H104/H105/H110/H112 style).",
        ),
    }
    for k, dphi in ordered:
        v = (factors.get(k) or {}).get("value")
        if v is not None and v >= 0.92 and dphi < 0.05:
            continue
        title, action = playbook[k]
        recs.append(
            {
                "factor": k,
                "name": FACTOR_NAMES[k],
                "value": None if v is None else f"{v:.3f}",
                "delta_phi_share": f"{dphi:.4f}",
                "lane": title,
                "inject": action,
            }
        )
    # Always emit claim lock
    recs.append(
        {
            "factor": "*",
            "name": "claim_boundary",
            "value": "",
            "delta_phi_share": "",
            "lane": "governance",
            "inject": (
                "BRPC is candidate telemetry. Coherence is not consciousness. "
                "Never auto-promote .pwgt from C_BRPC."
            ),
        }
    )
    return recs


def teach_claims_from_receipt(receipt: dict, limit: int = 4) -> list[str]:
    """Generate pending teach claims aimed at weakest factors (review only)."""
    claims: list[str] = []
    factors = receipt.get("factors") or {}
    for rec in (receipt.get("recommendations") or [])[:limit]:
        k = rec.get("factor")
        if not k or k == "*":
            continue
        v = (factors.get(k) or {}).get("value")
        name = rec.get("name") or k
        claims.append(
            f"BRPC factor {k} ({name}) is weak"
            + (f" at {v:.3f}" if isinstance(v, (int, float)) else "")
            + f". Correction: {rec.get('inject')} "
            "Answer with continuous prose bound to operators/geometry; "
            "do not invent consciousness; never auto-promote weights."
        )
    return claims


def build_receipt(
    hardness_path: Path,
    surgical_path: Path,
    dialogue_path: Path,
    weights: dict[str, float] | None = None,
) -> dict[str, Any]:
    hardness = read_json(hardness_path) if hardness_path else None
    surgical = read_json(surgical_path) if surgical_path else None
    dialogue = read_json(dialogue_path) if dialogue_path else None
    if isinstance(hardness, list):
        hardness = None
    if isinstance(surgical, list):
        surgical = None
    if isinstance(dialogue, list):
        dialogue = None

    base_w = dict(weights or DEFAULT_WEIGHTS)
    factors = build_factors(hardness, surgical, dialogue)
    eff_w, weight_warnings = effective_weights(factors, base_w)
    brpc = compute_brpc(factors, eff_w)
    recs = recommendations(factors, brpc)

    sources = {
        "hardness": {
            "path": str(hardness_path.relative_to(ROOT)) if hardness_path.is_file() else None,
            "present": bool(hardness),
            "sha256": sha256_file(hardness_path) if hardness_path.is_file() else None,
            "status": (hardness or {}).get("status"),
            "passed": (hardness or {}).get("passed"),
            "case_count": (hardness or {}).get("case_count"),
        },
        "surgical": {
            "path": str(surgical_path.relative_to(ROOT)) if surgical_path.is_file() else None,
            "present": bool(surgical),
            "sha256": sha256_file(surgical_path) if surgical_path.is_file() else None,
            "second_pass_rate": (surgical or {}).get("second_pass_rate"),
            "still_fail_after_reask": (surgical or {}).get("still_fail_after_reask"),
            "promote_recommended": (surgical or {}).get("promote_recommended"),
        },
        "dialogue": {
            "path": str(dialogue_path.relative_to(ROOT)) if dialogue_path.is_file() else None,
            "present": bool(dialogue),
            "sha256": sha256_file(dialogue_path) if dialogue_path.is_file() else None,
            "status": (dialogue or {}).get("status"),
            "passed": (dialogue or {}).get("passed"),
            "case_count": (dialogue or {}).get("case_count"),
        },
    }

    warnings = list(weight_warnings)
    if brpc["H7"]["state"] == "below_band":
        warnings.append(
            {
                "kind": "below_band",
                "factor": "C_BRPC",
                "detail": "C_BRPC below H7 band — prefer repair regime, not promote",
            }
        )
    elif brpc["H7"]["state"] == "above_band":
        warnings.append(
            {
                "kind": "above_band",
                "factor": "C_BRPC",
                "detail": (
                    "C_BRPC above calibrated band — high gate scores are not max-coherence goals; "
                    "raise adversarial hardness / live stress rather than densify Bitwork"
                ),
            }
        )
    if brpc["evidence_coverage"] < 0.5:
        warnings.append(
            {
                "kind": "low_evidence_coverage",
                "factor": "E_cov",
                "detail": "C_BRPC incomplete without broader gate evidence",
            }
        )

    receipt = {
        "schema": "perci.brpc-receipt.v0.1",
        "theory": "BRPC v0.1 — Boundary-Regulated Predictive Coherence",
        "claim_status": "candidate",
        "domain": "software-agent-adaptation",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "claim_boundary": [
            "not a consciousness equation",
            "not a universal field",
            "not an established cross-domain law",
            "coherence is not truth",
            "never auto-promote .pwgt from this receipt",
        ],
        "sources": sources,
        "weights_requested": base_w,
        "weights_effective": {k: round(v, 6) for k, v in eff_w.items()},
        "factors": {
            k: {
                **factors[k],
                "name": FACTOR_NAMES[k],
            }
            for k in DEFAULT_WEIGHTS
        },
        "brpc": brpc,
        "warnings": warnings,
        "recommendations": recs,
        "teach_candidates_pending_review": teach_claims_from_receipt(
            {"factors": factors, "recommendations": recs}
        ),
        "automatic_promotion": False,
        "promote_recommended": False,
    }
    raw = json.dumps(receipt, sort_keys=True, ensure_ascii=False).encode("utf-8")
    receipt["receipt_sha256"] = hashlib.sha256(raw).hexdigest()
    return receipt


def print_summary(receipt: dict) -> None:
    b = receipt["brpc"]
    print("BRPC v0.1 × Perci Runtime Summary")
    print(f"Domain: {receipt['domain']}")
    print(f"Claim status: {receipt['claim_status']}")
    print(f"C_BRPC: {b['C_BRPC']:.3f}")
    print(f"DeltaPhi_BRPC: {b['DeltaPhi_BRPC']:.3f}")
    print(f"Omega_BRPC: {b['Omega_BRPC']:.3f}")
    print(f"Evidence coverage: {b['evidence_coverage']:.3f}")
    print(f"Omega evidence-backed: {b['Omega_evidence_backed']:.3f}")
    print(f"H7 state: {b['H7']['state']}")
    print("Factors:")
    for k in DEFAULT_WEIGHTS:
        f = receipt["factors"][k]
        val = f.get("value")
        vs = f"{val:.3f}" if isinstance(val, (int, float)) else "n/a"
        print(f"  {k} ({f['name']}): {vs}  [{f.get('status')}]  δφ={b['delta_phi_parts'].get(k, 0):.4f}")
    print(f"Weakest: {', '.join(b.get('weakest_factors') or [])}")
    warns = receipt.get("warnings") or []
    if warns:
        print("Warnings:")
        for w in warns:
            print(f"  - {w.get('kind')}: {w.get('detail')}")
    else:
        print("Warnings: none")
    print("Top injects:")
    for rec in (receipt.get("recommendations") or [])[:4]:
        if rec.get("factor") == "*":
            continue
        print(f"  [{rec['factor']}] {rec['inject'][:100]}")
    print("automatic_promotion: false")


def run_hardness(binary: Path) -> int:
    cmd = [sys.executable, str(ROOT / "scripts" / "evaluate_hardness.py"), "--perci-bin", str(binary)]
    print("=== run hardness ===")
    return subprocess.call(cmd, cwd=str(ROOT))


def run_surgical(binary: Path, cycles: int = 24) -> int:
    cmd = [
        sys.executable,
        str(ROOT / "scripts" / "interact_evolve_loop.py"),
        "--mode",
        "surgical",
        "--cycles",
        str(cycles),
        "--binary",
        str(binary),
    ]
    print(f"=== run surgical loop cycles={cycles} ===")
    return subprocess.call(cmd, cwd=str(ROOT))


def inject_teach_corrections(binary: Path, claims: list[str], limit: int = 3) -> list[dict]:
    """Stage teach claims as pending review only (no weight promote)."""
    results: list[dict] = []
    if not binary.is_file():
        return [{"ok": False, "error": f"missing binary {binary}"}]
    for claim in claims[:limit]:
        try:
            proc = subprocess.run(
                [str(binary), "teach", claim],
                cwd=str(ROOT),
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=120,
            )
            results.append(
                {
                    "ok": proc.returncode == 0,
                    "claim": claim[:200],
                    "preview": (proc.stdout or proc.stderr or "")[:180],
                }
            )
        except Exception as exc:  # noqa: BLE001 — receipt path must not crash loop
            results.append({"ok": False, "claim": claim[:200], "error": str(exc)})
    return results


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hardness", type=Path, default=DEFAULT_HARDNESS)
    parser.add_argument("--surgical", type=Path, default=DEFAULT_SURGICAL)
    parser.add_argument("--dialogue", type=Path, default=DEFAULT_DIALOGUE)
    parser.add_argument("--json-out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument(
        "--run-gates",
        action="store_true",
        help="Refresh hardness then surgical before scoring",
    )
    parser.add_argument("--surgical-cycles", type=int, default=24)
    parser.add_argument(
        "--inject-teach",
        action="store_true",
        help="Stage teach claims for weakest factors (pending review only)",
    )
    parser.add_argument(
        "--feedback-loop",
        action="store_true",
        help="Score → inject teach on weak factors → re-run surgical → re-score",
    )
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    if args.run_gates or args.feedback_loop:
        if not args.binary.is_file():
            print(f"missing binary: {args.binary}", file=sys.stderr)
            return 2
        rc = run_hardness(args.binary)
        if rc != 0:
            print(f"hardness exit {rc}", file=sys.stderr)
        rc = run_surgical(args.binary, args.surgical_cycles)
        if rc != 0:
            print(f"surgical exit {rc}", file=sys.stderr)

    receipt = build_receipt(args.hardness, args.surgical, args.dialogue)
    if not args.quiet:
        print_summary(receipt)

    inject_log: list[dict] = []
    if args.inject_teach or args.feedback_loop:
        claims = receipt.get("teach_candidates_pending_review") or []
        # Inject only when a non-latency factor is soft, or C is below band.
        # U (resource) is fixed by profiling/code path, not teach claims.
        soft: list[str] = []
        for k, f in (receipt.get("factors") or {}).items():
            v = f.get("value")
            if v is None:
                soft.append(k)
            elif k != "U" and float(v) < 0.90:
                soft.append(k)
            elif k == "U" and float(v) < 0.70:
                soft.append(k)
        need = bool(soft) or receipt["brpc"]["H7"]["state"] == "below_band"
        if need and claims:
            # Prefer claims for soft factors
            prefer = [c for c in claims if any(f"factor {s}" in c for s in soft)] or claims
            print(f"=== inject teach corrections (pending review) soft={soft} ===")
            inject_log = inject_teach_corrections(args.binary, prefer)
            for row in inject_log:
                mark = "ok" if row.get("ok") else "fail"
                print(f"  teach[{mark}]: {row.get('claim', '')[:80]}")
        else:
            print(
                "=== inject skipped (factors strong; above/within band — "
                "raise adversarial stress, do not densify) ==="
            )

    if args.feedback_loop:
        print("=== feedback re-run surgical ===")
        run_surgical(args.binary, args.surgical_cycles)
        receipt = build_receipt(args.hardness, args.surgical, args.dialogue)
        receipt["feedback_loop"] = {
            "inject_log": inject_log,
            "second_pass": True,
        }
        if not args.quiet:
            print("--- after feedback ---")
            print_summary(receipt)

    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    args.json_out.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"receipt: {args.json_out}")

    # Exit code: 0 within/above band with coverage; 1 below band; 2 incomplete evidence
    if receipt["brpc"]["evidence_coverage"] < 0.35:
        return 2
    if receipt["brpc"]["H7"]["state"] == "below_band":
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
