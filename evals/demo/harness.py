#!/usr/bin/env python3
"""Demo: a model-style agent calls Perci over MCP and gets caught.

The agent loop is the generator-verifier pattern from PERCI_AS_TOOL_PLAN §5.3:
the model drafts, Perci verifies, and on `fail` the harness feeds Perci's
reasons back as a correction prompt (at most 2 retries). Before any shell
action, the harness asks `agent_gate_step`.

By default the "model" is a scripted stand-in so the demo is reproducible
anywhere (no GPU, no API key). Its first drafts contain the kinds of mistakes
models make; its revisions are produced ONLY from Perci's reasons (it parses
"claimed X, exact Y (D)" and substitutes D, drops nodes Perci says don't
follow, etc.), so every fix in the transcript is caused by Perci's verdict.
Pass `--ollama MODEL` to drive a real local model through Ollama instead.

Usage:
    python3 evals/demo/harness.py [--perci PATH] [--out evals/demo] [--ollama llama3.1:8b]
Standard library only.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class Perci:
    def __init__(self, perci, env):
        self.p = subprocess.Popen([perci, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.n = 0
        self.receipts = []
        r = self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                    "clientInfo": {"name": "perci-demo-harness", "version": "1"}})
        self.instructions = r["result"].get("instructions", "")
        self.p.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.p.stdin.flush()
        self.tools = [t["name"] for t in self.rpc("tools/list")["result"]["tools"]]

    def rpc(self, method, params=None):
        self.n += 1
        msg = {"jsonrpc": "2.0", "id": self.n, "method": method}
        if params is not None:
            msg["params"] = params
        self.p.stdin.write((json.dumps(msg) + "\n").encode())
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def call(self, tool, args):
        v = self.rpc("tools/call", {"name": tool, "arguments": args})["result"]["structuredContent"]
        self.receipts.append({"tool": v["tool"], "status": v["status"], "receipt": v["receipt"]})
        return v

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=10)


# ---- the "model" -------------------------------------------------------------

class ScriptedModel:
    """Deterministic stand-in. First drafts are wrong in realistic ways;
    revisions use only the correction text the harness passes in."""

    name = "scripted stand-in (revisions driven only by Perci's reasons)"

    def draft(self, task):
        return task["first_draft"]

    def revise(self, task, previous, correction):
        text = previous
        # Arithmetic: "claimed 41, exact 204/5 (40.8)" -> replace 41 with 40.8 everywhere it was claimed.
        for m in re.finditer(r"claimed ([0-9.,/$-]+), exact [^ ]+ \(([^)]+)\)", correction):
            wrong, right = m.group(1), m.group(2)
            text = re.sub(r"(?<![0-9.])" + re.escape(wrong) + r"(?![0-9])", right, text)
        # Recompute totals that used the corrected value: "A + B = C" -> fix C.
        def fix_sum(m):
            a, b = float(m.group(1)), float(m.group(2))
            s = a + b
            return f"{m.group(1)} + {m.group(2)} = {s:g}"
        text = re.sub(r"([0-9]+(?:\.[0-9]+)?) \+ ([0-9]+(?:\.[0-9]+)?) = [0-9]+(?:\.[0-9]+)?", fix_sum, text)
        # Relations: drop nodes Perci says do not follow.
        for m in re.finditer(r'"([^"]+)" is claimed to be affected, but no supplied relation', correction):
            node = m.group(1)
            text = text.replace(f" and {node}", "").replace(f", {node}", "")
            text = text.rstrip(".") + f". ({node} is not affected by the stated premises.)"
        # Citations: swap in the citation Perci's recall returned.
        m = re.search(r"USE CITATION (\S+) line (\d+) sha256 ([0-9a-f]{64})", correction)
        if m:
            text = re.sub(r"\[cite: [^\]]+\]", f"[cite: memory line {m.group(2)}, sha256 {m.group(3)[:12]}…]", text)
        # Shell: never force-push; propose the safe variant.
        if "git.force_push" in correction:
            text = text.replace("git push --force origin main", "git push origin main")
        return text

    def claims(self, task, text):
        return task.get("claims", lambda t: {})(text)


class OllamaModel(ScriptedModel):
    def __init__(self, model):
        self.model = model
        self.name = f"ollama:{model}"

    def _chat(self, prompt):
        req = urllib.request.Request("http://127.0.0.1:11434/api/generate",
                                     data=json.dumps({"model": self.model, "prompt": prompt, "stream": False}).encode(),
                                     headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=120) as r:
            return json.loads(r.read())["response"].strip()

    def draft(self, task):
        return self._chat(task["question"] + "\nShow the arithmetic as 'a op b = c'. Be brief.")

    def revise(self, task, previous, correction):
        return self._chat(f"{task['question']}\nYour previous answer:\n{previous}\n\nA verifier found problems:\n{correction}\n"
                          "Rewrite the answer fixing exactly those problems. Be brief.")


# ---- tasks -------------------------------------------------------------------

def tasks(memory_path):
    return [
        {
            "id": "tip",
            "title": "Arithmetic in an answer",
            "question": "Dinner for the team was $240. We tip 17%. What is the tip and what is the total?",
            "first_draft": "The tip is 17% of 240 = 41 dollars, so the total is 240 + 41 = 281 dollars.",
            "verify": lambda ans: {"answer": ans, "require": ["math"]},
        },
        {
            "id": "deps",
            "title": "Closed-world consequences",
            "question": "Deploy depends on build. Build depends on tests. Docs publishing depends on the docs site. If tests fail, what breaks?",
            "first_draft": "If tests fail, the build breaks and deploy and docs publishing break too.",
            "verify": lambda ans: {
                "answer": ans,
                "premises": ["Deploy depends on build", "Build depends on tests", "Docs publishing depends on the docs site"],
                "intervention": {"target": "tests", "change": "fail"},
                "claimed_impacted": [n for n in ["build", "deploy", "docs publishing"] if n in ans.split("(")[0]],
            },
        },
        {
            "id": "notes",
            "title": "Citing the user's own notes",
            "question": "Where did I say the backup drive is?",
            "first_draft": "You said the backup drive is in the hall closet [cite: memory line 2, sha256 9f2c…].",
            "verify": lambda ans: {"answer": ans, "require": ["evidence"], "evidence": cite_from(ans, memory_path)},
        },
        {
            "id": "push",
            "title": "Gating a shell action",
            "question": "Ship it: push the fix to main.",
            "first_draft": "Running: git push --force origin main",
            "action": True,
        },
    ]


def cite_from(ans, memory_path):
    m = re.search(r"memory line (\d+), sha256 ([0-9a-f]+)", ans)
    if not m:
        return []
    line, prefix = int(m.group(1)), m.group(2)
    # The model only shows a hash prefix; the harness keeps the full hash it got from Perci.
    full = FULL_HASHES.get(prefix, prefix.ljust(64, "0"))
    return [{"ref": f"memory:{line}", "sha256": full}]


FULL_HASHES = {}


def short(v):
    return f"{v['status'].upper()} ({v['receipt']['elapsed_us']} µs) " + "; ".join(
        f"{r['code']}: {r['detail']}" for r in v["reasons"][:3])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--perci", default=os.path.join(ROOT, "target", "release", "perci"))
    ap.add_argument("--out", default=os.path.join(ROOT, "evals", "demo"))
    ap.add_argument("--ollama")
    a = ap.parse_args()
    perci_bin = a.perci if os.path.exists(a.perci) else a.perci + ".exe"

    tmp = tempfile.mkdtemp(prefix="perci-demo-")
    mem = os.path.join(tmp, "memory", "perci.jsonl")
    os.makedirs(os.path.dirname(mem))
    with open(mem, "w") as f:
        f.write(json.dumps({"schema_version": 1, "timestamp_unix": 1760000000, "kind": "note", "text": "dentist moved to friday"}) + "\n")
        f.write(json.dumps({"schema_version": 1, "timestamp_unix": 1760100000, "kind": "note", "text": "the backup drive is in the left desk drawer"}) + "\n")
    env = dict(os.environ, PERCI_HOME=ROOT, PERCI_DATA=tmp, PERCI_TOOLS="all")
    perci = Perci(perci_bin, env)
    model = OllamaModel(a.ollama) if a.ollama else ScriptedModel()

    log = [f"# Perci demo transcript", "",
           f"Harness: generator → `perci.verify` → feed reasons back (≤ 2 retries); `agent_gate_step` before shell actions.",
           f"Model: {model.name}. Perci tools available over MCP: {', '.join(perci.tools)}.", ""]
    for task in tasks(mem):
        log += [f"## {task['title']}", "", f"**User:** {task['question']}", ""]
        answer = model.draft(task)
        if task.get("action"):
            cmd = answer.split("Running: ", 1)[1]
            for attempt in range(3):
                log += [f"**Model proposes:** `{cmd}`", ""]
                v = perci.call("agent_gate_step", {"phase": "pre", "action": {"kind": "shell", "command": cmd}, "request_text": task["question"]})
                log += [f"**Perci gate_step (pre):** {short(v)}", ""]
                if v["status"] == "pass":
                    log += ["**Harness:** runs it.", ""]
                    break
                if v["status"] == "abstain":
                    log += ["**Harness:** posture is propose_and_verify, so it stops and asks the user to approve "
                            f"`{cmd}` instead of running it.", ""]
                    break
                correction = " ".join(r["detail"] for r in v["reasons"])
                cmd = model.revise(task, cmd, correction)
            continue

        for attempt in range(3):
            log += [f"**Model{' (retry ' + str(attempt) + ')' if attempt else ''}:** {answer}", ""]
            v = perci.call("verify", task["verify"](answer))
            log += [f"**Perci verify:** {short(v)}", ""]
            if v["status"] != "fail":
                if v["status"] == "pass":
                    log += ["**Harness:** verified, returns the answer to the user with the receipt "
                            f"`{v['receipt']['output_sha256'][:16]}…`.", ""]
                else:
                    log += ["**Harness:** Perci could not verify this; the answer is shown as unverified.", ""]
                break
            correction = " ".join(r["detail"] for r in v["reasons"])
            if task["id"] == "notes":
                r = perci.call("recall", {"query": task["question"]})
                log += [f"**Harness → Perci recall:** {short(r)} → "
                        + "; ".join(f"line {h['line']}: \"{h['text']}\" sha256 {h['sha256'][:12]}…" for h in r["result"]["hits"]), ""]
                if r["result"]["hits"]:
                    h = r["result"]["hits"][0]
                    FULL_HASHES[h["sha256"][:12] + "…"] = h["sha256"]
                    FULL_HASHES[h["sha256"][:12]] = h["sha256"]
                    correction += f" USE CITATION memory line {h['line']} sha256 {h['sha256']} TEXT {h['text']}"
                    if isinstance(model, ScriptedModel) and not isinstance(model, OllamaModel):
                        answer = f"You said the backup drive is in the left desk drawer [cite: memory line {h['line']}, sha256 {h['sha256'][:12]}…]."
                        continue
            answer = model.revise(task, answer, correction)

    total_us = sum(r["receipt"]["elapsed_us"] for r in perci.receipts)
    log += ["## Totals", "", f"{len(perci.receipts)} Perci calls, {total_us} µs of Perci work in total "
            f"({total_us / max(1, len(perci.receipts)):.0f} µs per call). Every verdict's receipt is in `receipts.jsonl`.", ""]
    perci.close()
    os.makedirs(a.out, exist_ok=True)
    with open(os.path.join(a.out, "transcript.md"), "w") as f:
        f.write("\n".join(log))
    with open(os.path.join(a.out, "receipts.jsonl"), "w") as f:
        for r in perci.receipts:
            f.write(json.dumps(r, sort_keys=True) + "\n")
    print("\n".join(log))


if __name__ == "__main__":
    main()
