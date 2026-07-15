# Lumen → Perci training stage

Approved Lumen learning receipts land here as JSONL.

## Governance

1. Review and redact secrets.
2. Deduplicate and hold out evaluation samples.
3. Only then fold curated examples into Perci's curriculum builder.
4. Rebuild weights from the Perci tree:

```powershell
cd $env:PERCI_HOME
python .\scripts\build_weights.py --output .\models\perci-cognitive-v0.1.pwgt
python .\scripts\verify_weights.py
```

Lumen never writes `.pwgt` bytes directly.
