# Contributing to Perci

Thanks for helping. Perci is a preview; bug reports from real harnesses are
the most useful contribution right now.

## Reporting a problem

Open an issue with the bug template. Please include `perci --version`, your
OS, the host (Claude Code, Cursor, Codex, your own harness), and, when a
verdict looks wrong, the exact tool arguments and the verdict JSON (it is
safe to share: it contains hashes, not your files). For a policy verdict,
say what you expected (allow / needs approval / refuse) and why.

Security problems: please do not open a public issue; see
[SECURITY.md](SECURITY.md).

## Development setup

```sh
git clone https://github.com/jacksonjp0311-gif/Perci && cd Perci
git lfs pull                      # weights (only route and the chat engine need them)
cargo build --release             # Rust 1.75+
cargo test --lib --tests          # weight tests skip without LFS weights
```

Useful gates (CI runs all of them):

```sh
cargo fmt --all -- --check
python3 scripts/clippy_ratchet.py --base origin/main     # no new clippy warnings
python3 evals/tools-heldout-v1/run_heldout.py             # exact/check_math: 0 wrong passes
python3 scripts/bench_public.py --perci target/release/perci --n 100 --iters 30
python3 evals/run_b2e.py                                  # bug2eval regression cases (needs bug2eval)
```

## Pull requests

- Keep changes focused, with a test that fails before and passes after.
  A bug in a verdict usually belongs in `tests/` (through the `perci`
  binary) or next to the tool in `src/tools/`.
- Never loosen a safety verdict to make a test pass. A change that turns a
  `refuse` into anything else needs a clear argument in the PR and in
  `docs/DECISIONS.md`.
- Determinism matters: the same input must give the same `output_sha256`
  on every platform. No floats in hashed output, no wall-clock values in
  hashed fields.
- `src/tools/` must not depend on the chat engine (`tests/tool_boundary.rs`).
- Update `docs/TOOLS.md` / `docs/QUICKSTART.md` when behavior visible to a
  harness changes, and add a line to `CHANGELOG.md`.
- No new clippy warnings (the ratchet in CI compares against the base commit).

## License

By contributing you agree that your contribution is licensed under MIT OR
Apache-2.0, like the rest of the project.
