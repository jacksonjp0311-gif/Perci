# Perci as a tool

Perci's checks as stateless tools that AI harnesses (Claude Code, Codex,
Cursor, custom loops) call before they trust an answer or run an action.
Every call returns a **`perci.verdict.v1`**: `pass` / `fail` / `abstain` /
`error`, machine-readable reasons, evidence, and a receipt whose
`input_sha256` / `output_sha256` make the result replayable.

```
perci mcp                                # MCP server over stdio (JSON-RPC 2.0, one message per line)
perci tool list                          # enabled tools
perci tool schema [NAME]                 # input schemas + the verdict schema
perci tool exact --json '{"expr":"17% of 240","claimed":"41"}'   # one-shot; also @file or stdin
perci ask --stateless "what is 3/7 plus 2/9"                     # no session, no learning, byte-identical
```

One-shot exit codes: `0` pass, `1` fail, `4` abstain, `3` error, `2` usage.

## Tools

Wire names (MCP) drop the `perci.` prefix and use `_` (model APIs allow only
`[A-Za-z0-9_-]`); both spellings are accepted by `perci tool`.

| wire name | tool | what it decides | default |
|---|---|---|---|
| `exact` | perci.exact | rational arithmetic, %, powers, √, primality, linear equations; checks a `claimed` value (rounding accepted only when it is honest rounding) | on |
| `check_math` | perci.check_math | finds every `expr = value` claim in free text and recomputes it | on |
| `relation` | perci.relation | closed-world consequences of premises under an intervention; checks `claimed_impacted` / `claimed_invariant` | on |
| `route` | perci.route | deterministic route: refuse → exact → relation → recall → llm, plus the Bitwork label (v0.2 pack) | on |
| `policy_check` | perci.policy_check | structural gate for shell / write / delete / network / commit / push (`config/policy.json`) | on |
| `recall` | perci.recall | cited memory lookup through `CitedMemory` (AgentSearch, substring fallback); embeds `perci.recall.v1` | on |
| `verify` | perci.verify | one call: hygiene + math + evidence hashes (file, `memory:N`, or a recall citation) + relation + policy | on |
| `status` | perci.status | doctor report, enabled tools, policy hash, recall backend | on |
| `agent_check_claims` | perci.agent.check_claims | splits an answer into claims, says which Perci could check and how | on |
| `agent_gate_step` | perci.agent.gate_step | `pre`: policy before a step; `post`: exit code, math and hygiene of its output | on |
| `remember` | perci.remember | appends a note to the journal through `CitedMemory::remember`; refuses secrets | **off** |
| `agent_verify_patch` | perci.agent.verify_patch | applies a diff in a scratch worktree and runs an allowlisted verifier; the caller's tree is untouched | **off** |
| `eval` | perci.eval | bug2eval through `perci::b2e`: `validate`, `validate_all`, `run` (needs `PERCI_EVAL_ALLOW_AGENT=1`) | **off** |

`PERCI_TOOLS=all` (or a comma list such as `exact,verify,remember`) changes the
set. Off-by-default tools write, execute code or take minutes.

### Status meanings

* `pass`: Perci checked it and it holds. Only `pass` may be shown as verified.
* `fail`: Perci found something wrong; `reasons[]` says what (e.g. `MATH_MISMATCH: 17% of 240: claimed 41, exact 204/5 (40.8)`). Feed the reasons back to the model.
* `abstain`: Perci could not check it (unparsed, ambiguous notation, nothing checkable, needs approval). Never present it as verified.
* `error`: bad arguments or an internal failure (panics are caught and become `error`).

`confidence` is `exact` (exact arithmetic, hash comparisons), `bounded` (closed
world: only the premises/notes given) or `heuristic` (keyword policy escalation).

### Receipt

`input_sha256` = sha256 of canonical JSON `{tool, args}`; `output_sha256` =
sha256 of canonical `{status, result, reasons, evidence}` (sorted keys, floats
as strings). `elapsed_us`, `config` (the `PERCI_*` variables that affect output)
and `weights` (pack identity) are recorded but not hashed. `deterministic:
false` marks tools whose output depends on the outside world (remember, eval,
verify_patch).

## Harness configuration

All three hosts launch `perci mcp` as a child process. Use absolute paths and
set `PERCI_HOME` to the checkout (weights, `config/policy.json`).

**Claude Code** (`.mcp.json` in the project, or `claude mcp add perci -- /path/to/perci mcp`):

```json
{"mcpServers": {"perci": {"command": "/path/to/perci", "args": ["mcp"],
  "env": {"PERCI_HOME": "/path/to/Perci"}}}}
```

**Codex** (`~/.codex/config.toml`):

```toml
[mcp_servers.perci]
command = "/path/to/perci"
args = ["mcp"]
env = { PERCI_HOME = "/path/to/Perci" }
```

**Cursor** (`.cursor/mcp.json` or `~/.cursor/mcp.json`): same JSON as Claude Code.

Suggested system-prompt line (the server's `instructions` say the same):
"Call `verify` on any answer with numbers or citations before presenting it,
and `agent_gate_step` (phase `pre`) before shell commands or writes. On `fail`
fix what the reasons say; on `abstain` say it is unverified."

None of these configs have been exercised from the real hosts yet (see Known gaps).

## Latency (box: Linux, 8 vCPU, release build, Oct 17 2026)

`scripts/bench_tools.py` (n = 200 per tool). Round trip = harness writes the
request → reads the reply over MCP stdio; work = the tool's own `elapsed_us`.

| tool | verdict in bench | round trip p50 | p95 | work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.045 ms | 0.054 ms | 5 µs |
| exact (check claim) | fail | 0.053 ms | 0.071 ms | 8 µs |
| check_math | fail | 0.085 ms | 0.124 ms | 24 µs |
| relation | pass | 0.203 ms | 0.262 ms | 128 µs |
| route (v0.2, default) | pass | 0.756 ms | 0.918 ms | 672 µs |
| route (v0.3) | pass | 7.96 ms | 11 ms | 7.65 ms |
| policy_check | fail | 0.055 ms | 0.080 ms | 6–10 µs |
| recall, AgentSearch (201 notes, warm) | pass | 1.79 ms | 2.40 ms | 1.56 ms |
| recall, substring fallback | pass | 0.073 ms | 0.076 ms | 17 µs |
| remember | pass | 0.243 ms | 0.515 ms | 185 µs |
| verify (math + relation + policy) | fail | 0.280 ms | 0.348 ms | 189 µs |
| status | pass | 0.158 ms | 0.200 ms | 58 µs |
| agent_check_claims | fail | 0.124 ms | 0.144 ms | 49 µs |
| agent_gate_step (pre / post) | fail / pass | 0.061 / 0.062 ms | 0.069 / 0.074 ms | 10 µs |
| agent_verify_patch (python verifier) | pass | 185 ms | 185 ms | 185 ms |
| eval validate B2E-PERCI-001 | pass | 35.4 s (one-shot) | | |

Other: MCP `ping` 15 µs; cold start → `tools/list` 3.2 ms; one-shot `perci tool
exact` process 1.1 ms; chat daemon TCP `ping` 18 µs (was 44 ms before
`TCP_NODELAY` + single-write replies). First recall in a fresh data dir pays
note sync + AgentSearch start (~120 ms); `perci mcp` does that in the
background at start (`PERCI_MCP_NO_PREWARM=1` disables it). Server-side
dispatch + seal overhead is ~9 µs (`cargo run --release --example tool_overhead`).

## Quality gates

* `evals/tools-heldout-v1/run_heldout.py` (seed 20261017): independent `Fraction`
  oracle. exact: 197 items, 193 correct, 4 ambiguous `a/b / c/d` abstained as
  expected, **0 wrong passes**, coverage 100%. check_math: 106/106 planted errors
  caught, **0 false fails** on 94 correct answers. Replay: identical
  `output_sha256` across two processes. Exits 1 on any gate failure.
* `evals/demo/harness.py`: generator → verify → retry loop with a scripted model
  (or `--ollama MODEL`); transcript in `evals/demo/transcript.md`, receipts in
  `evals/demo/receipts.jsonl`.

## Extension points

* `perci::tools::register(Arc<dyn Tool>)` adds or overrides a tool by name.
* `Ctx::with_memory(CitedMemory)` points recall/remember/verify at another journal.
* Boundary: `src/tools/` must not use `chat`, `voice`, `deliberation` or
  `session` (`tests/tool_boundary.rs`), the seam for a later `perci-core` crate.
