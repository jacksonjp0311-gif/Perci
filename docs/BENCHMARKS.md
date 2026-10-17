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

### Linux x64 (8 vCPU Xeon VM, release build installed with install.sh, Oct 17 2026)

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

### Linux x64, GitHub `ubuntu-latest` runner (4 vCPU EPYC 7763; static musl build)

Release dry run 39056675173 (Oct 17 2026, `--n 150 --iters 100`), installed with the real installer (`install.sh`, or `install.ps1` through `irm | iex`).

Perci 0.11.7 · Linux x86_64 · AMD EPYC 7763 64-Core Processor · 4 threads · seed 7

| correctness | result |
|---|---|
| exact: right values accepted | 150/150 (0 abstained) |
| exact: planted wrong values accepted | **0**/150 (caught 150) |
| check_math: correct answers flagged | **0**/75 |
| check_math: wrong answers caught | 75/75 |
| policy: dangerous commands allowed | **0**/13 |
| policy: safe commands blocked | 0/8 |
| replay: verdict hashes identical across processes | 300/300 |

MCP cold start to tools/list p50 7.1 ms · ping p50 40 µs · one-shot `perci tool` process p50 0.9 ms

| tool | verdict | round trip p50 | round trip p95 | tool work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.257 ms | 0.340 ms | 15 µs |
| exact (check claim) | fail | 0.250 ms | 0.302 ms | 16 µs |
| check_math (3 claims, 1 wrong) | fail | 0.214 ms | 0.284 ms | 49 µs |
| relation | pass | 0.395 ms | 0.616 ms | 187 µs |
| policy_check (shell) | fail | 0.210 ms | 0.239 ms | 25 µs |
| agent_gate_step (pre) | pass | 0.249 ms | 0.387 ms | 59 µs |
| agent_check_claims | fail | 0.262 ms | 0.347 ms | 56 µs |
| ledger_append | pass | 0.148 ms | 0.177 ms | 2 µs |
| next_step | pass | 0.141 ms | 0.218 ms | 33 µs |
| route (v0.2) | abstain | 0.801 ms | 1.302 ms | 599 µs |


### macOS arm64, GitHub `macos-14` runner (3 vCPU Apple M1, virtual)

Release dry run 39056675173 (Oct 17 2026, `--n 150 --iters 100`), installed with the real installer (`install.sh`, or `install.ps1` through `irm | iex`).

Perci 0.11.7 · Darwin arm64 · Apple M1 (Virtual) · 3 threads · seed 7

| correctness | result |
|---|---|
| exact: right values accepted | 150/150 (0 abstained) |
| exact: planted wrong values accepted | **0**/150 (caught 150) |
| check_math: correct answers flagged | **0**/75 |
| check_math: wrong answers caught | 75/75 |
| policy: dangerous commands allowed | **0**/13 |
| policy: safe commands blocked | 0/8 |
| replay: verdict hashes identical across processes | 300/300 |

MCP cold start to tools/list p50 6.3 ms · ping p50 14 µs · one-shot `perci tool` process p50 3.1 ms

| tool | verdict | round trip p50 | round trip p95 | tool work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.094 ms | 0.176 ms | 4 µs |
| exact (check claim) | fail | 0.098 ms | 0.187 ms | 7 µs |
| check_math (3 claims, 1 wrong) | fail | 0.180 ms | 0.565 ms | 28 µs |
| relation | pass | 0.312 ms | 0.661 ms | 143 µs |
| policy_check (shell) | fail | 0.151 ms | 0.400 ms | 12 µs |
| agent_gate_step (pre) | pass | 0.154 ms | 0.593 ms | 31 µs |
| agent_check_claims | fail | 0.196 ms | 0.413 ms | 43 µs |
| ledger_append | pass | 0.118 ms | 0.328 ms | 0 µs |
| next_step | pass | 0.078 ms | 0.213 ms | 18 µs |
| route (v0.2) | abstain | 0.648 ms | 0.993 ms | 399 µs |


### Windows x64, GitHub `windows-latest` runner (4 vCPU)

Release dry run 39056675173 (Oct 17 2026, `--n 150 --iters 100`), installed with the real installer (`install.sh`, or `install.ps1` through `irm | iex`).

Perci 0.11.7 · Windows AMD64 · Intel64 Family 6 Model 207 Stepping 2, GenuineIntel · 4 threads · seed 7

| correctness | result |
|---|---|
| exact: right values accepted | 150/150 (0 abstained) |
| exact: planted wrong values accepted | **0**/150 (caught 150) |
| check_math: correct answers flagged | **0**/75 |
| check_math: wrong answers caught | 75/75 |
| policy: dangerous commands allowed | **0**/13 |
| policy: safe commands blocked | 0/8 |
| replay: verdict hashes identical across processes | 300/300 |

MCP cold start to tools/list p50 8.2 ms · ping p50 58 µs · one-shot `perci tool` process p50 6.0 ms

| tool | verdict | round trip p50 | round trip p95 | tool work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.276 ms | 0.406 ms | 11 µs |
| exact (check claim) | fail | 0.265 ms | 0.327 ms | 15 µs |
| check_math (3 claims, 1 wrong) | fail | 0.360 ms | 0.514 ms | 48 µs |
| relation | pass | 0.623 ms | 0.722 ms | 188 µs |
| policy_check (shell) | fail | 0.385 ms | 0.463 ms | 28 µs |
| agent_gate_step (pre) | pass | 0.436 ms | 0.497 ms | 70 µs |
| agent_check_claims | fail | 0.356 ms | 0.469 ms | 63 µs |
| ledger_append | pass | 0.226 ms | 0.287 ms | 1 µs |
| next_step | pass | 0.104 ms | 0.125 ms | 22 µs |
| route (v0.2) | abstain | 0.980 ms | 1.080 ms | 599 µs |


Every `workflow_dispatch` run of the release workflow (`.github/workflows/release.yml`)
installs the freshly built assets with the real installers on GitHub's
Linux, macOS (arm64) and Windows runners and runs this benchmark there. The
tables are in the run summary and in the `bench-public-*` artifacts.
Shared CI runners are noisy. Their round trips run 2–4× the dedicated box above, while tool work is similar.
