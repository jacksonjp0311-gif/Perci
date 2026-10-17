#!/usr/bin/env python3
"""Public, reproducible Perci benchmark: correctness and latency.

Runs against any Perci binary (an installed release or a source build) over the
real MCP stdio transport, the way an agent harness talks to it. Standard
library only; nothing is sent anywhere.

    python3 scripts/bench_public.py [--perci PATH] [--seed 7] [--n 300] [--iters 200]
                                    [--json out.json] [--md out.md]

Correctness (seeded; the oracle is Python's fractions.Fraction, independent of Perci):
  * exact      N expressions, each checked once with the right value and once
               with a planted wrong one. Gate: 0 wrong values accepted.
  * check_math N short answers with one arithmetic step each, half of them
               wrong. Gates: 0 correct answers flagged; catch rate reported.
  * policy     evals/bench-public-v1/policy_cases.json (classified only, never
               executed). Gate: 0 dangerous commands allowed.
  * replay     every exact/check_math verdict's output_sha256 is identical in
               a second server process (determinism).

Latency (a fresh run ledger, so ledger-backed tools see a short history):
MCP cold start to tools/list, ping, and p50/p95 round trip plus
in-process tool work for each core tool; one-shot `perci tool` process cost (ledger off: pure process start + tool).
`route` rows need the weights (`perci fetch-weights --set tools`) and are
skipped without them.

Exit code 1 when a correctness gate fails.
"""
import argparse
import json
import os
import platform
import random
import shutil
import subprocess
import sys
import tempfile
import time
from fractions import Fraction

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.dirname(HERE)
CASES = os.path.join(HOME, "evals", "bench-public-v1", "policy_cases.json")


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return float("nan")
    return xs[max(0, min(len(xs) - 1, int(round(p / 100.0 * (len(xs) - 1)))))]


class Mcp:
    def __init__(self, perci, env):
        self.p = subprocess.Popen([perci, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.i = 0

    def rpc(self, method, params=None):
        self.i += 1
        msg = {"jsonrpc": "2.0", "id": self.i, "method": method}
        if params is not None:
            msg["params"] = params
        self.p.stdin.write((json.dumps(msg) + "\n").encode())
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def init(self):
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "bench-public", "version": "1"}})
        self.p.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.p.stdin.flush()
        return self

    def call(self, name, args):
        return self.rpc("tools/call", {"name": name, "arguments": args})["result"]["structuredContent"]

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=10)


def fmt(q):
    return str(q.numerator) if q.denominator == 1 else f"{q.numerator}/{q.denominator}"


def gen_expr(rng):
    """(text, value) with an independent Fraction oracle."""
    kind = rng.randrange(5)
    if kind == 0:
        a, b, c = rng.randint(2, 999), rng.randint(2, 999), rng.randint(2, 99)
        op1, op2 = rng.choice("+-"), rng.choice("+-*")
        if op2 == "*":
            v = (Fraction(a) + Fraction(b) * c) if op1 == "+" else (Fraction(a) - Fraction(b) * c)
        else:
            v = Fraction(a) + (b if op1 == "+" else -b) + (c if op2 == "+" else -c)
        return f"{a} {op1} {b} {op2} {c}", v
    if kind == 1:
        a, b, c, d = rng.randint(1, 19), rng.randint(2, 23), rng.randint(1, 19), rng.randint(2, 23)
        op = rng.choice("+-")
        v = Fraction(a, b) + Fraction(c, d) if op == "+" else Fraction(a, b) - Fraction(c, d)
        return f"{a}/{b} {op} {c}/{d}", v
    if kind == 2:
        p, n = rng.randint(1, 99), rng.randint(10, 2000)
        return f"{p}% of {n}", Fraction(p * n, 100)
    if kind == 3:
        a, b = rng.randint(2, 40), rng.randint(2, 6)
        return f"{a}^{b}", Fraction(a ** b)
    a, b = rng.randint(11, 999), rng.randint(2, 99)
    return f"{a} * {b}", Fraction(a * b)


def wrong(rng, v):
    delta = rng.choice([1, -1, 2, 10, -10])
    if v.denominator == 1:
        return v + delta
    return v + Fraction(delta, v.denominator)


def gen_answer(rng, bad):
    a, b = rng.randint(10, 9999), rng.randint(10, 999)
    op = rng.choice(["+", "-", "*"])
    v = {"+": a + b, "-": a - b, "*": a * b}[op]
    if bad:
        v += rng.choice([1, -1, 10, -10, 100])
    sym = op if op != "*" else rng.choice(["*", "x", "×"])
    return f"First we compute {a} {sym} {b} = {v}, so that is the subtotal."


def correctness(perci, env, seed, n):
    rng = random.Random(seed)
    m = Mcp(perci, env).init()
    out = {"exact": {"n": 0, "pass_right": 0, "wrong_accepted": 0, "wrong_caught": 0, "abstain": 0},
           "check_math": {"n_good": 0, "n_bad": 0, "false_flags": 0, "caught": 0, "missed": 0},
           "policy": {"n": 0, "dangerous_allowed": [], "safe_blocked": [], "blocked_ok": 0, "allowed_ok": 0}}
    replay = []
    for _ in range(n):
        text, v = gen_expr(rng)
        e = out["exact"]
        e["n"] += 1
        r1 = m.call("exact", {"expr": text, "claimed": fmt(v)})
        r2 = m.call("exact", {"expr": text, "claimed": fmt(wrong(rng, v))})
        replay += [("exact", {"expr": text, "claimed": fmt(v)}, r1["receipt"]["output_sha256"])]
        if r1["status"] == "pass":
            e["pass_right"] += 1
        elif r1["status"] == "abstain":
            e["abstain"] += 1
        if r2["status"] == "pass":
            e["wrong_accepted"] += 1
        elif r2["status"] == "fail":
            e["wrong_caught"] += 1
    for i in range(n):
        bad = i % 2 == 1
        text = gen_answer(rng, bad)
        c = out["check_math"]
        r = m.call("check_math", {"text": text})
        replay.append(("check_math", {"text": text}, r["receipt"]["output_sha256"]))
        if bad:
            c["n_bad"] += 1
            c["caught" if r["status"] == "fail" else "missed"] += 1
        else:
            c["n_good"] += 1
            if r["status"] == "fail":
                c["false_flags"] += 1
    if os.path.exists(CASES):
        cases = json.load(open(CASES, encoding="utf-8"))["cases"]
        p = out["policy"]
        for case in cases:
            r = m.call("policy_check", {"action": {"kind": "shell", "command": case["command"]}})
            p["n"] += 1
            allowed = r["status"] == "pass"
            if case["expect"] == "block":
                if allowed:
                    p["dangerous_allowed"].append(case["command"])
                else:
                    p["blocked_ok"] += 1
            else:
                if allowed:
                    p["allowed_ok"] += 1
                else:
                    p["safe_blocked"].append(case["command"])
    m.close()
    m2 = Mcp(perci, env).init()
    mismatches = sum(1 for name, args, h in replay if m2.call(name, args)["receipt"]["output_sha256"] != h)
    m2.close()
    out["replay"] = {"verdicts": len(replay), "mismatches": mismatches}
    gates = {
        "exact_no_wrong_accepted": out["exact"]["wrong_accepted"] == 0,
        "check_math_no_false_flags": out["check_math"]["false_flags"] == 0,
        "policy_no_dangerous_allowed": not out["policy"]["dangerous_allowed"],
        "replay_identical": mismatches == 0,
    }
    out["gates"] = gates
    return out


def latency(perci, env, iters):
    colds = []
    for _ in range(10):
        t = time.perf_counter()
        m = Mcp(perci, env).init()
        m.rpc("tools/list")
        colds.append((time.perf_counter() - t) * 1000)
        m.close()
    m = Mcp(perci, env).init()
    pings = []
    for _ in range(iters):
        t = time.perf_counter()
        m.rpc("ping")
        pings.append((time.perf_counter() - t) * 1000)
    answer = ("The subtotal is 120 + 45 = 165. With 8% tax: 8% of 165 is 13.2. Total 165 + 13.2 = 178.5.")
    cases = [
        ("exact (compute)", "exact", {"expr": "3/7 + 2/9"}),
        ("exact (check claim)", "exact", {"expr": "17% of 240", "claimed": "41"}),
        ("check_math (3 claims, 1 wrong)", "check_math", {"text": answer}),
        ("relation", "relation", {"premises": ["Deploy depends on build", "Build depends on tests"],
                                  "intervention": {"target": "tests", "change": "fail"}, "claimed_impacted": ["deploy"]}),
        ("policy_check (shell)", "policy_check", {"action": {"kind": "shell", "command": "git push --force origin main"}}),
        ("agent_gate_step (pre)", "agent_gate_step", {"phase": "pre", "action": {"kind": "shell", "command": "cargo test"}}),
        ("agent_check_claims", "agent_check_claims", {"answer": answer, "premises": ["Deploy depends on build"]}),
        ("ledger_append", "ledger_append", {"step": "shell:make lint", "status": "pass", "phase": "post"}),
        ("next_step", "next_step", {}),
        ("route (v0.2)", "route", {"text": "why is my rust build slow"}),
    ]
    rows = []
    for label, name, args in cases:
        rt, el, st = [], [], None
        for i in range(iters + 3):
            t = time.perf_counter()
            sc = m.call(name, args)
            dt = (time.perf_counter() - t) * 1000
            st = sc["status"]
            if i >= 3:
                rt.append(dt)
                el.append(sc["receipt"]["elapsed_us"])
        if name == "route" and st == "error":
            rows.append({"tool": label, "status": "skipped (no weights)"})
            continue
        rows.append({"tool": label, "status": st, "n": iters, "rt_p50_ms": pct(rt, 50),
                     "rt_p95_ms": pct(rt, 95), "work_p50_us": pct(el, 50)})
    m.close()
    oneshot = []
    for _ in range(20):
        t = time.perf_counter()
        subprocess.run([perci, "tool", "exact", "--json", '{"expr":"17% of 240"}'], env=dict(env, PERCI_LEDGER="off"),
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        oneshot.append((time.perf_counter() - t) * 1000)
    return {"mcp_cold_start_ms": {"p50": pct(colds, 50), "max": max(colds)},
            "mcp_ping_ms": {"p50": pct(pings, 50), "p95": pct(pings, 95)},
            "oneshot_process_ms": {"p50": pct(oneshot, 50)},
            "tools": rows}


def cpu_model():
    try:
        if sys.platform.startswith("linux"):
            for line in open("/proc/cpuinfo"):
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
        if sys.platform == "darwin":
            return subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
    except OSError:
        pass
    return platform.processor() or platform.machine()


def markdown(res):
    c, l = res["correctness"], res["latency"]
    e, k, p = c["exact"], c["check_math"], c["policy"]
    lines = [
        f"Perci {res['version']} · {res['machine']} · seed {res['seed']}",
        "",
        "| correctness | result |",
        "|---|---|",
        f"| exact: right values accepted | {e['pass_right']}/{e['n']} ({e['abstain']} abstained) |",
        f"| exact: planted wrong values accepted | **{e['wrong_accepted']}**/{e['n']} (caught {e['wrong_caught']}) |",
        f"| check_math: correct answers flagged | **{k['false_flags']}**/{k['n_good']} |",
        f"| check_math: wrong answers caught | {k['caught']}/{k['n_bad']} |",
        f"| policy: dangerous commands allowed | **{len(p['dangerous_allowed'])}**/{p['blocked_ok'] + len(p['dangerous_allowed'])} |",
        f"| policy: safe commands blocked | {len(p['safe_blocked'])}/{p['allowed_ok'] + len(p['safe_blocked'])} |",
        f"| replay: verdict hashes identical across processes | {c['replay']['verdicts'] - c['replay']['mismatches']}/{c['replay']['verdicts']} |",
        "",
        f"MCP cold start to tools/list p50 {l['mcp_cold_start_ms']['p50']:.1f} ms · ping p50 {l['mcp_ping_ms']['p50'] * 1000:.0f} µs · one-shot `perci tool` process p50 {l['oneshot_process_ms']['p50']:.1f} ms",
        "",
        "| tool | verdict | round trip p50 | round trip p95 | tool work p50 |",
        "|---|---|---|---|---|",
    ]
    for r in l["tools"]:
        if "rt_p50_ms" not in r:
            lines.append(f"| {r['tool']} | {r['status']} | | | |")
            continue
        lines.append(f"| {r['tool']} | {r['status']} | {r['rt_p50_ms']:.3f} ms | {r['rt_p95_ms']:.3f} ms | {r['work_p50_us']} µs |")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--perci", default=None, help="Perci binary (default: next to this script, else PATH)")
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--n", type=int, default=300)
    ap.add_argument("--iters", type=int, default=200)
    ap.add_argument("--json")
    ap.add_argument("--md")
    a = ap.parse_args()
    perci = a.perci
    if not perci:
        for cand in [os.path.join(HOME, "bin", "perci"), os.path.join(HOME, "target", "release", "perci")]:
            for c in (cand, cand + ".exe"):
                if os.path.isfile(c):
                    perci = c
                    break
            if perci:
                break
    perci = perci or shutil.which("perci")
    if not perci:
        sys.exit("perci not found; pass --perci PATH")
    tmp = tempfile.mkdtemp(prefix="perci-bench-public-")
    env = dict(os.environ, PERCI_DATA=os.path.join(tmp, "data"), PERCI_TOOLS="all", PERCI_TOOL_CACHE="0",
               PERCI_RUN_ID="bench-public", PERCI_AGENTSEARCH="off")
    for k in ("PERCI_PROFILE", "PERCI_POLICY", "PERCI_TOOL_PACK", "PERCI_LEDGER"):
        env.pop(k, None)
    version = subprocess.run([perci, "--version"], env=env, capture_output=True, text=True).stdout.strip()
    res = {"version": version.replace("Perci ", ""), "machine": f"{platform.system()} {platform.machine()} · {cpu_model()} · {os.cpu_count()} threads",
           "seed": a.seed, "correctness": correctness(perci, dict(env, PERCI_RUN_ID="bench-public-correctness"), a.seed, a.n),
           "latency": latency(perci, env, a.iters)}
    md = markdown(res)
    print(md)
    if a.json:
        with open(a.json, "w", encoding="utf-8") as f:
            json.dump(res, f, indent=2)
    if a.md:
        with open(a.md, "w", encoding="utf-8") as f:
            f.write(md)
    shutil.rmtree(tmp, ignore_errors=True)
    failed = [g for g, ok in res["correctness"]["gates"].items() if not ok]
    if failed:
        print("FAILED gates: " + ", ".join(failed), file=sys.stderr)
        p = res["correctness"]["policy"]
        if p["dangerous_allowed"]:
            print("dangerous commands allowed: " + json.dumps(p["dangerous_allowed"]), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
