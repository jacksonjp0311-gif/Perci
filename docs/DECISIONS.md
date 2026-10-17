# Decisions: Perci as a tool (Track A)

Open questions from `PERCI_AS_TOOL_PLAN.md`, resolved while building phases
T0–T3, T7 and T8 (Oct 17, 2026). Track B's decisions (CI, bug2eval cases,
cited memory) are in [`DECISIONS-evals-memory.md`](DECISIONS-evals-memory.md).
Each entry: what was chosen, why, what it costs.

## Structure

1. **One crate, enforced seam; no workspace split yet.** Tools live in
   `src/tools/` and `src/verdict.rs`; `tests/tool_boundary.rs` fails if they
   use `chat`, `voice`, `deliberation` or `session`. Splitting into
   `perci-core`/`perci-tools` crates would have touched every module and
   collided with Track B for no runtime gain. Cost: the core still compiles
   with the whole app; the split is a mechanical follow-up.
2. **Dependencies unchanged** (memmap2, serde, serde_json). MCP, canonical JSON
   and SHA-256 use what the repo already has.

## Wire format

3. **`perci.verdict.v1`**: status `pass|fail|abstain|error`, confidence
   `exact|bounded|heuristic`, receipts hash canonical JSON (sorted keys, floats
   as strings). Timing, config and weights identity are recorded, not hashed, so
   the same input gives the same `output_sha256` on any machine and run.
4. **Exit codes** for one-shot `perci tool`: 0 pass, 1 fail, 4 abstain, 3 error,
   2 usage (2 is the usual CLI usage code).
5. **Wire names drop `perci.` and use `_`** (`agent_verify_patch`): OpenAI and
   Anthropic tool names allow only `[A-Za-z0-9_-]`. Dotted names still work in
   `perci tool`.
6. **MCP**: line-delimited JSON-RPC 2.0 on stdio; protocol versions
   2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05; `outputSchema` = verdict
   schema; result in `structuredContent` plus a text summary; 256 KiB message
   cap; logs only on stderr. No HTTP transport yet (T9).
7. **First customer is generic MCP** (Claude Code, Codex, Cursor configs in
   `docs/TOOLS.md`) rather than one vendor's SDK.

## Semantics

8. **Ambiguous notation abstains.** `15/24 / 16/13` and `1/2/3` read differently
   by people and by precedence rules, so `exact` returns `abstain` / `AMBIGUOUS`
   and asks for parentheses. `a/b * c/d` is unambiguous and is evaluated. Found by
   the held-out set (8 items, all "right by precedence" but not what the
   question meant).
9. **Rounding tolerance.** A claim is accepted as rounding only when it is
   honest rounding: ≥ 2 decimals for terminating values, ≥ 1 for repeating ones,
   or an explicit approximation word (≈, about, roughly). "41" for 40.8 is wrong.
10. **Policy is JSON (`config/policy.json`), not TOML**: no TOML dependency.
    Load order `PERCI_POLICY`, `PERCI_HOME/config/policy.json`, embedded copy.
11. **Posture mapping**: allow → `pass`, propose_and_verify → `abstain`
    (`NEEDS_APPROVAL`: the harness asks the user), refuse → `fail`. The keyword
    heuristic can only make a posture stricter, never looser.
12. **Default policy** (`config/policy.json`): refuse force-push, `--no-verify`,
    recursive delete of root/home, disk formatting, raw device writes, power off,
    pipe-to-shell, encoded PowerShell, writes to `.git`, weights, keys and `.env`,
    and anything outside the workspace. Ask before push, history rewrites, `rm`,
    `mv`, privilege, network (except localhost), package installs, `--amend`.
    Allow reads, builds and tests, plain commits, and writes under `src/`,
    `tests/`, `docs/`, `scripts/`, `evals/`. Diff budget 400 changed lines.
13. **Routing uses the v0.2 pack by default** (0.76 ms vs 7.96 ms for v0.3);
    `PERCI_TOOL_PACK` or `args.pack` selects another. v0.3 hot-loop work (P4/P5)
    is not done.
14. **Read-only by default.** `remember`, `agent_verify_patch` and `eval` are
    off unless `PERCI_TOOLS` enables them.

## Memory and evals (wired to Track B)

15. **recall / remember go through `CitedMemory`** (`open_default`, `recall`,
    `remember`, `verify`, `sync`, `warm_up`). The tool embeds the
    `perci.recall.v1` document minus `elapsed_us`, so the output hash stays
    stable. Verdict evidence cites the journal record (`memory:N` +
    `record_sha256`), the source of truth; `verify` accepts that, a plain
    file/line hash, or the full recall citation (re-checked against note *and*
    journal).
16. **`remember` appends to the journal** (via `CitedMemory::remember`) instead
    of the earlier staging file, so a remembered note is immediately recallable
    and cited. Safety is: off by default, secrets refused, 1200-char cap. Cost:
    no human review step before a note lands. Re-add staging if agents are
    allowed to write memory unattended.
17. **Recall writes derived files.** `CitedMemory` syncs one note file per
    journal record and an AgentSearch index next to the journal; `recall` is
    still annotated read-only because the journal is never changed. `perci mcp`
    warms this in the background at start (`PERCI_MCP_NO_PREWARM=1` disables).
18. **`eval` goes through `perci::b2e`** (`validate_case`, `run_case`,
    `validate_all`). `run` executes an agent command, so it needs the operator's
    `PERCI_EVAL_ALLOW_AGENT=1`; a tool argument would let the calling agent grant
    itself code execution. bug2eval `invalid`/`error` map to verdict `error`.
19. **Python child processes are fine** (AgentSearch, bug2eval): both are
    optional and degrade to a clear verdict when missing.

## Verification and CI

20. **Held-out set is generated, independent, but written by me.**
    `evals/tools-heldout-v1` uses a `fractions.Fraction` oracle that shares no
    code with Perci and a fixed seed. James's hand-written items are still
    needed before anyone calls it a benchmark.
21. **CI**: Track B's `ci.yml` already runs fmt, the clippy ratchet and tests
    (no LFS on PRs; real weights on main/nightly). I added a `tools` job: release
    build without LFS, held-out gate, demo harness, MCP bench smoke. It needs no
    weights, so it protects the LFS bandwidth quota.
22. **`agent_verify_patch`** applies the diff in a `git worktree add --detach`
    scratch tree, runs only allowlisted verifiers (cargo, python, pytest,
    bug2eval) with a ≤ 600 s timeout and `CARGO_NET_OFFLINE=1`, and proves the
    caller's tree is unchanged with `git status` before/after.
