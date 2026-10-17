#!/usr/bin/env python3
"""Seeded held-out checks for perci.exact and perci.check_math.

Items are generated from a seed with an INDEPENDENT oracle (Python's
fractions.Fraction and trial division), never from Perci's code. Gates:
  * exact:      0 wrong `pass` values (abstain is allowed and counted)
  * check_math: 0 false `fail` on correct answers; catch rate on planted errors
  * replay:     every verdict's output_sha256 is identical across two server processes

Usage: python3 evals/tools-heldout-v1/run_heldout.py [--seed N] [--n 200] [--perci PATH] [--json out]
Exit code 1 when a gate fails. Standard library only.
"""
import argparse
import json
import os
import random
import subprocess
import sys
from fractions import Fraction

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class Mcp:
    def __init__(self, perci, env):
        self.p = subprocess.Popen([perci, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.i = 0
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "heldout", "version": "1"}})

    def rpc(self, method, params=None):
        self.i += 1
        self.p.stdin.write((json.dumps({"jsonrpc": "2.0", "id": self.i, "method": method, "params": params or {}}) + "\n").encode())
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def call(self, name, args):
        return self.rpc("tools/call", {"name": name, "arguments": args})["result"]["structuredContent"]

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def fmt(q):
    """Exact decimal when terminating (<= 6 places), else the fraction."""
    d = q.denominator
    while d % 2 == 0:
        d //= 2
    while d % 5 == 0:
        d //= 5
    if d == 1:
        s = f"{float(q):.6f}".rstrip("0").rstrip(".")
        if Fraction(s) == q:
            return s
    return f"{q.numerator}/{q.denominator}" if q.denominator != 1 else str(q.numerator)


def is_prime(n):
    if n < 2:
        return False
    i = 2
    while i * i <= n:
        if n % i == 0:
            return False
        i += 1
    return True


def gen_exact(rng, n):
    items = []
    words = {"+": ["+", "plus"], "-": ["-", "minus"], "*": ["*", "times", "x"], "/": ["/", "divided by"]}
    for k in range(n):
        kind = rng.choice(["frac", "pct", "pow", "mixed", "linear", "sqrt", "prime", "word"])
        if kind == "frac":
            a, b, c, d = rng.randint(1, 30), rng.randint(2, 30), rng.randint(1, 30), rng.randint(2, 30)
            op = rng.choice("+-*/")
            ans = {"+": Fraction(a, b) + Fraction(c, d), "-": Fraction(a, b) - Fraction(c, d),
                   "*": Fraction(a, b) * Fraction(c, d), "/": Fraction(a, b) / Fraction(c, d)}[op]
            # Division of fractions needs parentheses; the bare form is ambiguous
            # and Perci must abstain on it (checked separately below).
            if op == "/":
                items.append((f"({a}/{b}) / ({c}/{d})", ans))
                if rng.random() < 0.3:
                    items.append((f"{a}/{b} / {c}/{d}", "ABSTAIN"))
            else:
                items.append((f"{a}/{b} {op} {c}/{d}", ans))
        elif kind == "pct":
            p, x = rng.randint(1, 99), rng.randint(1, 5000)
            items.append((rng.choice([f"{p}% of {x}", f"{p} percent of {x}", f"what is {p}% of {x}"]), Fraction(p, 100) * x))
        elif kind == "pow":
            b, e = rng.randint(2, 12), rng.randint(2, 20)
            items.append((rng.choice([f"{b}^{e}", f"{b} to the power of {e}", f"{b}**{e}"]), Fraction(b) ** e))
        elif kind == "mixed":
            a, b, c, d = (rng.randint(1, 99) for _ in range(4))
            items.append((f"({a} + {b}) * {c} - {d}", Fraction((a + b) * c - d)))
        elif kind == "linear":
            a = rng.choice([i for i in range(-9, 10) if i])
            x = Fraction(rng.randint(-50, 50), rng.choice([1, 1, 2, 3]))
            b = rng.randint(-40, 40)
            c = a * x + b
            if c.denominator != 1:
                continue
            items.append((f"solve {a}x + {b} = {c.numerator}".replace("+ -", "- "), x))
        elif kind == "sqrt":
            r = rng.randint(1, 2000)
            items.append((rng.choice([f"sqrt of {r * r}", f"square root of {r * r}"]), Fraction(r)))
        elif kind == "prime":
            m = rng.randint(2, 2_000_000)
            items.append((f"is {m} prime", is_prime(m)))
        else:
            a, b = rng.randint(1, 999), rng.randint(1, 999)
            op = rng.choice("+-*/")
            w = rng.choice(words[op])
            ans = {"+": Fraction(a + b), "-": Fraction(a - b), "*": Fraction(a * b), "/": Fraction(a, b)}[op]
            items.append((rng.choice([f"{a} {w} {b}", f"what is {a} {w} {b}", f"calculate {a} {w} {b}"]), ans))
    return items


def gen_answers(rng, n):
    """(text, has_error) pairs in LLM style."""
    out = []
    for _ in range(n):
        kind = rng.choice(["sum", "pct", "prod", "frac", "chain"])
        if kind == "sum":
            a, b = rng.randint(10, 9999), rng.randint(10, 9999)
            expr, val = f"{a} + {b}", Fraction(a + b)
            tmpl = "Adding them up: {e} = {v}. That's the total."
        elif kind == "pct":
            p, x = rng.randint(2, 40), rng.choice([rng.randint(20, 2000), rng.randint(1, 99) * 10])
            expr, val = f"{p}% of {x}", Fraction(p, 100) * x
            tmpl = "Sure! {e} is {v}, so that's the discount."
        elif kind == "prod":
            a, b = rng.randint(2, 300), rng.randint(2, 300)
            expr, val = f"{a} * {b}", Fraction(a * b)
            tmpl = "Each box holds {a} items, so {e} = {v} items in total."
        elif kind == "frac":
            a, b, c, d = rng.randint(1, 9), rng.randint(2, 12), rng.randint(1, 9), rng.randint(2, 12)
            expr, val = f"{a}/{b} + {c}/{d}", Fraction(a, b) + Fraction(c, d)
            tmpl = "Combining the fractions: {e} = {v}."
        else:
            a, b, c = rng.randint(100, 900), rng.randint(5, 95), rng.randint(2, 9)
            expr, val = f"({a} - {b}) * {c}", Fraction((a - b) * c)
            tmpl = "Net per week is {e} = {v}."
        correct = fmt(val)
        has_error = rng.random() < 0.5
        claimed = correct
        if has_error:
            mode = rng.choice(["off_by_one", "times_ten", "digit_swap", "drop_term"])
            if mode == "off_by_one" or val.denominator != 1:
                wrong = val + rng.choice([1, -1])
            elif mode == "times_ten":
                wrong = val * 10
            elif mode == "digit_swap":
                s = str(val.numerator)
                if len(s) >= 2 and s[-1] != s[-2]:
                    wrong = Fraction(int(s[:-2] + s[-1] + s[-2]))
                else:
                    wrong = val + 2
            else:
                wrong = val - rng.randint(2, 9)
            claimed = fmt(wrong)
        a_val = expr.split()[0].strip("(")
        out.append((tmpl.format(e=expr, v=claimed, a=a_val), has_error))
    return out


def run(perci, env, seed, n):
    rng = random.Random(seed)
    exact_items = gen_exact(rng, n)
    answers = gen_answers(rng, n)
    m = Mcp(perci, env)
    ex = {"items": len(exact_items), "correct": 0, "wrong_pass": 0, "abstain": 0, "fail": 0, "wrong_examples": [], "abstain_examples": []}
    hashes = []
    for expr, oracle in exact_items:
        v = m.call("exact", {"expr": expr})
        hashes.append(v["receipt"]["output_sha256"])
        if oracle == "ABSTAIN":
            ex["expected_abstain"] = ex.get("expected_abstain", 0) + 1
            if v["status"] == "abstain":
                ex["abstained_as_expected"] = ex.get("abstained_as_expected", 0) + 1
            else:
                ex["wrong_pass"] += 1
                ex["wrong_examples"].append({"expr": expr, "perci": v["status"], "oracle": "abstain (ambiguous)"})
            continue
        if v["status"] != "pass":
            ex["abstain" if v["status"] == "abstain" else "fail"] += 1
            if len(ex["abstain_examples"]) < 5:
                ex["abstain_examples"].append(expr)
            continue
        r = v["result"]
        if isinstance(oracle, bool):
            ok = r.get("prime") == oracle
        else:
            got = r.get("value")
            ok = got is not None and Fraction(got) == oracle
        if ok:
            ex["correct"] += 1
        else:
            ex["wrong_pass"] += 1
            ex["wrong_examples"].append({"expr": expr, "perci": r.get("value", r.get("prime")), "oracle": str(oracle)})
    cm = {"answers": len(answers), "planted": 0, "caught": 0, "missed": 0, "correct_answers": 0,
          "false_fail": 0, "abstain_on_correct": 0, "missed_examples": [], "false_fail_examples": []}
    for text, has_error in answers:
        v = m.call("check_math", {"text": text})
        hashes.append(v["receipt"]["output_sha256"])
        if has_error:
            cm["planted"] += 1
            if v["status"] == "fail":
                cm["caught"] += 1
            else:
                cm["missed"] += 1
                if len(cm["missed_examples"]) < 5:
                    cm["missed_examples"].append(text)
        else:
            cm["correct_answers"] += 1
            if v["status"] == "fail":
                cm["false_fail"] += 1
                cm["false_fail_examples"].append(text)
            elif v["status"] == "abstain":
                cm["abstain_on_correct"] += 1
    m.close()
    return ex, cm, hashes


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=20261017)
    ap.add_argument("--n", type=int, default=200)
    ap.add_argument("--perci", default=os.path.join(ROOT, "target", "release", "perci"))
    ap.add_argument("--json")
    a = ap.parse_args()
    perci = a.perci if os.path.exists(a.perci) else a.perci + ".exe"
    env = dict(os.environ, PERCI_HOME=ROOT, PERCI_TOOLS="all")
    ex, cm, h1 = run(perci, env, a.seed, a.n)
    _, _, h2 = run(perci, env, a.seed, a.n)
    replay_ok = h1 == h2
    answerable = ex["items"] - ex.get("expected_abstain", 0)
    ex["coverage_pct"] = round(100 * ex["correct"] / max(1, answerable), 1)
    cm["catch_pct"] = round(100 * cm["caught"] / max(1, cm["planted"]), 1)
    report = {"seed": a.seed, "exact": ex, "check_math": cm, "replay_identical": replay_ok}
    print(json.dumps(report, indent=2))
    if a.json:
        with open(a.json, "w") as f:
            json.dump(report, f, indent=2)
    gates = ex["wrong_pass"] == 0 and cm["false_fail"] == 0 and replay_ok
    sys.exit(0 if gates else 1)


if __name__ == "__main__":
    main()
