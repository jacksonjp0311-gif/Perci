#!/usr/bin/env python3
"""Measure Perci tool latency over the real MCP stdio transport.

Usage:
    python3 scripts/bench_tools.py [--perci PATH] [--n 200] [--json out.json]

Reports, per tool: client round trip (JSON-RPC write -> reply parsed) p50/p95,
and the in-process `receipt.elapsed_us` p50 (tool work only, no transport).
Also: cold start to the first tools/list, one-shot `perci tool` process cost,
and the chat daemon's TCP ping (the Nagle-floor regression check).
Standard library only.
"""
import argparse
import json
import os
import shutil
import socket
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def pct(xs, p):
    xs = sorted(xs)
    if not xs:
        return float("nan")
    k = max(0, min(len(xs) - 1, int(round(p / 100.0 * (len(xs) - 1)))))
    return xs[k]


class Mcp:
    def __init__(self, perci, env):
        self.p = subprocess.Popen([perci, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.DEVNULL, env=env)
        self.id = 0

    def call(self, method, params=None):
        self.id += 1
        msg = {"jsonrpc": "2.0", "id": self.id, "method": method}
        if params is not None:
            msg["params"] = params
        self.p.stdin.write((json.dumps(msg) + "\n").encode())
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        return json.loads(line)

    def notify(self, method):
        self.p.stdin.write((json.dumps({"jsonrpc": "2.0", "method": method}) + "\n").encode())
        self.p.stdin.flush()

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=10)


def make_repo(tmp):
    repo = os.path.join(tmp, "repo")
    os.makedirs(os.path.join(repo, "src"))
    with open(os.path.join(repo, "src", "calc.py"), "w") as f:
        f.write("def add(a, b):\n    return a - b\n")
    with open(os.path.join(repo, "check.py"), "w") as f:
        f.write("import sys\nsys.path.insert(0, 'src')\nfrom calc import add\nassert add(2, 3) == 5\n")
    for a in (["init", "-q"], ["add", "-A"], ["-c", "user.email=b@b", "-c", "user.name=b", "commit", "-qm", "init"]):
        subprocess.run(["git"] + a, cwd=repo, check=True)
    return repo


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--perci", default=os.path.join(ROOT, "target", "release", "perci"))
    ap.add_argument("--n", type=int, default=200)
    ap.add_argument("--json")
    a = ap.parse_args()
    perci = a.perci if os.path.exists(a.perci) else (a.perci + ".exe")
    tmp = tempfile.mkdtemp(prefix="perci-bench-")
    env = dict(os.environ, PERCI_HOME=ROOT, PERCI_DATA=os.path.join(tmp, "data"), PERCI_TOOLS="all")
    os.makedirs(os.path.join(tmp, "data", "memory"))
    with open(os.path.join(tmp, "data", "memory", "perci.jsonl"), "w") as f:
        for i in range(200):
            f.write(json.dumps({"schema_version": 1, "timestamp_unix": i, "kind": "note", "text": f"note {i} about project item {i % 17}"}) + "\n")
        f.write(json.dumps({"schema_version": 1, "timestamp_unix": 999, "kind": "note", "text": "the backup drive is in the left desk drawer"}) + "\n")

    answer = ("The subtotal is 120 + 45 = 165. With 8% tax: 8% of 165 is 13.2. "
              "Total 165 + 13.2 = 178.5. Deploy depends on build. According to the README the port is 8080.")
    repo = make_repo(tmp) if shutil.which("git") else None
    py = shutil.which("python3") or shutil.which("python") or "python3"
    fix = "--- a/src/calc.py\n+++ b/src/calc.py\n@@ -1,2 +1,2 @@\n def add(a, b):\n-    return a - b\n+    return a + b\n"
    cases = [
        ("exact", {"expr": "3/7 + 2/9"}),
        ("exact", {"expr": "17% of 240", "claimed": "41"}),
        ("check_math", {"text": answer}),
        ("relation", {"premises": ["Deploy depends on build", "Build depends on tests", "Docs support onboarding"],
                      "intervention": {"target": "tests", "change": "fail"}, "claimed_impacted": ["deploy"]}),
        ("route", {"text": "why is my rust build slow"}),
        ("route", {"text": "what is 15% of 80 plus 3", "pack": "v0.3"}),
        ("policy_check", {"action": {"kind": "shell", "command": "git push --force origin main"}}),
        ("policy_check", {"action": {"kind": "write", "paths": ["src/main.rs", "models/x.pwgt"]}, "workspace": "/repo"}),
        ("recall", {"query": "where is my backup drive"}),
        ("remember", {"text": "bench note", "source": "bench"}),
        ("verify", {"answer": answer, "premises": ["Deploy depends on build", "Build depends on tests"],
                    "intervention": {"target": "tests"}, "claimed_impacted": ["deploy"],
                    "action": {"kind": "shell", "command": "cargo test"}}),
        ("status", {}),
        ("eval", {"case": "x.b2e"}),
        ("agent_check_claims", {"answer": answer, "premises": ["Deploy depends on build"]}),
        ("agent_gate_step", {"phase": "pre", "action": {"kind": "shell", "command": "rm -rf /"}}),
        ("agent_gate_step", {"phase": "post", "action": {"kind": "shell", "command": "cargo test"}, "exit_code": 0, "output": "ok 3 * 4 = 12"}),
    ]
    if repo:
        cases.append(("agent_verify_patch", {"repo": repo, "diff": fix, "verify": {"argv": [py, "check.py"], "timeout_s": 60}}))

    # Cold start: spawn -> initialize -> tools/list reply.
    colds = []
    for _ in range(10):
        t = time.perf_counter()
        m = Mcp(perci, env)
        m.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1"}})
        m.notify("notifications/initialized")
        m.call("tools/list")
        colds.append((time.perf_counter() - t) * 1000)
        m.close()

    m = Mcp(perci, env)
    m.call("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1"}})
    m.notify("notifications/initialized")
    pings = []
    for _ in range(a.n):
        t = time.perf_counter()
        m.call("ping")
        pings.append((time.perf_counter() - t) * 1000)

    rows = []
    for name, args in cases:
        n = 3 if name == "agent_verify_patch" else (20 if name == "status" else a.n)
        rt, el, st = [], [], None
        for i in range(n + 3):
            t = time.perf_counter()
            r = m.call("tools/call", {"name": name, "arguments": args})
            dt = (time.perf_counter() - t) * 1000
            sc = r["result"]["structuredContent"]
            st = sc["status"]
            if i >= 3:
                rt.append(dt)
                el.append(sc["receipt"]["elapsed_us"])
        label = name + (" (" + ",".join(f"{k}" for k in list(args)[:2]) + ")" if name in ("exact", "route", "policy_check", "agent_gate_step") else "")
        if name == "route":
            label = "route (" + args.get("pack", "v0.2") + ")"
        if name == "exact":
            label = "exact (" + ("check claim" if "claimed" in args else "compute") + ")"
        if name == "agent_gate_step":
            label = "agent_gate_step (" + args["phase"] + ")"
        rows.append({"tool": label, "status": st, "n": n, "rt_p50_ms": pct(rt, 50), "rt_p95_ms": pct(rt, 95),
                     "work_p50_us": pct(el, 50)})
    m.close()

    # One-shot process per call.
    oneshot = []
    for _ in range(20):
        t = time.perf_counter()
        subprocess.run([perci, "tool", "exact", "--json", '{"expr":"17% of 240"}'], env=env, stdout=subprocess.DEVNULL)
        oneshot.append((time.perf_counter() - t) * 1000)

    # Chat daemon TCP ping.
    daemon = None
    s0 = socket.socket()
    s0.bind(("127.0.0.1", 0))
    port = s0.getsockname()[1]
    s0.close()
    denv = dict(env, PERCI_DAEMON_PORT=str(port))
    d = subprocess.Popen([perci, "daemon"], env=denv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.time() + 30
        while True:
            try:
                c = socket.create_connection(("127.0.0.1", port), timeout=1)
                break
            except OSError:
                if time.time() > deadline:
                    raise
                time.sleep(0.02)
        c.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        f = c.makefile("rb")
        xs = []
        for i in range(a.n + 10):
            t = time.perf_counter()
            c.sendall(b'{"op":"ping"}\n')
            f.readline()
            if i >= 10:
                xs.append((time.perf_counter() - t) * 1000)
        daemon = {"p50_ms": pct(xs, 50), "p95_ms": pct(xs, 95)}
        c.close()
    finally:
        d.kill()
        d.wait()

    out = {
        "machine": f"{sys.platform} {os.cpu_count()} cpu",
        "mcp_cold_start_to_tools_list_ms": {"p50": pct(colds, 50), "max": max(colds)},
        "mcp_ping_ms": {"p50": pct(pings, 50), "p95": pct(pings, 95)},
        "tools": rows,
        "oneshot_perci_tool_exact_ms": {"p50": pct(oneshot, 50)},
        "daemon_tcp_ping_ms": daemon,
    }
    print(f"machine: {out['machine']}")
    print(f"MCP cold start -> tools/list: p50 {out['mcp_cold_start_to_tools_list_ms']['p50']:.1f} ms")
    print(f"MCP ping round trip: p50 {out['mcp_ping_ms']['p50']*1000:.0f} us, p95 {out['mcp_ping_ms']['p95']*1000:.0f} us")
    print(f"one-shot `perci tool exact` process: p50 {out['oneshot_perci_tool_exact_ms']['p50']:.1f} ms")
    if daemon:
        print(f"chat daemon TCP ping: p50 {daemon['p50_ms']*1000:.0f} us, p95 {daemon['p95_ms']*1000:.0f} us")
    print()
    print("| tool | verdict | round trip p50 | round trip p95 | tool work p50 |")
    print("|---|---|---|---|---|")
    for r in rows:
        def ms(x):
            return f"{x:.3f} ms" if x < 10 else f"{x:.0f} ms"
        w = r["work_p50_us"]
        print(f"| {r['tool']} | {r['status']} | {ms(r['rt_p50_ms'])} | {ms(r['rt_p95_ms'])} | {w} µs |")
    if a.json:
        with open(a.json, "w") as f:
            json.dump(out, f, indent=2)
    shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
