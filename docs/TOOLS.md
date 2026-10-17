# Perci as a tool

Perci's checks as stateless tools that AI harnesses (Claude Code, Codex,
Cursor, custom loops) call before they trust an answer or run an action.
Every call returns a **`perci.verdict.v1`**: `pass` / `fail` / `abstain` /
`error`, machine-readable reasons, evidence, and a receipt whose
`input_sha256` / `output_sha256` make the result replayable.

```
perci mcp                                # MCP server over stdio (JSON-RPC 2.0, one message per line)
perci tool list                          # enabled tools
perci tool schema [NAME|verdict]         # a tool's input schema (all tools without NAME), or the verdict schema
perci packs list | verify | calibrate    # weight packs (docs/PROFILES.md)
perci ledger verify RUN | learn          # run ledgers and the temporal model
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
| `route` | perci.route | deterministic route: refuse → exact → relation → recall → llm, plus the label of any verified PERCIW03 weight pack (its own labels) with calibrated confidence; an `llm` route below the gate abstains | on |
| `policy_check` | perci.policy_check | structural gate for shell / write / delete / network / commit / push (`config/policy.json`) | on |
| `recall` | perci.recall | cited memory lookup through `CitedMemory` (AgentSearch, substring fallback); embeds `perci.recall.v1` | on |
| `verify` | perci.verify | one call: hygiene + math + evidence hashes (file, `memory:N`, or a recall citation) + relation + policy | on |
| `status` | perci.status | doctor report, enabled tools, policy hash, recall backend | on |
| `agent_check_claims` | perci.agent.check_claims | splits an answer into claims, says which Perci could check and how | on |
| `agent_gate_step` | perci.agent.gate_step | `pre`: policy and sequence rules (against the run ledger) before a step; `post`: exit code, math and hygiene of its output | on |
| `ledger_append` | perci.ledger_append | records a step the harness ran itself (e.g. tests passed) in the run ledger; gated calls are recorded implicitly | on |
| `ledger_verify` | perci.ledger_verify | re-hashes a run's chain from disk: edits, reordering, drops, moves; `expect_head` catches truncation | on |
| `ledger_show` | perci.ledger_show | the last entries of a run, plus whether the chain verifies | on |
| `next_step` | perci.next_step | temporal router: likely next steps with counts, sequence-rule warnings, cache / repeat-failure hints for a planned call | on |
| `remember` | perci.remember | appends a note to the journal through `CitedMemory::remember`; refuses secrets | **off** |
| `agent_verify_patch` | perci.agent.verify_patch | applies a diff in a scratch worktree and runs an allowlisted verifier; the caller's tree is untouched | **off** |
| `eval` | perci.eval | bug2eval through `perci::b2e`: `validate`, `validate_all`, `run` (needs `PERCI_EVAL_ALLOW_AGENT=1`) | **off** |

`PERCI_TOOLS=all` (or a comma list such as `exact,verify,remember`) changes the
set; without it the profile's `tools.enabled` applies. Off-by-default tools
write user state, execute code or take minutes (`ledger_append` writes only
Perci's own append-only run ledger).

### Run ledger, sequence rules, next_step

`perci mcp` records every verdict (except the read-only meta tools
`ledger_show`, `ledger_verify`, `next_step`, `status`) in a hash-chained ledger
for the session, `PERCI_PROFILE/ledger/<run>.jsonl` (`PERCI_RUN_ID`, else
`mcp-<unix>-<pid>`; `PERCI_LEDGER=off` disables). The receipt names the entry
(`receipt.ledger = "ledger:<run>#<seq>"`). `agent_gate_step` `pre` evaluates
the policy's sequence rules against the ledger: `git push` before tests passed
after the last write is refused (`SEQUENCE_REQUIRES`), a destructive delete
without a backup needs approval, and a satisfied rule cites the entry it
relied on as `ledger_entry` evidence. Without a run ledger the rule is
reported as `SEQUENCE_UNCHECKED` (abstain). `next_step` learns integer n-gram
counts (order 3 with back-off) from verified ledgers (`perci ledger learn`)
plus the current run, and returns `{step, count, of, permille, outcomes}` rows
so the suggestion is explainable. Layout and formats: `docs/PROFILES.md`.

### Verdict cache

`exact`, `check_math`, `relation`, `route` and `policy_check` verdicts are
cached per process by `input_sha256` + output config (4096 entries, FIFO;
errors are not cached; `PERCI_TOOL_CACHE=0` disables). A hit returns the same
`output_sha256` with `receipt.cache_hit: true` and the lookup time as
`elapsed_us`. Profile changes made while a server runs are not seen by cached
verdicts; restart the server.

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
as strings). `elapsed_us`, `config` (the `PERCI_*` variables that affect output),
`weights` (verified pack identity `name@version manifest-sha256:…`),
`cache_hit` and `ledger` (the run-ledger entry) are recorded but not hashed. `deterministic:
false` marks tools whose output depends on the outside world (remember, eval,
verify_patch).

## Harness configuration

All three hosts launch `perci mcp` as a child process. An installed release
(see [QUICKSTART.md](QUICKSTART.md)) needs only `"command": "perci"` (or its
absolute path). With a source build, use absolute paths and set `PERCI_HOME`
to the checkout (weights, `config/policy.json`).

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

`scripts/bench_tools.py` (n = 200 per tool, verdict cache off for the work
numbers, every call recorded in the run ledger). Round trip = harness writes
the request → reads the reply over MCP stdio; work = the tool's own
`elapsed_us`. The ledger rows were measured once the bench run held 3,457
entries; the small-run column is a 190-entry run (`next_step` and the
sequence check scale with run length).

| tool | verdict in bench | round trip p50 | p95 | work p50 |
|---|---|---|---|---|
| exact (compute) | pass | 0.057 ms | 0.075 ms | 5 µs |
| exact (check claim) | fail | 0.060 ms | 0.070 ms | 7 µs |
| check_math | fail | 0.101 ms | 0.123 ms | 22 µs |
| relation | pass | 0.211 ms | 0.252 ms | 117 µs |
| route (v0.2, default) | abstain (low calibrated confidence) | 0.63 ms | 0.80 ms | 483 µs |
| route (v0.3) | pass | 2.03 ms | 2.73 ms | 1.78 ms (was 7.65 ms) |
| policy_check | fail | 0.085 ms | 0.114 ms | 7–11 µs |
| recall, AgentSearch (201 notes, warm) | pass | 1.54 ms | 2.13 ms | 1.28 ms |
| remember | pass | 0.273 ms | 0.491 ms | 197 µs |
| verify (math + relation + policy) | fail | 0.307 ms | 0.483 ms | 192 µs |
| status | pass | 0.450 ms | 0.603 ms | 257 µs |
| agent_check_claims | fail | 0.142 ms | 0.199 ms | 57 µs |
| agent_gate_step pre (`git push`, sequence rule) | abstain | 0.68 ms | 0.85 ms | 541 µs (38 µs at 190 entries) |
| agent_gate_step post | pass | 0.099 ms | 0.119 ms | 15 µs |
| ledger_append (append is in the round trip) | pass | 0.077 ms | 0.086 ms | 1 µs |
| next_step (run history) | pass | 0.33 ms | 0.51 ms | 260 µs (11 µs at 190 entries) |
| next_step (planned push) | pass | 0.76 ms | 1.03 ms | 640 µs (19 µs at 190 entries) |
| ledger_verify / ledger_show (re-hash from disk) | pass | 19–20 ms | 22–33 ms | ~5.5 µs per entry (1.1 ms at 190) |
| exact / route v0.3 / policy_check, cache hit | pass / pass / fail | 0.049–0.066 ms | 0.064–0.105 ms | 3–5 µs |
| agent_verify_patch (python verifier) | pass | 185 ms | 186 ms | 185 ms |

Route speed (`cargo run --release --example route_bench`, 381 prompts × 3,
classifier only): v0.3 p50 17.6 → 2.9 ms (p95 32.2 → 4.2 ms), v0.2 p50 0.67 →
0.31 ms, with an identical output fingerprint (the old scan is kept as
`PERCI_SCAN=reference` and a test asserts equality). The gain is the domain
scan running on up to 8 threads (`PERCI_SCAN_THREADS`) with allocation-free
candidates and a popcnt kernel; single-threaded v0.3 is ~12 ms.

Other: MCP `ping` 16 µs; cold start → `tools/list` 3.2 ms (78 ms in the bench run above, which includes the profile's pack discovery and AgentSearch); one-shot `perci tool
exact` process 1.1–23 ms; chat daemon TCP `ping` 18 µs (was 44 ms before
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
