# Perci benchmarks

One script, `scripts/bench_public.py`, measures both **correctness** and
**latency** through the same MCP stdio transport an agent harness uses. It
ships inside every release archive, so you can run it against your own
install:

```sh
python3 ~/.perci/scripts/bench_public.py --json bench.json --md bench.md         # macOS / Linux
python "$env:LOCALAPPDATA\Perci\app\scripts\bench_public.py" --md bench.md   # Windows
python3 scripts/bench_public.py --perci target/release/perci                    # source checkout
```

Standard library Python 3.8+ only. It runs in a temporary data folder and
sends nothing anywhere. Exit code 1 means a correctness gate failed. Options:
`--seed` (default 7), `--n` (cases per correctness suite, default 300),
`--iters` (calls per latency row, default 200).

## What it measures

**Correctness.** Every case is generated from the seed and graded by an oracle
that does not share code with Perci (Python's `fractions.Fraction`):

| suite | cases | gate |
|---|---|---|
| `exact` | N expressions (mixed integer arithmetic with precedence, fractions, percentages, powers, products), each submitted once with the right value and once with a planted wrong one | **0** wrong values accepted |
| `check_math` | N one-step answers in prose (`+ - * x ×`), half with a planted error | **0** correct answers flagged; catch rate reported |
| `policy_check` | the 21 commands in `evals/bench-public-v1/policy_cases.json` (8 everyday, 13 destructive or publishing). They are classified only, never executed | **0** dangerous commands allowed |
| replay | every `exact` / `check_math` verdict re-run in a second server process | identical `output_sha256` |

**Latency.** These are measured on a fresh run ledger, so ledger-backed tools see a short history:

* MCP cold start: spawn `perci mcp`, `initialize`, first `tools/list` reply.
* MCP `ping` round trip.
* Per tool, p50 and p95 of the client round trip (request written to reply
  parsed) and p50 of the tool's own `receipt.elapsed_us`, with the verdict cache
  off.
* A one-shot `perci tool exact` process, start to exit.

The `route` row needs the v0.2 pack (`perci fetch-weights --set tools`) and is
skipped without it.

Limits, stated plainly: the synthetic suites are easy on purpose. They
check that Perci never accepts a wrong value and never flags a right one. They
are not a measure of how much of real agent output Perci can parse (it abstains
on what it cannot parse, and abstentions are counted). The harder seeded
held-out gate is `evals/tools-heldout-v1/run_heldout.py`, which CI runs on
every push. `ledger_verify` and `next_step` grow with the length of the run.
See the 3,457-entry numbers in [TOOLS.md](TOOLS.md#latency-box-linux-8-vcpu-release-build-oct-17-2026).

## Results

### Linux x64 (8 vCPU Xeon VM, installed release build, Oct 17 2026)

Perci 0.11.7 · Linux x86_64 · Intel(R) Xeon(R) Processor · 8 threads · seed 7

| correctness | result |
|---|---|
| exact: right values accepted | 300/300 (0 abstained) |
| exact: planted wrong values accepted | **0**/300 (caught 300) |
| check_math: correct answers flagged | **0**/150 |
| check_math: wrong answers caught | 150/150 |
| policy: dangerous commands allowed | **0**/13 |
| policy: safe commands blocked | 0/8 |
| replay: verdict hashes identical across processes | 600/600 |

MCP cold start to tools/list p50 3.3 ms · ping p50 14 µs · one-shot `perci tool` process p50 1.2 ms

| tool | verdict | round trip p50 | round trip p95 | tool work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.054 ms | 0.070 ms | 5 µs |
| exact (check claim) | fail | 0.058 ms | 0.071 ms | 7 µs |
| check_math (3 claims, 1 wrong) | fail | 0.124 ms | 0.151 ms | 32 µs |
| relation | pass | 0.185 ms | 0.378 ms | 108 µs |
| policy_check (shell) | fail | 0.066 ms | 0.073 ms | 9 µs |
| agent_gate_step (pre) | pass | 0.171 ms | 0.195 ms | 110 µs |
| agent_check_claims | fail | 0.093 ms | 0.107 ms | 27 µs |
| ledger_append | pass | 0.048 ms | 0.057 ms | 0 µs |
| next_step | pass | 0.170 ms | 0.197 ms | 123 µs |
| route (v0.2) | abstain | 0.518 ms | 0.767 ms | 389 µs |

<!-- ci-results -->
Every `workflow_dispatch` run of the release workflow (`.github/workflows/release.yml`)
installs the freshly built assets with the real installers on GitHub's
Linux, macOS (arm64) and Windows runners and runs this benchmark there. The
tables are in the run summary and in the `bench-public-*` artifacts.
