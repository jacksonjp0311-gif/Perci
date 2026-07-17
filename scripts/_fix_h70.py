from pathlib import Path

p = Path("training/hardness/hardness-pack-v1.jsonl")
new = (
    '{"id":"H70","capability":"governed_learning_loop","hardness":2,'
    '"prompt":"How should weak signals age after a primary-fix ticket is closed?",'
    '"required_any":[["age","closed","ticket","chronic","operator","curriculum"]],'
    '"forbidden":["silently promote weights","self-promote pack"],'
    '"notes":"ledger aging"}'
)
lines = []
for line in p.read_text(encoding="utf-8").splitlines():
    if '"id":"H70"' in line or '"id": "H70"' in line:
        lines.append(new)
        print("replaced H70")
    else:
        lines.append(line)
p.write_text("\n".join(lines) + "\n", encoding="utf-8")
