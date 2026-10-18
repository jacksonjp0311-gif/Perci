# Perci: a fast, deterministic verifier and gate for AI agents

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" alt="Perci mark" width="96" height="96">
</p>

<p align="center">
  <a href="https://github.com/jacksonjp0311-gif/Perci/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/jacksonjp0311-gif/Perci/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/jacksonjp0311-gif/Perci/releases"><img alt="Release" src="https://img.shields.io/github/v/release/jacksonjp0311-gif/Perci?include_prereleases&label=release"></a>
  <a href="#license"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue"></a>
  <img alt="Status: preview" src="https://img.shields.io/badge/status-preview-orange">
  <img alt="MCP stdio" src="https://img.shields.io/badge/MCP-stdio-5c0a12">
</p>

> **Preview.** Perci 0.11 is usable today and tested on Linux, macOS and
> Windows runners, but its interfaces may still change and the release binaries are not
> code-signed yet. See [Status](#status-preview).

Your agent says *"17% of 240 is 41"*, cites a note that has changed, or wants
to `git push` before the tests ran. Perci catches each of these before it
reaches the user or the repository. It is a local MCP server and CLI that
**recomputes math, checks claimed consequences and citations, gates shell
steps and writes, and keeps a hash-chained ledger of the run**, and its
**Temporal Router** learns from those ledgers what your system usually does
next. Every answer is a verdict (`pass` / `fail` / `abstain` / `error`) with
machine-readable reasons and a replayable receipt. Same input, same verdict,
byte for byte. Most checks take microseconds. No API key, no network, no model
in the loop.

## Why

Agents are good at producing plausible text and plausible actions. They are
bad at noticing when the arithmetic is off, when a cited file no longer says
what they claim, or when a step is out of order. Asking another model to
check is slow, costs tokens and can be wrong in the same way. Perci answers
those questions by computation and by rules you can read, says "abstain"
when it cannot check something instead of guessing, and leaves a receipt
you can verify later.

## Quickstart

```sh
curl -fsSL https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.sh | sh     # macOS / Linux
irm https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.ps1 | iex           # Windows
claude mcp add --scope user perci -- perci mcp                                                 # then connect your agent
```

The installer checks the download against the release's `SHA256SUMS`, puts
Perci in `~/.perci` (Windows: `%LOCALAPPDATA%\Perci\app`), downloads and
verifies the weights, and runs `perci doctor`. Cursor, Codex, Claude Desktop
and generic-harness configs, verifying downloads, and what each tool does:
**[docs/QUICKSTART.md](docs/QUICKSTART.md)**.

## 60-second demo

```text
$ echo '{"expr":"17% of 240","claimed":"41"}' | perci tool exact
  FAIL     EXACT_MISMATCH: claimed 41, exact 204/5 (40.8)
$ echo '{"text":"... 3 x 42 = 128."}' | perci tool check_math
  FAIL     MATH_MISMATCH: 3 x 42: claimed 128, exact 126
$ echo '{"action":{"kind":"shell","command":"git push --force origin main"}}' | perci tool policy_check
  FAIL     POLICY_REFUSE: git.force_push: rewrites or deletes remote history
$ PERCI_RUN_ID=demo  … agent_gate_step pre  "git push origin main"
  FAIL     SEQUENCE_REQUIRES: seq.deploy_requires_tests: deploying requires tests that passed after the last code change   ledger:demo#1
$ PERCI_RUN_ID=demo  … agent_gate_step post "cargo test" exit 0
  PASS                                                                                                    ledger:demo#2
$ PERCI_RUN_ID=demo  … agent_gate_step pre  "git push origin main"
  ABSTAIN  NEEDS_APPROVAL: git.history: publishes   (sequence rule satisfied, cites ledger:demo#2)
$ PERCI_RUN_ID=demo  … ledger_verify
  PASS     3 entries, chain intact
```

## Features

* **Exact math, not estimates.** `exact` does checked rational arithmetic
  (fractions, %, powers, roots, primality, linear equations) and checks a
  claimed value; `check_math` finds every `expr = value` claim in free text and
  recomputes it.
* **Claims and citations.** `verify` checks an answer in one call: math,
  evidence hashes (a file line, a memory note, a recall citation),
  consequences of stated premises (`relation`) and policy.
* **A gate for actions.** `policy_check` and `agent_gate_step` classify shell
  commands, writes, deletes, network calls, commits and pushes structurally
  (argv and paths, looking through `sudo`, `bash -c`, PowerShell wrappers) as
  allow / needs approval / refuse, against a policy file you can read and
  extend.
* **Sequence rules over a hash-chained ledger.** Every gated step is recorded
  in an append-only, hash-chained run ledger. Rules such as "deploy only after
  tests passed since the last code change" are checked against it, and
  verdicts cite the ledger entries they relied on. `ledger_verify` detects
  edits, reordering and dropped entries.
* **The Temporal Router.** See below.
* **Receipts.** Every verdict carries `input_sha256`, `output_sha256` and a
  receipt id, `receipt_sha256`, that binds the input, the output and the
  engine/pack version, so two different questions never share a receipt.
* **Calibrated routing.** `route` gives a deterministic route (refuse / exact /
  relation / recall / llm) from Perci's Bitwork weight pack with calibrated
  confidence, and abstains below the confidence gate.
* **Your rules, outside the install.** Profiles (`PERCI_PROFILE`) add policy
  overlays, sequence rules and your own weight packs, all sha256-pinned
  ([docs/PROFILES.md](docs/PROFILES.md)).

### The Temporal Router

`next_step` (also callable as `temporal_route`) is Perci's Temporal Router. It
learns integer n-gram counts of step sequences from every run ledger whose
hash chain verifies (`perci ledger learn`; broken chains are skipped and
reported) plus the current run. Given where a run is now, it returns the steps
that usually come next, each with the counts behind it (`count` of `of`, and
how those steps ended), warns when a planned step would break a sequence rule
(for example, a push before the tests have passed), and flags a planned call
that Perci already answered (cacheable) or that already failed with the same
input. When it has not seen the context it abstains (`NO_HISTORY`) instead of
guessing.

It is deterministic and runs in well under a millisecond on short runs (see
the benchmark table). It is advisory: `agent_gate_step` enforces, the router
suggests. There is no accuracy benchmark for its suggestions yet. How useful
they are depends on how repetitive your own runs are, and they get better as
your ledgers grow.

## Benchmarks

`scripts/bench_public.py` against an installed release (Linux x64, 8 threads;
round trip = harness writes the MCP request → reads the reply; work = the
tool's own time). Full results, other platforms and how to reproduce them:
[docs/BENCHMARKS.md](docs/BENCHMARKS.md).

| tool | round trip p50 | work p50 |
|---|---|---|
| `exact` (check a claimed value) | 0.058 ms | 7 µs |
| `check_math` (3 claims in a paragraph) | 0.12 ms | 32 µs |
| `policy_check` (shell command) | 0.066 ms | 9 µs |
| `agent_gate_step` (pre, policy + sequence rules) | 0.17 ms | 110 µs |
| `agent_check_claims` | 0.093 ms | 27 µs |
| `relation` (consequences of premises) | 0.19 ms | 108 µs |
| `ledger_append` | 0.048 ms | <1 µs |
| `next_step` (Temporal Router) | 0.17 ms | 123 µs |
| `route` (v0.2 pack, calibrated) | 0.52 ms | 389 µs |

MCP cold start to `tools/list` 3.3 ms; a one-shot `perci tool` process 1.2 ms.
Seeded correctness run (independent `fractions.Fraction` oracle): 300/300
planted wrong values rejected, 0/150 correct answers flagged, 150/150 wrong
ones caught, 0/13 dangerous commands allowed, and 600/600 verdict hashes
identical across processes.

## How it works

* **Tool layer** (`src/tools/`): small, stateless tools behind one dispatcher.
  Each returns an `Outcome` that is sealed into a `perci.verdict.v1` with
  canonical-JSON hashes. Exposed over MCP stdio (`perci mcp`) and as a
  one-shot CLI (`perci tool NAME`). A per-process verdict cache answers
  repeated identical calls in a few microseconds.
* **Policy and ledger**: `config/policy.json` (shell / write / delete /
  network / commit / push rules plus sequence rules) and the hash-chained
  JSONL ledgers under the profile. Nothing leaves the machine.
* **Bitwork** (only `route` and the chat engine need it): Perci's own sparse
  binary associative weights (`PERCIW03`, ~200 MB; the `tools` set is ~20 MB).
  Routing is a popcount scan over packed bit vectors on up to 8 threads,
  integer-only and deterministic, with calibration buckets measured on
  held-out cases. Weights are released as separate, sha256-checked assets
  (`perci fetch-weights`).
* **Research engine**: Perci also contains an experimental local chat engine
  built on the same weights (operators, exact tools, governed memory). The
  tool layer does not depend on it; `exact`, `check_math`, `policy_check` and
  the ledger need no weights at all. Its history and measurements are in
  [docs/RESEARCH_LOG.md](docs/RESEARCH_LOG.md).

## Harness integrations

| host | how | tested |
|---|---|---|
| Claude Code | `claude mcp add --scope user perci -- perci mcp`, or a project `.mcp.json` | yes: `claude mcp list` shows `✔ Connected` |
| Cursor (CLI and editor) | `~/.cursor/mcp.json` or `.cursor/mcp.json` | CLI yes: `cursor-agent mcp list-tools perci` lists the 14 tools; editor not run |
| Codex CLI | `~/.codex/config.toml` `[mcp_servers.perci]` or `codex mcp add` | config parsed by `codex mcp list`; launch checked with the MCP inspector |
| Claude Desktop | `claude_desktop_config.json` | config validated, not host-tested |
| Any MCP client | stdio, newline-delimited JSON-RPC 2.0 | MCP Inspector CLI: `tools/list` and `tools/call` |
| Anything else | `perci tool NAME --json '<args>'`, exit code 0 pass / 1 fail / 4 abstain / 3 error | CI |

Details and exact configs: [docs/QUICKSTART.md](docs/QUICKSTART.md#host-test-status).

## Status: preview

What works and is tested: the tool layer (CI runs the tests on Linux and
Windows on every push; the release workflow builds Linux x64, macOS arm64/x64
and Windows x64, smoke-tests every archive, and installs the Linux, macOS
arm64 and Windows builds with the real installers before publishing), the
seeded correctness gates above, and the host CLIs in the table.

Not done yet:

* Release binaries are **not code-signed or notarized**. Windows SmartScreen
  and macOS Gatekeeper may warn on a browser download; verify against
  `SHA256SUMS` first ([how](docs/QUICKSTART.md#verifying-downloads-unsigned-preview-builds)).
* Model-driven tool calls inside Claude Code, Codex and Cursor need accounts
  and have not been run in CI; the hosts were checked to start Perci and list
  its tools.
* The Temporal Router has no accuracy benchmark yet.
* `perci.verdict.v1` may gain fields before 1.0 (it gained
  `receipt_sha256` in 0.11.8); existing fields keep their meaning.

Changes per release: [CHANGELOG.md](CHANGELOG.md). Design decisions:
[docs/DECISIONS.md](docs/DECISIONS.md).

## Documentation

* [QUICKSTART](docs/QUICKSTART.md): install, connect a host, demo, verify downloads
* [TOOLS](docs/TOOLS.md): every tool, the verdict and receipt format, latency
* [PROFILES](docs/PROFILES.md): your own policy, sequence rules and weight packs
* [BENCHMARKS](docs/BENCHMARKS.md): how the numbers above are measured, on three OSes
* [docs/README.md](docs/README.md): index of everything else, including research notes

## Contributing and security

Issues and pull requests are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).
Please report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
