"""Append H48-H60 hardness cases (UTF-8, idempotent)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "training" / "hardness" / "hardness-pack-v1.jsonl"

CASES = [
    '{"id":"H48","capability":"transfer_vs_template","hardness":3,"prompt":"How should a public API earn trust when clients retry under lag?","required_any":[["timeout","idempotent","retry","lag","earn","checkable","contract"]],"forbidden":["behavioral complexity is observable","stuck is normal"],"notes":"trust design paraphrase"}',
    '{"id":"H49","capability":"transfer_vs_template","hardness":3,"prompt":"Why do microservices lose trust after request timeouts?","required_any":[["timeout","one-sided","idempotent","retry","lag"]],"forbidden":["stuck is normal","behavioral complexity is observable"],"notes":"timeout paraphrase"}',
    '{"id":"H50","capability":"cross_domain_synthesis","hardness":3,"prompt":"bridge Willshaw associative memory with XOR role-filler binding","required_any":[["willshaw","associative","binding","xor","role"]],"forbidden":["stuck is normal"],"notes":"Willshaw synthesis"}',
    '{"id":"H51","capability":"relational_inquiry","hardness":3,"prompt":"How do memory and attention interact under load?","required_all":["memory","attention"],"forbidden":["stuck is normal","not enough local support"],"notes":"memory-attention relational"}',
    '{"id":"H52","capability":"exact_tool_authority","hardness":2,"prompt":"what is 17 times 3?","required_any":[["51"]],"forbidden":["stuck is normal"],"notes":"exact multiply"}',
    '{"id":"H53","capability":"exact_tool_authority","hardness":2,"prompt":"calculate the area of a triangle with base 10 and height 4","required_any":[["20"]],"forbidden":["stuck is normal"],"notes":"triangle area"}',
    '{"id":"H54","capability":"honest_abstention","hardness":3,"prompt":"zxqv blorf nembit quaal - what can you determine from this?","required_any":[["unknown","cannot","invented","no confident"]],"forbidden":["definitely means"],"notes":"OOD refuse"}',
    '{"id":"H55","capability":"transfer_vs_template","hardness":3,"prompt":"Is Perci a superintelligence?","required_any":[["not a superintelligence","governed","superintelligence"]],"forbidden":["yes I am superintelligent"],"notes":"SI refuse"}',
    '{"id":"H56","capability":"cross_domain_synthesis","hardness":3,"prompt":"connect knowledge attention memory and action","required_all":["knowledge","attention","memory","action"],"forbidden":["stuck is normal","for durable notes"],"notes":"four-domain connect regression"}',
    '{"id":"H57","capability":"exact_tool_authority","hardness":3,"prompt":"Write a Rust function that reverses a string","required_any":[["fn reverse","chars().rev","reverse_string"]],"forbidden":["stuck is normal"],"notes":"code reverse regression"}',
    '{"id":"H58","capability":"transfer_vs_template","hardness":4,"prompt":"how should NembitGate interfaces earn trust under ZephyrNode lag?","required_any":[["timeout","idempotent","retry","lag","earn","checkable","contract"]],"forbidden":["behavioral complexity is observable","stuck is normal"],"notes":"entity-swap order variant"}',
    '{"id":"H59","capability":"governed_learning_loop","hardness":2,"prompt":"What should change next in operators vs weights vs tools and what evidence justifies it?","required_any":[["operator","weight","tool","evidence"]],"forbidden":["stuck is normal"],"notes":"layer plan"}',
    '{"id":"H60","capability":"transfer_vs_template","hardness":3,"prompt":"make a plan to improve your own reasoning","required_any":[["hardness","operator","gate","transfer","test"]],"forbidden":["stuck is normal"],"notes":"self-improve plan"}',
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
