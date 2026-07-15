# Perci training

Perci v0.1 includes a fully generated 200 MiB binary associative weight pack.

## Reproducible build

```powershell
python .\scripts\build_weights.py --output .\models\perci-cognitive-v0.1.pwgt
```

The builder uses a fixed seed and produces:

- 403,266 binary prototypes
- 4,096-bit activations
- 16 expert partitions
- learned positive expert masks
- an exact 200 MiB output file
- a JSON manifest containing checksums and limitations

## Evaluation

```powershell
python -m pip install numpy
python .\scripts\test_weights.py
```

The included `heldout-evaluation.txt` records the build-time probe results. This test measures domain routing and prototype retrieval only.

## Future language-model track

The current weights are not a transformer. A future release may attach a legally usable compact pretrained decoder through `PERCI_MODEL_CMD`, then distill Perci-specific tool usage and governance behavior. Raw private conversation exports should not be trained directly; they must first be reviewed, redacted, deduplicated, separated into canonical versus speculative content, and held out from evaluation where appropriate.
