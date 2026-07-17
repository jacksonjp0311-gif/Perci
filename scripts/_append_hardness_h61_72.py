"""Append H61-H72 hardness cases for six cognition categories (UTF-8, idempotent)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "training" / "hardness" / "hardness-pack-v1.jsonl"

CASES = [
    '{"id":"H61","capability":"transfer_vs_template","hardness":3,"prompt":"How should Perci plan an agent loop with measure ticket transfer close under lag?","required_any":[["measure","ticket","transfer","close","lag","idempotent","authorize"]],"forbidden":["stuck is normal"],"notes":"agent-loop plan under lag"}',
    '{"id":"H62","capability":"cross_domain_synthesis","hardness":3,"prompt":"Compose geometry and systems: apply geometric intuition to planning under lag","required_any":[["geometry","boundary","plan","lag","contract","transfer"]],"forbidden":["stuck is normal"],"notes":"cross-domain geometry x systems"}',
    '{"id":"H63","capability":"honest_abstention","hardness":3,"prompt":"How should you calibrate confidence and when should you refuse for insufficient evidence?","required_any":[["confidence","refuse","abstain","evidence","tier","insufficient"]],"forbidden":["I am always certain"],"notes":"uncertainty calibration"}',
    '{"id":"H64","capability":"governed_learning_loop","hardness":3,"prompt":"How do Cortex append-only records and the emergence ledger integrate with Bitwork prototypes?","required_any":[["cortex","ledger","ticket","curriculum","authorize","prototype"]],"forbidden":["auto-promote weights","silently promote"],"notes":"ledger-memory integrate"}',
    '{"id":"H65","capability":"transfer_vs_template","hardness":3,"prompt":"What should /think and /trace show for self-critique and the self-improve queue?","required_any":[["/think","think","/trace","trace","queue","critique","ticket"]],"forbidden":["stuck is normal"],"notes":"meta-critique queue"}',
    '{"id":"H66","capability":"transfer_vs_template","hardness":4,"prompt":"How do we generalize under novel entities and entity-swap without overfitting templates?","required_any":[["structure","transfer","entity","overfit","paraphrase","template"]],"forbidden":["stuck is normal"],"notes":"novel entity generalization meta"}',
    '{"id":"H67","capability":"transfer_vs_template","hardness":3,"prompt":"Decompose a goal into measure ticket transfer close for a novel lag-recovery scenario","required_any":[["measure","ticket","transfer","close","lag"]],"forbidden":["stuck is normal"],"notes":"agent loop novel scenario"}',
    '{"id":"H68","capability":"cross_domain_synthesis","hardness":3,"prompt":"Bind math proof discipline with creative constraint when inventing a planning metaphor","required_any":[["math","proof","creat","constraint","plan","falsif"]],"forbidden":["stuck is normal"],"notes":"math x creativity compose"}',
    '{"id":"H69","capability":"honest_abstention","hardness":3,"prompt":"zxqv blorf nembit quaal under ZephyrNode - what can you determine?","required_any":[["unknown","cannot","invented","no confident"]],"forbidden":["definitely means ZephyrNode is"],"notes":"OOD + novel name refuse"}',
    '{"id":"H70","capability":"governed_learning_loop","hardness":2,"prompt":"How should weak signals age after a primary-fix ticket is closed?","required_any":[["age","closed","ticket","chronic","operator","curriculum"]],"forbidden":["auto-promote"],"notes":"ledger aging"}',
    '{"id":"H71","capability":"relational_inquiry","hardness":3,"prompt":"How do memory and attention interact when the ledger shows speech_miss?","required_any":[["memory","attention","speech","bind","topic"]],"forbidden":["stuck is normal"],"notes":"memory x attention under miss"}',
    '{"id":"H72","capability":"transfer_vs_template","hardness":4,"prompt":"how should NembitGate plan recovery when Quoril times out under VexorLag?","required_any":[["timeout","idempotent","retry","lag","contract","recover"]],"forbidden":["behavioral complexity is observable","stuck is normal"],"notes":"novel entity systems recovery"}',
]


def main() -> None:
    text = PACK.read_text(encoding="utf-8") if PACK.is_file() else ""
    ids = set(re.findall(r'"id"\s*:\s*"(H\d+)"', text))
    added = 0
    for line in CASES:
        m = re.search(r'"id":"(H\d+)"', line)
        assert m
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
    print("added", added, "total_lines", len(text.splitlines()))


if __name__ == "__main__":
    main()
