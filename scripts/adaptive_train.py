#!/usr/bin/env python3
"""Adaptive Perci training — interact, score, fold, morph weights.

Pipeline:
  1. Run a held-out + adaptive task suite through `perci classify` / `perci ask`
  2. Score routing + answer quality heuristics
  3. Fold wins into training/adaptive/traces.jsonl + curriculum
  4. Optionally rebuild 200 MiB .pwgt with adaptive injection (--morph)

Usage:
  python scripts/adaptive_train.py
  python scripts/adaptive_train.py --morph
  python scripts/adaptive_train.py --morph --perci-bin target/release/perci.exe
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ADAPTIVE_DIR = ROOT / "training" / "adaptive"
TRACES = ADAPTIVE_DIR / "traces.jsonl"
CURRICULUM_INJECT = ADAPTIVE_DIR / "inject_prompts.json"
FROM_LUMEN = ROOT / "training" / "from-lumen"
PACKS = ROOT / "knowledge" / "packs"

# (prompt, expected_label_substr, kind)
TASK_SUITE: list[tuple[str, str, str]] = [
    ("who are you", "identity", "identity"),
    ("hello perci", "greeting", "greeting"),
    ("calculate 20 percent of 80", "math", "math"),
    ("calculate 12 divided by 5", "math", "math"),
    ("triangle area base 8 height 5", "geometry", "geometry"),
    ("circle circumference radius 4", "geometry", "geometry"),
    ("verify permission before durable change", "governance", "governance"),
    ("how should cortex connect to perci and lumen", "systems", "systems"),
    ("explain binary neural networks simply", "explanation", "explanation"),
    ("compare deterministic tools with neural prediction", "comparison", "comparison"),
    ("remember that adaptive training morphs local weights", "memory", "memory"),
    ("what assumptions are required to prove a claim", "logic", "logic"),
    ("fix cargo compile error in rust forge stream", "code", "code"),
    ("implement unit test for keystore", "code", "code"),
    ("write a plan to evolve local intelligence packs", "planning", "planning"),
    ("scientific method for measuring model improvement", "science", "science"),
    ("rewrite this sentence so it is clearer", "english", "english"),
    ("are you conscious or just software", "identity", "identity"),
    ("how do I reason about uncertainty and evidence", "science", "science"),
    ("debug ownership error involving borrow checker", "code", "code"),
    ("what is self awareness for a governed local tool", "identity", "identity"),
    ("security checklist before agent mutation", "governance", "governance"),
    ("make a practical plan to build local AI mesh", "planning", "planning"),
    ("explain lumen cortex selective memory", "systems", "systems"),
]


def run_cmd(bin_path: Path, args: list[str], env: dict) -> tuple[int, str, str]:
    proc = subprocess.run(
        [str(bin_path), *args],
        cwd=str(ROOT),
        env=env,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return proc.returncode, proc.stdout.strip(), proc.stderr.strip()


def score_route(expected: str, label: str, kind: str) -> bool:
    lab = label.lower()
    exp = expected.lower()
    if exp in lab or lab in exp:
        return True
    # Soft equivalents
    soft = {
        "code": {"general", "systems", "planning"},
        "science": {"logic", "explanation", "general"},
        "systems": {"governance", "planning", "explanation"},
        "identity": {"greeting", "general"},
        "math": {"geometry"},
        "geometry": {"math"},
    }
    if kind in soft and lab in soft[kind]:
        return True
    return False


def score_answer(kind: str, answer: str) -> float:
    a = answer.lower().strip()
    if not a or len(a) < 8:
        return 0.0
    score = 0.4
    # Ceremonial headings are not a fluency metric. Prefer bounded directness,
    # epistemic honesty, and an actionable verification signal.
    if 20 <= len(a) <= 600:
        score += 0.15
    if any(marker in a for marker in ("verify", "check", "test", "measure", "because")):
        score += 0.10
    if any(marker in a for marker in ("i don't know", "uncertain", "haven't run")):
        score += 0.05
    # Natural voice / woven guidance (not bolted "Relevant Cortex guidance")
    if "practical angle" in a or "two practical angles" in a or "that's " in a or a.startswith("hey") or a.startswith("hi"):
        score += 0.2
    if "relevant cortex guidance" in a or "pack:" in a or "exact result" in a or "that's " in a:
        score += 0.15
    if "from packs:" in a or "conclusion:" in a:
        score += 0.1
    if kind == "math" and re.search(r"\b\d+(\.\d+)?\b", a):
        score += 0.3
    if kind == "geometry" and ("area" in a or "got it" in a or re.search(r"\d", a)):
        score += 0.25
    if kind == "identity" and ("perci" in a or "bitwork" in a or "tool" in a or "local" in a):
        score += 0.25
    if kind in {"greeting", "memory"} and any(
        w in a for w in ("here", "ready", "remember", "glad", "welcome", "hey", "hi")
    ):
        score += 0.25
    if kind == "code" and any(w in a for w in ("error", "reproduc", "test", "patch", "cargo", "invariant", "stuck", "failing")):
        score += 0.25
    if kind == "governance" and any(w in a for w in ("permission", "author", "ledger", "sandbox", "verify", "rollback")):
        score += 0.25
    if "i don't know" in a and kind not in {"general"}:
        score -= 0.2
    return max(0.0, min(1.0, score))


def score_reasoning(prompt: str, answer: str, kind: str) -> float:
    """Mirror perci::reason::score_reasoning for Python eval harness."""
    a = answer.lower()
    u = prompt.lower()
    s = 0.25
    if "goal:" in a or "here's how i'd reason" in a or "how i'd reason" in a:
        s += 0.05
    if "steps:" in a or re.search(r"\n\s*1\.", answer):
        s += 0.05
    if "stress-test" in a or "counterexample" in a or "what if" in a:
        s += 0.10
    if "next check" in a or "verify" in a or "re-run" in a:
        s += 0.10
    if "known:" in a or "assuming:" in a or "unknown:" in a:
        s += 0.10
    if "haven't run" in a or "i don't know" in a or "uncertain" in a:
        s += 0.05
    if "i ran cargo" in a or "tests passed" in a:
        s -= 0.20
    keys = [w for w in re.split(r"[^a-z0-9]+", u) if len(w) >= 4][:5]
    if keys:
        hits = sum(1 for k in keys if k in a)
        s += 0.10 * (hits / len(keys))
    if kind == "code" and any(x in a for x in ("reproduc", "error", "patch", "verify")):
        s += 0.10
    if kind in {"logic", "science"} and any(x in a for x in ("evidence", "assum", "falsif", "counter")):
        s += 0.10
    if 20 <= len(answer.strip()) <= 700:
        s += 0.08
    return max(0.0, min(1.0, s))


def load_lumen_prompts() -> dict[str, list[str]]:
    """Fold from-lumen + curriculum into labeled adaptive prompts."""
    by_label: dict[str, list[str]] = {lab: [] for lab in [
        "greeting","identity","english","logic","math","geometry","memory","code",
        "governance","planning","explanation","systems","science","creativity",
        "comparison","general",
    ]}
    keyword_map = [
        ("code", ["cargo", "rust", "compile", "forge", "debug", "patch", "function", "test"]),
        ("math", ["percent", "calcul", "fraction", "multiply", "divide", "plus"]),
        ("geometry", ["triangle", "circle", "radius", "pythag"]),
        ("governance", ["permission", "authority", "audit", "snapshot", "constitution"]),
        ("systems", ["cortex", "lumen", "perci", "mesh", "nexus", "hybrid"]),
        ("science", ["science", "evidence", "hypothesis", "uncertainty", "measure"]),
        ("identity", ["who are", "conscious", "perci is", "self-aware", "not agi"]),
        ("planning", ["plan", "milestone", "evolve cycle", "next step"]),
        ("memory", ["remember", "cortex", "lesson", "teach"]),
        ("logic", ["assum", "proof", "contradict", "reason"]),
        ("explanation", ["explain", "plain", "teach me"]),
        ("comparison", ["compare", "tradeoff", "contrast"]),
        ("english", ["sentence", "grammar", "rewrite"]),
        ("greeting", ["hello", "hi perci"]),
    ]

    def classify_text(text: str) -> str:
        t = text.lower()
        best, best_n = "general", 0
        for lab, keys in keyword_map:
            n = sum(1 for k in keys if k in t)
            if n > best_n:
                best, best_n = lab, n
        return best

    sources = list(FROM_LUMEN.glob("*.jsonl"))
    sources += list((ROOT / "training" / "curriculum").glob("curriculum-*.jsonl"))
    for path in sources:
        if not path.is_file():
            continue
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            texts: list[str] = []
            if "text" in row:
                texts.append(str(row["text"]))
            if "messages" in row and isinstance(row["messages"], list):
                for m in row["messages"]:
                    if isinstance(m, dict) and m.get("content"):
                        texts.append(str(m["content"]))
            for text in texts:
                text = re.sub(r"\s+", " ", text).strip()
                if len(text) < 12 or len(text) > 280:
                    continue
                if re.search(r"(?i)(api[_-]?key|secret|password|sk-)", text):
                    continue
                lab = classify_text(text)
                if text not in by_label[lab]:
                    by_label[lab].append(text)

    # Pack card titles as high-value identity/systems/governance prompts
    if PACKS.is_dir():
        for md in PACKS.rglob("*.md"):
            try:
                first = next(
                    (ln[2:].strip() for ln in md.read_text(encoding="utf-8", errors="replace").splitlines()
                     if ln.startswith("# ")),
                    "",
                )
            except OSError:
                continue
            if not first:
                continue
            prompt = f"apply the operator card: {first}"
            lab = classify_text(first + " " + md.as_posix())
            if prompt not in by_label[lab]:
                by_label[lab].append(prompt[:240])

    return by_label


def interact(bin_path: Path, env: dict) -> list[dict]:
    ADAPTIVE_DIR.mkdir(parents=True, exist_ok=True)
    traces: list[dict] = []
    wins = 0
    for prompt, expected, kind in TASK_SUITE:
        code_c, out_c, err_c = run_cmd(bin_path, ["classify", prompt], env)
        label, score, overlap = "?", 0, 0
        if code_c == 0 and out_c:
            try:
                row = json.loads(out_c.splitlines()[-1])
                label = str(row.get("label", "?"))
                score = int(row.get("score", 0))
                overlap = int(row.get("overlap", 0))
            except (json.JSONDecodeError, ValueError):
                pass
        route_ok = score_route(expected, label, kind)

        code_a, out_a, err_a = run_cmd(bin_path, ["ask", prompt], env)
        answer = out_a if code_a == 0 else f"[ask fail] {err_a or out_a}"
        ans_score = score_answer(kind, answer)
        r_score = score_reasoning(prompt, answer, kind)
        # Win: route ok + decent answer; deep kinds also need reason floor
        deep = kind in {"code", "logic", "science", "planning", "governance", "systems", "comparison"}
        win = route_ok and ans_score >= 0.45 and (not deep or r_score >= 0.40)
        if win:
            wins += 1
        trace = {
            "at": datetime.now(timezone.utc).isoformat(),
            "prompt": prompt,
            "expected": expected,
            "kind": kind,
            "label": label,
            "classify_score": score,
            "overlap": overlap,
            "route_ok": route_ok,
            "answer_score": round(ans_score, 3),
            "reason_score": round(r_score, 3),
            "win": win,
            "answer": answer[:800],
        }
        traces.append(trace)
        flag = "WIN " if win else "miss"
        print(
            f"  [{flag}] {kind:12} route={label:12} a={ans_score:.2f} r={r_score:.2f}  {prompt[:42]}"
        )

    with TRACES.open("a", encoding="utf-8") as f:
        for t in traces:
            f.write(json.dumps(t, ensure_ascii=False) + "\n")

    # Build inject prompts from wins + lumen folds
    by_label = load_lumen_prompts()
    preferred_path = ADAPTIVE_DIR / "preferred_pairs.jsonl"
    for t in traces:
        if not t["win"]:
            continue
        # Use prompt as adaptive surface form for the routed label when sensible
        lab = t["label"] if t["label"] in by_label else t["kind"]
        if lab not in by_label:
            lab = "general"
        p = t["prompt"]
        if p not in by_label[lab]:
            by_label[lab].append(p)
        # Also store answer snippets as explanation/systems surface when strong
        if t["answer_score"] >= 0.7 and lab in ("identity", "governance", "systems", "science", "code"):
            snippet = re.sub(r"\s+", " ", t["answer"]).strip()[:200]
            if snippet and snippet not in by_label[lab]:
                by_label[lab].append(snippet)
        # Preferred reason pairs: high reason_score deep answers become morph inject
        r = float(t.get("reason_score") or 0)
        if r >= 0.75 and t["kind"] in {
            "code", "logic", "science", "planning", "governance", "systems", "comparison"
        }:
            # Inject compressed goal line + first step as routing surface
            ans = t.get("answer") or ""
            goal_line = ""
            for line in ans.splitlines():
                if "goal:" in line.lower():
                    goal_line = line.split(":", 1)[-1].strip()
                    break
            if goal_line and goal_line not in by_label.get(lab, []):
                by_label.setdefault(lab, []).append(goal_line[:240])
            # Persist full preferred pair for future fine surfaces
            pair = {
                "prompt": t["prompt"],
                "label": lab,
                "kind": t["kind"],
                "reason_score": r,
                "answer": ans[:1200],
                "source": "adaptive_win",
            }
            with preferred_path.open("a", encoding="utf-8") as pf:
                pf.write(json.dumps(pair, ensure_ascii=False) + "\n")

    # Fold preferred_pairs into inject (prompt + short conclusion lines)
    if preferred_path.is_file():
        for line in preferred_path.read_text(encoding="utf-8", errors="replace").splitlines()[-80:]:
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            lab = str(row.get("label") or "general")
            pr = str(row.get("prompt") or "").strip()
            if pr and lab in by_label and pr not in by_label[lab]:
                by_label[lab].append(pr[:240])
            ans = str(row.get("answer") or "")
            for line2 in ans.splitlines():
                if "conclusion:" in line2.lower():
                    c = line2.split(":", 1)[-1].strip()
                    if c and lab in by_label and c not in by_label[lab]:
                        by_label[lab].append(c[:240])
                    break

    # Drop empties for size
    inject = {k: v[:80] for k, v in by_label.items() if v}
    CURRICULUM_INJECT.write_text(json.dumps(inject, indent=2), encoding="utf-8")

    reason_scores = [t["reason_score"] for t in traces]
    deep_scores = [
        t["reason_score"]
        for t in traces
        if t["kind"] in {"code", "logic", "science", "planning", "governance", "systems", "comparison"}
    ]
    summary = {
        "at": datetime.now(timezone.utc).isoformat(),
        "tasks": len(traces),
        "wins": wins,
        "win_rate": round(wins / max(len(traces), 1), 3),
        "reason_score_avg": round(sum(reason_scores) / max(len(reason_scores), 1), 3),
        "reason_score_deep_avg": round(sum(deep_scores) / max(len(deep_scores), 1), 3)
        if deep_scores
        else 0.0,
        "inject_labels": {k: len(v) for k, v in inject.items()},
        "traces": str(TRACES),
        "inject": str(CURRICULUM_INJECT),
    }
    (ADAPTIVE_DIR / "last_run.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    # Also publish for Lumen evolve scorecard
    lumen_pub = ROOT.parent / ".lumen" / "evolve" / "perci_reason.json"
    try:
        lumen_pub.parent.mkdir(parents=True, exist_ok=True)
        lumen_pub.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    except OSError:
        pass
    print(json.dumps(summary, indent=2))
    return traces


def morph(output: Path) -> int:
    """Rebuild weights with adaptive injection via build_weights."""
    env = os.environ.copy()
    env["PERCI_ADAPTIVE_INJECT"] = str(CURRICULUM_INJECT)
    env["PERCI_ADAPTIVE"] = "1"
    # Seed morph from inject hash so weights truly change when curriculum changes
    raw = CURRICULUM_INJECT.read_bytes() if CURRICULUM_INJECT.is_file() else b""
    digest = hashlib.sha256(raw).hexdigest()[:16]
    env["PERCI_ADAPTIVE_SEED"] = digest
    print(f"morphing weights → {output}")
    print(f"adaptive seed fragment: {digest}")
    cmd = [
        sys.executable,
        str(ROOT / "scripts" / "build_weights.py"),
        "--output",
        str(output),
    ]
    proc = subprocess.run(cmd, cwd=str(ROOT), env=env)
    if proc.returncode != 0:
        return proc.returncode
    v = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "verify_weights.py"), "--model", str(output)],
        cwd=str(ROOT),
    )
    return v.returncode


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--morph", action="store_true", help="rebuild .pwgt with adaptive inject")
    ap.add_argument(
        "--perci-bin",
        type=Path,
        default=ROOT / "target" / "release" / "perci.exe",
    )
    ap.add_argument(
        "--output",
        type=Path,
        default=ROOT / "models" / "perci-cognitive-v0.1.pwgt",
    )
    ap.add_argument("--skip-interact", action="store_true")
    args = ap.parse_args()

    bin_path = args.perci_bin
    if not bin_path.is_file():
        # fallback debug
        alt = ROOT / "target" / "debug" / "perci.exe"
        if alt.is_file():
            bin_path = alt
        else:
            print(f"perci binary missing: {args.perci_bin}", file=sys.stderr)
            return 1

    env = os.environ.copy()
    env["PERCI_WEIGHTS"] = str(ROOT / "models" / "perci-cognitive-v0.1.pwgt")
    env["PERCI_PACKS"] = str(ROOT / "knowledge" / "packs")
    env["PERCI_MEMORY"] = str(ROOT / "memory" / "adaptive.mem")
    env["PYTHONUTF8"] = "1"

    if not args.skip_interact:
        print("== adaptive interact ==")
        t0 = time.time()
        interact(bin_path, env)
        print(f"interact seconds: {time.time() - t0:.1f}")

    if args.morph:
        print("== adaptive morph ==")
        t0 = time.time()
        rc = morph(args.output)
        print(f"morph seconds: {time.time() - t0:.1f}")
        return rc
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
