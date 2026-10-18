# Bitwork baseline evaluation

Reproduces Tables 1 and 2 of the [Bitwork white paper](../../docs/BITWORK_WHITEPAPER.md)
(section 10): Bitwork's `route` label on the 158 held-out calibration cases versus
TF-IDF + logistic regression, TF-IDF 1-NN, BM25 1-NN and the keyword router, all
trained on the regenerated v0.3 training corpus (hash-checked against the pack
manifest), plus a keyword-priors ablation, a prior/held-out overlap check, latency
and a cross-process determinism check.

```sh
git lfs pull --include="models/*"
cargo build --release
python3 -m venv .venv && .venv/bin/pip install scikit-learn numpy scipy
.venv/bin/python evals/bitwork_baseline/bitwork_baseline.py \
  --perci target/release/perci --out evals/bitwork_baseline/results.json
```

About 5 minutes on 8 cores (most of it training the logistic regression).
`results.json` is the committed run used in the paper (Perci 0.11.8 at `8004170`,
Linux x86-64, 8 vCPUs). Accuracies are deterministic; latencies depend on the machine.
The held-out sets are small and written by the project author; read the paper's
caveats before quoting the numbers.
