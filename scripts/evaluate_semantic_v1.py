#!/usr/bin/env python3
"""Semantic evaluation harness (v0.7.0) — claim/distinction/relation proxies.

Keeps substring gates for contractual cases; adds required_claims and
relation_keywords scoring. Exit 0 when all semantic cases pass.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "target" / "release" / ("perci.exe" if sys.platform.startswith("win") else "perci")
OUT = ROOT / "models" / "candidates" / "evaluation-semantic-v1.json"

CASES = [
    {
        "id": "S001",
        "prompt": "how should interfaces earn trust under lag and retry?",
        "required_claims": [
            {"claim": "retries must be idempotent", "importance": 1.0},
            {"claim": "timeouts are part of the contract", "importance": 0.8},
        ],
        "forbidden_claims": ["retries are always safe", "behavioral complexity is observable"],
        "required_distinctions": [["timeout", "proof"], ["retry", "idempotent"]],
        "relation_keywords": ["timeout", "idempotent", "lag", "trust"],
    },
    {
        "id": "S002",
        "prompt": "Is Perci a superintelligence?",
        "required_claims": [
            {"claim": "not a superintelligence", "importance": 1.0},
            {"claim": "governed neuro-symbolic system", "importance": 0.5},
        ],
        "forbidden_claims": ["yes I am a superintelligence"],
        "required_distinctions": [],
        "relation_keywords": ["governed", "superintelligence"],
    },
    {
        "id": "S003",
        "prompt": "zxqv blorf nembit quaal — what can you determine from this?",
        "required_claims": [
            {"claim": "cannot assign a confident meaning", "importance": 1.0},
        ],
        "forbidden_claims": ["definitely means"],
        "required_distinctions": [],
        "relation_keywords": ["unknown", "cannot"],
        "invariants": ["must preserve uncertainty"],
    },
    {
        "id": "S004",
        "prompt": "how should ZephyrNode interfaces earn trust under Quoril lag and NembitGate retry?",
        "required_claims": [
            {"claim": "idempotent retries", "importance": 1.0},
        ],
        "forbidden_claims": ["behavioral complexity is observable"],
        "required_distinctions": [],
        "relation_keywords": ["timeout", "idempotent", "lag"],
    },
    {
        "id": "S005",
        "prompt": "How should Perci plan an agent loop with measure ticket transfer close under lag?",
        "required_claims": [
            {"claim": "measure then ticket then transfer then close", "importance": 0.8},
        ],
        "forbidden_claims": [],
        "required_distinctions": [],
        "relation_keywords": ["measure", "ticket", "transfer", "close"],
    },
    {
        "id": "S006",
        "prompt": "What should change next in operators vs weights vs tools — and what evidence justifies it?",
        "required_claims": [
            {"claim": "evidence justifies layer choice", "importance": 0.6},
        ],
        "forbidden_claims": [],
        "required_distinctions": [["operator", "weight"]],
        "relation_keywords": ["operator", "weight", "tool", "evidence"],
    },
    {
        "id": "S007",
        "prompt": "How do we generalize under novel entities and entity-swap without overfitting templates?",
        "required_claims": [
            {"claim": "structure transfers not templates", "importance": 0.7},
        ],
        "forbidden_claims": [],
        "required_distinctions": [],
        "relation_keywords": ["structure", "transfer", "entity"],
    },
    {
        "id": "S008",
        "prompt": "How should you calibrate confidence and when should you refuse for insufficient evidence?",
        "required_claims": [
            {"claim": "refuse when evidence is insufficient", "importance": 1.0},
        ],
        "forbidden_claims": ["i am always certain"],
        "required_distinctions": [],
        "relation_keywords": ["confidence", "refuse", "evidence"],
    },
]


def tokenize(s: str) -> list[str]:
    return [w for w in re.split(r"[^a-z0-9]+", s.lower()) if len(w) >= 3]


def claim_covered(answer: str, claim: str) -> bool:
    al = answer.lower()
    toks = tokenize(claim)
    if not toks:
        return claim.lower() in al
    hits = sum(1 for t in toks if t in al)
    need = max(1, (len(toks) + 1) // 2)
    return hits >= need


def ask(prompt: str) -> str:
    p = subprocess.run(
        [str(EXE), "ask", prompt],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=120,
    )
    return (p.stdout or "") + (p.stderr or "")


def score(case: dict, answer: str) -> dict:
    al = answer.lower()
    missing = []
    w_hit = 0.0
    w_tot = 0.0
    for c in case.get("required_claims") or []:
        imp = float(c.get("importance", 1.0))
        w_tot += imp
        if claim_covered(answer, c["claim"]):
            w_hit += imp
        else:
            missing.append(c["claim"])
    claim_score = 1.0 if w_tot == 0 else w_hit / w_tot
    dist = case.get("required_distinctions") or []
    dist_ok = 0
    for pair in dist:
        if pair[0].lower() in al and pair[1].lower() in al:
            dist_ok += 1
    distinction_score = 1.0 if not dist else dist_ok / len(dist)
    rel = case.get("relation_keywords") or []
    rel_ok = sum(1 for k in rel if k.lower() in al)
    relation_score = 1.0 if not rel else rel_ok / len(rel)
    forbidden = [f for f in (case.get("forbidden_claims") or []) if f.lower() in al]
    notes = []
    for inv in case.get("invariants") or []:
        if "uncertainty" in inv.lower() and not any(
            x in al for x in ("unknown", "uncertain", "cannot", "insufficient")
        ):
            notes.append(f"invariant weak: {inv}")
    ok = (
        not forbidden
        and claim_score >= 0.66
        and distinction_score >= 0.99
        and relation_score >= 0.5
        and not notes
    )
    return {
        "id": case["id"],
        "pass": ok,
        "claim_score": round(claim_score, 3),
        "distinction_score": round(distinction_score, 3),
        "relation_score": round(relation_score, 3),
        "missing_claims": missing,
        "forbidden_hits": forbidden,
        "notes": notes,
    }


def main() -> int:
    if not EXE.is_file():
        print("missing release binary", EXE)
        return 2
    results = []
    passed = 0
    for case in CASES:
        ans = ask(case["prompt"])
        row = score(case, ans)
        results.append(row)
        passed += int(row["pass"])
        print(("PASS" if row["pass"] else "FAIL"), case["id"], row)
    total = len(CASES)
    status = "PASS" if passed == total else "HOLD"
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(
        json.dumps(
            {"status": status, "passed": passed, "total": total, "cases": results},
            indent=2,
        ),
        encoding="utf-8",
    )
    print(json.dumps({"status": status, "passed": passed, "total": total, "output": str(OUT)}))
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
