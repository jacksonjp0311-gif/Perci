# Changelog

All notable changes to Perci's tool layer. Versions follow `Cargo.toml`.
0.x releases are previews: interfaces may still change, and changes that
affect harnesses are listed here. The research engine's history is in
[docs/RESEARCH_LOG.md](docs/RESEARCH_LOG.md).

## 0.11.8 (preview) - 2026-10-18

Pre-share fixes found by running Perci inside a live agent harness.

### Fixed

- `perci tool NAME --json` (and `perci tool NAME` with nothing piped) no
  longer waits on stdin. A bare `--json`, a terminal, an empty stdin, or a pipe
  that stays silent for `PERCI_STDIN_WAIT_MS` (1500 ms) mean `{}`. `--json -`
  reads stdin to EOF explicitly.
- Receipts: verdicts now carry `receipt.receipt_sha256`, an id over tool,
  input hash, output hash, version, build and weight/config identity. Two
  different inputs with the same output (two allowed reads) used to be told
  apart only by `input_sha256`; harnesses that used `output_sha256` as the
  receipt conflated them. The run ledger records the new id. The field is
  optional, so `perci.verdict.v1` stays compatible.
- `agent_gate_step`: status, `result.posture` and `result.summary` now always
  agree. A release before tests read `FAIL: propose_and_verify (git.history)`;
  it now reads `FAIL: refuse (seq.deploy_requires_tests)`, with the deciding
  reason first and the policy's own posture kept as `result.policy_posture`.
- Less noise, no less safety:
  - a step the policy already refuses (`git push --force`) no longer also
    gets a `SEQUENCE_REQUIRES` / `SEQUENCE_UNCHECKED` reason (the rule is
    still listed in `result.sequence`, marked `superseded`);
  - a delete whose literal targets Perci has just checked and found absent
    is allowed (`DELETE_TARGETS_ABSENT`) and skips the backup-before-delete
    rule (`skip_if_targets_absent`). Refusals, protected paths, globs,
    variables, pipelines and wrappers are never lifted.
- `exact` accepts a numeric `claimed` (MCP hosts and the inspector send
  `41`, not `"41"`).
- The verdict JSON Schema's evidence `kind` was a closed enum that missed
  kinds Perci emits (`ledger_entry`, `ledger_head`, `temporal_model`,
  `memory_record`); it is now an open string, so schema-validating MCP
  clients accept every verdict.

### Added

- **Temporal Router** naming for `next_step` across the docs, CLI help and
  MCP description, and the alias `temporal_route` (`perci tool temporal_route`
  or a raw MCP `tools/call`; `tools/list` still shows `next_step`).
- `perci --help` starts with the tool layer commands.
- Host test status in QUICKSTART: Claude Code, Cursor CLI, Codex CLI and the
  MCP Inspector CLI were run against Perci; Claude Desktop is validated only.
- Verifying downloads (SHA256SUMS) and what the Gatekeeper / SmartScreen
  warnings on unsigned preview builds look like.
- CONTRIBUTING.md, SECURITY.md, issue templates, CHANGELOG.md, docs index.

### Changed

- Releases of 0.x versions (and tags with a `-suffix`) are published as
  GitHub prereleases ("preview"). The installers' default "latest" now means
  the newest release including previews.
- The README is now about the tool layer; the research engine's long-form
  history moved to docs/RESEARCH_LOG.md.
- Personal absolute paths were removed from committed evaluation receipts and
  tests (they now read as repository-relative paths).

## 0.11.7 - 2026-10-17

First packaged release candidate of the tool layer: MCP server and CLI,
`perci.verdict.v1`, hash-chained run ledger with sequence rules, `next_step`,
profiles and weight packs, release workflow with SHA256SUMS and installers,
`perci fetch-weights`. Not published as a GitHub release.
