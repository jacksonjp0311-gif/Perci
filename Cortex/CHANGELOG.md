# Changelog

## 2.0.0 — Outcome-Grounded Repository Intelligence

- Made neural activation observational: association weights no longer change merely because paths co-activate.
- Added `cortex outcome` for explicit verified, helpful, irrelevant, failed, and unsafe outcome recording.
- Added bounded verification-weighted credit assignment, replay gates, immutable outcome ledger events, and before/after graph hashes.
- Added outcome and evidence-credit records to Cortex's single SQLite substrate.
- Added the agent-neutral `cortex-context/1.0` protocol via `cortex protocol`.
- Preserved the authority boundary: current source, tests, governance, and human authorization outrank learned associations.

## 1.3.1 — Trust-State Closure

- Activation, context, Governor, and health now consume the same current certificate.
- Current sessions are created after trust evaluation, preventing a new session from inflating continuity.
- Semantic scan configuration now controls retrieval candidate limits.
- Added an explicit Phoenix privacy-boundary policy; no Phoenix adapter is enabled.

## 1.3.0

- Added bounded self-host validation: Cortex can clone itself as an outer host, run a nested cloned engine, and verify the engine is excluded from host assimilation.
- Added full lifecycle before/after benchmark support for host-engine and nested-engine bootstrapping and activation.
- Added lane-relevance pruning with a bounded fallback, so uncertain routes cannot silently produce empty context packets.
- Added a polished README hero and verification, routing, local-first, and authority badges.

## 1.2.0

- Added the root-level deterministic Thalamus request-routing package, including intent classification, memory-lane budgeting, and auditable inhibitory evidence gating.
- Routed normal activation, public query, and neural-interlink CLI flows through Thalamus without changing Cortex's authority boundary.
- Blocked Git telemetry when the requested target is only nested within an ancestor worktree.
- Replaced the Windows-incompatible Bash-wrapper test with platform-specific wrapper execution and added Thalamus and telemetry-boundary coverage.

## 1.1.0 — 2026-07-11

- Integrated the standalone Neuron concepts directly into Cortex as `cortex.neuron`.
- Preserved Cortex as the sole repository-memory, episodic-memory, consolidation, and authority substrate.
- Added deterministic sparse spreading activation over existing repository relationships.
- Added bounded Hebbian association strengthening without autonomous topology rewriting.
- Added hash-chained neural event replay in the same SQLite database.
- Added environment learning for languages, manifests, ecosystems, frameworks, commands, entrypoints, CI, and runtime capabilities.
- Added neural support-path expansion to bounded context packets.
- Added environment and neural interlink sections to NexusGate packets.
- Added portable one-command PowerShell and Bash installation/bootstrap/verification/activation flows.
- Added automatic exclusion when the Cortex engine folder is dropped inside a host repository.
- Bound generated repository wrappers to the bootstrap-recorded Cortex home and Python engine so inherited environment variables cannot silently redirect repository memory or execution.
- Expanded the suite from 10 to 17 tests while retaining all original compatibility tests.

## 1.0.1 — 2026-07-11

- Excluded generated repository-local Cortex launcher scripts from assimilated memory to reduce retrieval noise.
- Revalidated CLI bootstrap, repository-local Bash activation, compile checks, and the 10-test integration suite.

## 1.0.0 — 2026-07-11

- Rebuilt Cortex around verified repository assimilation.
- Added repository-local agent integration and portable context packets.
- Added file inventory, unsupported-surface reporting, incremental indexing, and manifest hashes.
- Added semantic, structural, temporal, and episodic memory layers.
- Added Python and multi-language relationship extraction.
- Added Git commit, churn, and co-change telemetry.
- Added retrieval probes and bootstrap certificates.
- Added automatic activation refresh and Governor read-only fallback.
