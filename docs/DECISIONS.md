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
13. **Routing uses the v0.2 pack by default** (0.48 ms vs 1.8 ms for v0.3
    after the scan work below); `PERCI_TOOL_PACK`, the profile's `tool_pack`
    or `args.pack` (a pack *name*, never a path) selects another.
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

## Packs, profiles, ledger, temporal router (second workstream)

23. **Profile = an external directory**, `PERCI_PROFILE`, defaulting to
    `PERCI_DATA` so existing installs change nothing. Policy overlay, sequence
    rules, packs, memory journal, ledgers and the temporal model resolve from
    it; `profile.json` (`perci.profile.v1`) is optional and only names files.
    A file named there but missing, or present but invalid, is an error; the
    repo ships generic defaults only (docs/PROFILES.md). `status` warns when the
    profile sits inside a git checkout.
24. **Weight packs are verified at every load.** Manifest (`perci.weight-pack.v1`,
    a `perci.packs.v1` index, or a `perci.pack-manifest.v1` file) → safe paths,
    hash of every file (the entry hashed from the same mapped bytes that
    classification reads), labels = categories. Missing, modified or mispinned
    = `PACK_UNAVAILABLE`; no fallback to another pack, ever. Built-ins are
    pinned by the committed `models/PACKAGE_MANIFEST.json`. External packs are
    pinned in `profile.json` (`pins`, optional `require_pins`); signatures are
    not supported (no crypto dependency) and a `signature` field is reported
    as unverified. `args.pack` no longer accepts raw paths.
25. **`PERCI_WEIGHT_PACKS`, not `PERCI_PACKS`**: the latter already names the
    knowledge-note root (`intel_packs`).
26. **Hashing cost**: verification hashes the whole pack, so SHA-256 got a
    runtime-detected SHA-NI path (v0.3: 1.02 s → 0.27 s; v0.2 ≈ 25 ms), and
    `perci mcp` verifies the default pack beside the request loop so
    `initialize`/`tools/list` answer at once. No mtime-based skip: that would
    let a same-size, same-mtime edit through.
27. **Lexical priors are per pack.** The built-in keyword table (48 points per
    cue) is tied to the built-in label names; it is `off` by default for every
    other pack, or a pack supplies its own table. Otherwise a private label
    that happens to be named `memory` gets boosted.
28. **Route confidence is calibrated, not the margin.** On 158 held-out cases
    v0.2 is 64.5% accurate and v0.3 84.1%, and accuracy is not monotonic in
    the margin (v0.2's 4–8 bucket is worse than its 2–4 bucket; wrong labels
    up to margin 24). `perci packs calibrate` stores per-bucket counts; the
    gate uses the Wilson 95% lower bound (default 700‰). An `llm` route below
    it abstains with `ROUTE_LOW_CONFIDENCE` (the deterministic routes are
    unaffected). Consequence: with v0.2, most `llm` routes now abstain; that is
    the honest reading of a 64% label. Uncalibrated packs report
    `uncalibrated` and gate only on an explicit `min_margin`.
29. **Ledger**: one hash-chained JSONL per run, `hash = sha256(canonical entry
    without hash)`, genesis `sha256("perci.ledger.v1|<run>")`. Detects edits,
    reordering, drops and moves; truncation of the tail only against
    `expect_head`. A broken ledger is never appended to. MCP records every
    verdict except the read-only meta tools; one-shot CLI only with
    `PERCI_RUN_ID`. Single writer per run (no file lock).
30. **Sequence rules live in the policy** (`sequence.classes` + `rules`) or in
    the profile's `sequence-rules.json` (`perci.sequence.v1`), and only
    tighten. Tests count only as `phase: post` with status pass *after the last
    code change* (a pre-gate is not a test run). Without a ledger the rule is
    `SEQUENCE_UNCHECKED` (abstain), never silently passed. A sequence file in
    another schema makes gated steps abstain (`SEQUENCE_RULES_INVALID`) rather
    than dropping the rules or failing the whole policy.
31. **`policy_check` stays stateless**; only `agent_gate_step` consults the
    ledger. That keeps `policy_check` cacheable and replayable.
32. **Temporal router = integer n-gram** (order 3, back-off, ties by token),
    learned only from verified ledgers; the current run is a live model
    updated on each append. Output is counts (`count`, `of`, `permille`,
    per-step outcomes), so every suggestion is explainable. Marked
    `deterministic: false` because it depends on history.
33. **Verdict cache** for exact, check_math, relation, route, policy_check:
    key = input hash + output config, FIFO 4096, errors not cached, `cache_hit`
    in the receipt (unhashed), `PERCI_TOOL_CACHE=0` disables. Profile edits
    during a server's life are not seen by cached verdicts.
34. **Scan speed with identical results**: allocation-free integer candidates,
    exact pruning bound, popcnt kernel, domain scan on up to 8 threads. The old
    loop stays as `PERCI_SCAN=reference`; a test and the `route_bench`
    fingerprint assert identical output. A smaller "hot index" was tried and
    gave nothing; pruning fires on only ~3.5% of records, so threads and
    removing per-record allocation are the gain.
35. **PowerShell**: `&` / `.` are unwrapped, built-in aliases map one way to
    their cmdlet (a `remove-item` rule catches `del`, but `rm` rules keep Unix
    meaning), and long parameters match their abbreviations (`-rec`,
    `-Recurse:$true`; `:$false` does not count). `-Force` no longer reads as
    the flag cluster `-F -o -r -c -e`.
36. **`check_math` reads `x` between digits as ×** (`3 x 42`, `3x42`, `(3)x(4)`).
37. **`perci tool schema NAME`** prints that tool's input schema (it printed the
    verdict schema); `perci tool schema verdict` prints the verdict schema.
38. **Weights are separate release assets, fetched by `perci fetch-weights`,
    not bundled into the binary archives.** The default set is 240 MB (v0.3
    210 MB, v0.2 20 MB, language/dialogue 10 MB; the legacy v0.1 pack is
    left out). Bundling it would make four archives of about 240 MB each, 1 GB
    per release, and every user would download 240 MB even when they only
    want the tool layer, which needs no weights (`route` needs the 20 MB v0.2
    pack). Published once as assets, the weights are served from GitHub
    release storage, which has no Git LFS bandwidth quota. The release job
    reads them from the CI cache keyed by LFS object ids, so a release pulls
    nothing extra from LFS. `fetch-weights` checks each file's size and
    sha256 against `models/PACKAGE_MANIFEST.json`, which ships in the
    archive, before moving it into place. It uses the system `curl` (Windows
    10+, macOS, Linux), so the crate gains no HTTP or TLS dependency. Sets:
    `tools`, `default`, `all`. Source: `--from`, `PERCI_WEIGHTS_URL`
    (mirrors, air-gapped dirs), or this version's release.
39. **Archive = a complete Perci home minus the weights** (`perci/bin`,
    `models/` manifests, `config/`, `knowledge/`, docs, the public benchmark).
    Home resolution already looks two levels above the executable, so an
    unpacked or installed copy needs no `PERCI_HOME`. Asset names carry no
    version, so `releases/latest/download/<asset>` works for the installers.
40. **Linux binary is static musl** (runs on any x64 distribution). macOS ships
    native arm64 and x64 builds (an Intel runner builds and smoke-tests x64),
    and Windows ships MSVC x64. The release workflow's `workflow_dispatch`
    is a dry run: the same builds plus an end-to-end check that serves the
    assets on localhost and runs the real `install.sh` / `install.ps1`
    (`irm | iex` under Windows PowerShell 5.1), `perci doctor --deep`, a chat
    answer and the public benchmark. Only a `v*` tag publishes, after a
    check that the tag matches `Cargo.toml`.
41. **Installers verify before they install**: the archive's sha256 must match
    the release's `SHA256SUMS`, and the weights must match the manifest
    shipped inside that archive. The installers install per user (no sudo or
    admin), re-run as an in-place upgrade that keeps verified weights, and
    finish with `perci doctor` (`--tools` for a tools-only install).
    `perci doctor --tools` treats missing weights as a warning. A present but
    corrupt file is still a failure.
42. **Installer options use their own names** (`PERCI_INSTALL_WEIGHTS`,
    `PERCI_INSTALL_DIR`, `PERCI_RELEASE_URL`, `PERCI_VERSION`,
    `PERCI_NO_MODIFY_PATH`). `PERCI_WEIGHTS` already selects an alternate
    chat pack at runtime, so an installer setting it in the user's session
    would redirect Perci.
