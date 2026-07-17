"""Append H81-H100 entity-swap / transfer hardness (UTF-8, idempotent)."""
from __future__ import annotations

import re
from pathlib import Path

PACK = Path(__file__).resolve().parents[1] / "training" / "hardness" / "hardness-pack-v1.jsonl"

CASES = []
# Entity-swap trust variants
for i, (a, b, c) in enumerate(
    [
        ("AetherBus", "HelixCore", "DriftGate"),
        ("NovaLink", "PrismNode", "LagShield"),
        ("KiteMesh", "OrbitAPI", "RetryVault"),
        ("FluxPort", "SigmaEdge", "TimeoutMap"),
        ("RuneWire", "CobaltHub", "IdemKey"),
    ],
    start=81,
):
    CASES.append(
        f'{{"id":"H{i}","capability":"transfer_vs_template","hardness":4,'
        f'"prompt":"how should {a} interfaces earn trust under {b} lag and {c} retry?",'
        f'"required_any":[["timeout","idempotent","retry","lag","earn","checkable","contract"]],'
        f'"forbidden":["behavioral complexity is observable","stuck is normal"],'
        f'"notes":"entity-swap trust H{i}"}}'
    )

# More coverage 86-100
extra = [
    ('H86', 'transfer_vs_template', 3, 'Plan measure ticket transfer close when HelixCore partitions',
     '[["measure","ticket","transfer","close","partition","lag"]]', 'agent loop partition'),
    ('H87', 'cross_domain_synthesis', 3, 'Compose logic falsifiers with systems contracts under AetherBus lag',
     '[["logic","falsif","contract","lag","system"]]', 'logic x systems'),
    ('H88', 'honest_abstention', 3, 'quoril blorf zephyr nembit - determine meaning if any',
     '[["unknown","cannot","invented","no confident"]]', 'OOD novel mash'),
    ('H89', 'relational_inquiry', 3, 'What is the boundary between knowledge and attention under load?',
     '[["knowledge","attention"]]', 'relational load'),
    ('H90', 'exact_tool_authority', 2, 'calculate 144 divided by 12',
     '[["12"]]', 'exact div'),
    ('H91', 'exact_tool_authority', 2, 'what is 17 times 3?',
     '[["51"]]', 'exact mul qmark'),
    ('H92', 'governed_learning_loop', 3, 'How do intelligence channels operators frames hardness transfer curriculum Cortex and lab patterns work?',
     '[["operator","hardness","curriculum","cortex","pattern"]]', 'five channels'),
    ('H93', 'transfer_vs_template', 3, 'what patterns emerge from the ledger?',
     '[["pattern","authority","transfer","ticket"]]', 'pattern intel'),
    ('H94', 'transfer_vs_template', 3, 'How should you calibrate confidence and when should you refuse for insufficient evidence?',
     '[["confidence","refuse","evidence","tier"]]', 'uncertainty'),
    ('H95', 'cross_domain_synthesis', 3, 'Connect sparse distributed memory, vector symbolic binding, and Bitwork in one coherent thought.',
     '[["sparse","memory","binding","bitwork"]]', 'SDM VSA Bitwork'),
    ('H96', 'exact_tool_authority', 3, 'Write a Rust function that reverses a string',
     '[["fn reverse","chars().rev","reverse_string"]]', 'code reverse'),
    ('H97', 'transfer_vs_template', 3, 'Is Perci a superintelligence?',
     '[["not a superintelligence","governed","superintelligence"]]', 'SI refuse'),
    ('H98', 'transfer_vs_template', 4, 'how should OrbitAPI earn trust when PrismNode times out under NovaLink lag?',
     '[["timeout","idempotent","trust","lag","contract"]]', 'entity timeout'),
    ('H99', 'governed_learning_loop', 2, 'How do Cortex cards differ from curriculum JSONL and Bitwork weights?',
     '[["cortex","curriculum","weight","authorize"]]', 'three memories'),
    ('H100', 'transfer_vs_template', 4, 'how should IdemKey services earn trust when LagShield retries under CobaltHub timeout?',
     '[["timeout","idempotent","retry","lag","trust","contract"]]', 'entity-swap H100'),
]
for hid, cap, hard, prompt, req, notes in extra:
    CASES.append(
        f'{{"id":"{hid}","capability":"{cap}","hardness":{hard},'
        f'"prompt":"{prompt}",'
        f'"required_any":{req},'
        f'"forbidden":["stuck is normal","behavioral complexity is observable"],'
        f'"notes":"{notes}"}}'
    )


def main() -> None:
    text = PACK.read_text(encoding="utf-8") if PACK.is_file() else ""
    ids = set(re.findall(r'"id"\s*:\s*"(H\d+)"', text))
    added = 0
    for line in CASES:
        m = re.search(r'"id":"(H\d+)"', line)
        hid = m.group(1)
        if hid in ids:
            print("skip", hid)
            continue
        if text and not text.endswith("\n"):
            text += "\n"
        text += line + "\n"
        ids.add(hid)
        added += 1
        print("add", hid)
    PACK.write_text(text, encoding="utf-8")
    print("added", added, "total", len(text.splitlines()))


if __name__ == "__main__":
    main()
