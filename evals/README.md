# Perci evals

## bug2eval regression cases (`evals/b2e/`)

Every fixed Perci bug is a [bug2eval](https://github.com/jacksonjp0311-gif/bug2eval)
case: a broken snapshot, a fixed snapshot and a verifier. A case is **valid** when
its verifier fails on the broken snapshot and passes on the fixed one.

| Case | Bug | Verifier (black-box unless noted) |
|---|---|---|
| B2E-PERCI-001 | π/√ mojibake in exact geometry | `ask` circle area → `9π`, no `Ï€` (also with `PERCI_ASCII=1`) |
| B2E-PERCI-002 | agent `truncate` panics on a UTF-8 boundary | white-box: test appended to a scratch copy of `src/agent.rs` |
| B2E-PERCI-003 | "not answering" technical questions routed as Feedback | `perci::voice::detect_dialogue_act` on 6 technical prompts + 3 real complaints |
| B2E-PERCI-004 | weight tests pass vacuously without weights | `PERCI_REQUIRE_WEIGHTS=1 cargo test` must fail with "weights required" |
| B2E-PERCI-005 | LFS pointer weights → cryptic error | `perci ask hi` exits 2 and says `git lfs pull` |
| B2E-PERCI-006 | writes `memory/`/`models/` into the current folder | run from an empty temp folder; nothing may appear |
| B2E-PERCI-007 | curly apostrophes route differently | `normalize_for_routing`, `detect_social`, `detect_dialogue_act` |
| B2E-PERCI-008 | internal scaffold hints leak into answers | 5 open questions on the no-weights path; no `[Perci dialogue workspace` |
| B2E-PERCI-009 | daemon buffers an unbounded line | stream 32 MiB without a newline; daemon must hang up early and keep serving |
| B2E-PERCI-010 | `OPENAI_API_KEY` sent over http to a non-loopback host | fake LM on this machine's LAN address; the canary key must never arrive |
| B2E-PERCI-011 | recall citations must match the note files | `perci memory` remember → recall → sha256/offset checks → edited note detected |

003 was never committed to `main` (the broad patterns lived in an uncommitted local
diff); its broken snapshot is `main` plus `patches/b2e_003_broad_not_answering.patch`.
011 guards a new capability, so its "before" fails because `perci memory` did not exist.

Snapshots are pruned source trees (`Cargo.*`, `build.rs`, `src/`, `tests/`, `config/`,
`knowledge/`, the package manifest) with **no weights**; 005 carries the LFS pointer
files on purpose. Verifiers build Perci themselves (Rust + Python 3.10+ needed).

```bash
pip install "git+https://github.com/jacksonjp0311-gif/bug2eval.git@v0.1.3"
export PERCI_B2E_TARGET=/tmp/perci-b2e-target       # shared cargo target: much faster
python evals/run_b2e.py                             # validate all, write evals/receipts/b2e-*.json
perci eval validate B2E-PERCI-001 --json            # same from the Perci binary
python evals/build_cases.py --all                   # rebuild cases from evals/cases.json
```

Windows (ROG): `py -3.12 -m pip install …`, then `$env:PERCI_B2E_TARGET="$env:LOCALAPPDATA\Perci\b2e-target"`.
Each case takes ~30–60 s on the box (two debug builds plus the checks).

`evals/receipts/baseline-2026-10-17.json` is the first full run. Other receipts are
git-ignored; CI uploads its own as an artifact.

**Security:** a verifier executes code. Only run cases from this repository.
`perci::b2e` refuses cases outside `PERCI_HOME/evals/b2e` (plus `PERCI_B2E_CASES`).

## Adding a case

1. Write `evals/verifiers/b2e_NNN_*.py` (import `_perci_b2e`; exit 0 only when the bug is absent).
2. Add the spec to `evals/cases.json` (before/after refs; `grep:<commit subject>` works for refs that move on rebase).
3. `python evals/build_cases.py B2E-PERCI-NNN` (captures, validates, packs).
