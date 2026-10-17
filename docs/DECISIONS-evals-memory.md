# Decisions: CI, bug2eval cases and AgentSearch-backed cited memory

Track B of the tool-layer build (Oct 17, 2026). Each decision lists what was chosen,
why, and what it costs.

## CI (`.github/workflows/ci.yml`)

1. **Clippy is a ratchet, not `-D warnings`.** The tree has ~95 pre-existing clippy
   warnings, so `-D warnings` would be red forever or need a large unrelated cleanup.
   `scripts/clippy_ratchet.py` builds HEAD and the base commit with the *same*
   toolchain and fails if any (lint, file) count goes up. Line numbers are ignored so
   moving code does not trip it. Comparing against a committed baseline file was
   rejected: lint sets change between toolchains, so a baseline written on the box
   would break on CI's `stable`.
2. **Weights: real tests on `main`, nightly and on demand, not on PRs.** The `test`
   job (Linux + Windows) runs without LFS; weight tests skip with a note. The
   `weights` job restores `models/*.{pwgt,blng,bphr,bdlg}` from an Actions cache keyed
   by the LFS object ids (`git lfs ls-files -l -I models/*`), pulls only on a cache
   miss, refuses pointer files, and runs with `PERCI_REQUIRE_WEIGHTS=1` so weight tests
   cannot pass vacuously (B2E-PERCI-004). It then does a release smoke run (exact
   math, `9π`, remember/recall with a hash check, no stray folders).
3. **AgentSearch is exercised for real in CI**, pinned to `v1.0.0-rc5`, with
   `PERCI_REQUIRE_AGENTSEARCH=1` so the AgentSearch test fails instead of skipping.
4. **bug2eval validation runs on `main`/nightly** (about 10 min: each case builds Perci
   twice), pinned to `v0.1.3`, with a shared cached target dir and the receipt uploaded.
5. `ci-ok` aggregates every job (skipped weights/evals on PRs count as OK) so a single
   required check can be used for branch protection later.

## bug2eval integration (`evals/`, `src/b2e.rs`)

6. **Snapshots are built with `--before-dir/--after-dir`, not `--before-ref`.**
   bug2eval requires the verifier to be byte-identical in both snapshots, but every
   fix landed before its verifier existed. `evals/build_cases.py` exports a pruned
   `git archive` of each ref (LFS smudging off, so no 200 MiB weights) and copies the
   current `evals/verifiers/` into both sides. Cases are ~1.1 MiB each (~12.5 MiB for
   all eleven), committed as plain git files, not LFS, so CI does not spend LFS bandwidth.
7. **Black-box verifiers where possible.** 001, 005, 006, 008, 009, 010 and 011 drive the
   `perci` binary. 002 needs a private function, so the verifier appends a test to a
   *scratch copy* of `src/agent.rs` (the workspace is never modified, so bug2eval's
   tamper check stays meaningful). 003 and 007 call public library functions from a
   generated integration test. 004 checks `cargo test` itself.
8. **Verifiers run in a clean environment**: inherited `PERCI_*` variables and
   `OPENAI_API_KEY` are dropped, `PERCI_DATA` is a fresh temp folder, `PERCI_HOME` is
   the snapshot. Only `PERCI_B2E_TARGET` (build cache) and, for 011, `PERCI_AGENTSEARCH`
   pass through.
9. **009 and 010 test the actual hazard, not a proxy.** 009 streams 32 MiB without a
   newline and requires the daemon to hang up while the client is still sending (the old
   daemon silently buffers everything). 010 runs a fake LM on the machine's non-loopback
   address and checks whether the canary API key arrives (the old code sent it twice).
10. **003 is reconstructed.** The over-broad patterns were in James's uncommitted local
    diff and never reached `main`; the broken snapshot is `main` + a patch with exactly
    those six patterns. 011 is a capability guard ("before" fails because `perci memory`
    did not exist), not a historical bug.
11. **`perci::b2e` is the entry point for `perci.eval`.** A subprocess wrapper, because
    bug2eval is Python. It returns a `perci.eval.v1` verdict with a receipt (case sha256,
    timing-free output sha256, bug2eval version, argv, build id). It only accepts cases
    under `PERCI_HOME/evals/b2e` or `PERCI_B2E_CASES`, and `run_case` (which executes an
    agent command) requires `allow_agent_run`. A missing bug2eval is `error`, never `pass`.

## Cited memory (`src/agentsearch.rs`, `src/cited_memory.rs`)

12. **The append-only journal stays the source of truth.** `memory/perci.jsonl` is
    unchanged. Notes under `memory/notes/` are *derived* (one immutable file per record)
    and can be deleted at any time; they are rebuilt from the journal. AgentSearch hashes
    whole files, so one-file-per-record keeps every earlier citation's sha256 stable while
    the journal keeps growing (indexing the JSONL directly would change its hash on every
    append).
13. **Citations carry both sides**: note path, line and sha256, plus journal path, line,
    byte offset and the sha256 of the raw journal line. `verify` checks that the note is
    unchanged, that the journal still holds that record at that offset, and that the note
    still says what the journal says. A hand-edited note is never overwritten, so the
    evidence of the edit survives.
14. **AgentSearch over its JSON-lines stdio, not vendored.** One long-lived child
    (`python -m agentsearch --config … --db … stdio`), spawned on the first recall (a chat
    that never recalls never starts Python), a 20 s deadline per request, killed on drop.
    `PERCI_AGENTSEARCH` sets the command (`py -3.12 -m agentsearch` on the ROG; `off`
    disables it). Index and config live next to the journal in `memory/agentsearch/`,
    inside `PERCI_DATA` (outside OneDrive and the repo).
15. **Literal search, honestly.** AgentSearch has no semantic search. Recall runs one
    literal grep per query term (at most 4, longest first, filler words dropped) and ranks
    notes by distinct terms hit, then recency. "What did I say about my appointment" will
    not find "dentist". This is stated, not hidden.
16. **Newest-first note names** (`<9999999999-line>-L<line>-<kind>.md`). AgentSearch
    returns matches in path order and each returned file costs a reread and a hash
    (~0.2 ms). With inverted names, a small per-term window (3 × limit, 8..60 files) holds
    the newest matches, which cut warm recall from ~47–89 ms to ~8–10 ms. Older matches
    beyond the window are reported as `truncated`, not as missing.
17. **Fallback keeps citations.** If Python, the package or FTS5 is missing, or the child
    dies mid-session, recall scans the journal in Rust and still returns hashed citations,
    with `backend: "substring"` and a warning. It is faster (0.05–0.5 ms warm) but does not
    scale to indexing other folders (docs, projects), which is why AgentSearch stays the
    primary backend.
18. **Interfaces for Track A.** `CitedMemory::{open_default, remember, recall, verify,
    sync, warm_up, status-ish accessors}` with `perci.recall.v1` JSON (`RecallResult`,
    `Citation`, `RememberReceipt`, `VerifyResult`), plus `perci memory
    remember|recall|verify|status|sync [--json]` on the CLI. Chat recall answers now quote
    notes with a `note NNNNNN · sha256:…` tag.

## Not changed

- **bug2eval and AgentSearch themselves**: no change was needed. Both are used at their
  pinned tags through public CLIs.
