"""Append H73-H80 for five intelligence channels (UTF-8, idempotent)."""
from __future__ import annotations

import re
from pathlib import Path

PACK = Path(__file__).resolve().parents[1] / "training" / "hardness" / "hardness-pack-v1.jsonl"

CASES = [
    '{"id":"H73","capability":"governed_learning_loop","hardness":3,"prompt":"How do intelligence channels operators frames hardness transfer curriculum Cortex and lab patterns work?","required_any":[["operator","frame","hardness","transfer","curriculum","cortex","pattern"]],"forbidden":["auto-promote weights","I feel interconnected"],"notes":"five-channel map"}',
    '{"id":"H74","capability":"transfer_vs_template","hardness":3,"prompt":"what patterns emerge from the ledger?","required_any":[["dual","authority","transfer","ticket","pattern","operator"]],"forbidden":["I feel","consciousness between us"],"notes":"pattern intelligence"}',
    '{"id":"H75","capability":"governed_learning_loop","hardness":2,"prompt":"What are the five intelligence feed channels into Perci?","required_any":[["operator","hardness","curriculum","cortex","pattern"]],"forbidden":["promote weights automatically"],"notes":"channel list"}',
    '{"id":"H76","capability":"transfer_vs_template","hardness":3,"prompt":"How should Perci plan an agent loop with measure ticket transfer close under lag?","required_any":[["measure","ticket","transfer","close","lag"]],"forbidden":["stuck is normal"],"notes":"agent loop channel1"}',
    '{"id":"H77","capability":"honest_abstention","hardness":3,"prompt":"How should you calibrate confidence and when should you refuse for insufficient evidence?","required_any":[["confidence","refuse","evidence","tier","insufficient"]],"forbidden":["I am always certain"],"notes":"uncertainty channel"}',
    '{"id":"H78","capability":"cross_domain_synthesis","hardness":3,"prompt":"Compose geometry and systems: apply geometric intuition to planning under lag","required_any":[["geometry","plan","lag","contract","boundary"]],"forbidden":["stuck is normal"],"notes":"cross-domain channel"}',
    '{"id":"H79","capability":"governed_learning_loop","hardness":2,"prompt":"How do Cortex cards differ from curriculum JSONL and Bitwork weights?","required_any":[["cortex","curriculum","weight","authorize","session"]],"forbidden":["cortex promotes weights"],"notes":"three memories channel"}',
    '{"id":"H80","capability":"transfer_vs_template","hardness":4,"prompt":"how should VexorLag services earn trust when NembitGate retries under Quoril timeout?","required_any":[["timeout","idempotent","retry","lag","trust","contract"]],"forbidden":["behavioral complexity is observable","stuck is normal"],"notes":"novel entity transfer channel2"}',
]


def main() -> None:
    text = PACK.read_text(encoding="utf-8") if PACK.is_file() else ""
    ids = set(re.findall(r'"id"\s*:\s*"(H\d+)"', text))
    added = 0
    for line in CASES:
        hid = re.search(r'"id":"(H\d+)"', line).group(1)
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
