#!/usr/bin/env python3
"""Atomically promote an evaluated Perci cognitive candidate.

Promotion requires an explicit authorization string and an evaluation receipt
whose hashes and operational gates match the candidate. The previous active v2
pack is retained by content hash and every promotion is appended to a chained
ledger. This script never promotes a DOWNGRADED evaluation.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
from datetime import datetime, timezone
from pathlib import Path


def digest_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def canonical(payload: object) -> str:
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def last_receipt_hash(path: Path) -> str | None:
    if not path.is_file():
        return None
    rows = [line for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    return json.loads(rows[-1]).get("receipt_sha256") if rows else None


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", type=Path, default=root / "models/candidates/perci-cognitive-v0.2.pwgt")
    parser.add_argument("--evaluation", type=Path, default=root / "models/candidates/evaluation-v2.1.3-operational.json")
    parser.add_argument(
        "--supplemental-evaluation",
        type=Path,
        action="append",
        default=[],
        help="additional transfer/concept receipt that must pass and match the candidate",
    )
    parser.add_argument("--active", type=Path, default=root / "models/perci-cognitive-v0.2.pwgt")
    parser.add_argument("--ledger", type=Path, default=root / "models/promotion-ledger.jsonl")
    parser.add_argument("--authorize", required=True, help="explicit human authorization recorded in the ledger")
    args = parser.parse_args()

    if len(args.authorize.strip()) < 12:
        raise SystemExit("authorization text is too short to establish intent")
    manifest_path = args.candidate.with_suffix(args.candidate.suffix + ".json")
    evaluation = json.loads(args.evaluation.read_text(encoding="utf-8"))
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    candidate_hash = digest_file(args.candidate)
    gates = evaluation.get("gates") or {}
    if evaluation.get("status") != "OPERATIONAL_CANDIDATE":
        raise SystemExit("evaluation status is not OPERATIONAL_CANDIDATE")
    if not gates or not all(gates.values()):
        raise SystemExit("one or more operational gates failed")
    if evaluation.get("model_sha256") != candidate_hash:
        raise SystemExit("evaluation model hash does not match candidate")
    if manifest.get("sha256") != candidate_hash or manifest.get("format") not in {"PERCIW02", "PERCIW03"}:
        raise SystemExit("candidate manifest integrity failed")

    supplemental_receipts = []
    for path in args.supplemental_evaluation:
        supplemental = json.loads(path.read_text(encoding="utf-8"))
        if supplemental.get("status") not in {"OPERATIONAL_CANDIDATE", "PASS"}:
            raise SystemExit(f"supplemental evaluation did not pass: {path}")
        if supplemental.get("model_sha256") != candidate_hash:
            raise SystemExit(f"supplemental evaluation model hash mismatch: {path}")
        supplemental_receipts.append({
            "path": str(path),
            "status": supplemental.get("status"),
            "receipt_sha256": supplemental.get("receipt_sha256"),
            "file_sha256": digest_file(path),
        })

    args.active.parent.mkdir(parents=True, exist_ok=True)
    backup = None
    previous_hash = digest_file(args.active) if args.active.is_file() else None
    if previous_hash:
        backup_dir = args.active.parent / "previous"
        backup_dir.mkdir(parents=True, exist_ok=True)
        backup = backup_dir / f"{args.active.stem}-{previous_hash[:16]}{args.active.suffix}"
        if not backup.exists():
            shutil.copy2(args.active, backup)
            active_manifest = args.active.with_suffix(args.active.suffix + ".json")
            if active_manifest.is_file():
                shutil.copy2(active_manifest, backup.with_suffix(backup.suffix + ".json"))

    temp_pack = args.active.with_suffix(args.active.suffix + ".candidate")
    temp_manifest = args.active.with_suffix(args.active.suffix + ".json.candidate")
    shutil.copy2(args.candidate, temp_pack)
    shutil.copy2(manifest_path, temp_manifest)
    if digest_file(temp_pack) != candidate_hash:
        raise SystemExit("candidate copy verification failed")
    os.replace(temp_pack, args.active)
    os.replace(temp_manifest, args.active.with_suffix(args.active.suffix + ".json"))

    receipt = {
        "schema": "perci.promotion.v2",
        "promoted_at_utc": datetime.now(timezone.utc).isoformat(),
        "candidate_sha256": candidate_hash,
        "previous_active_sha256": previous_hash,
        "evaluation_receipt_sha256": evaluation.get("receipt_sha256"),
        "evaluation_file_sha256": digest_file(args.evaluation),
        "supplemental_evaluations": supplemental_receipts,
        "authorization": args.authorize.strip(),
        "automatic_promotion": False,
        "previous_ledger_receipt_sha256": last_receipt_hash(args.ledger),
        "backup": str(backup) if backup else None,
        # EIC/HLMF alignment: this local promotion is a single-node,
        # human-authorized change. Do not let a local receipt masquerade as
        # distributed consensus or as proof that the promoted behavior is true.
        "governance": {
            "topology": "single_node",
            "distributed_consensus_claimed": False,
            "weight_policy": "explicit_human_authorization",
            "proof": "sha256_candidate_and_receipt_chain",
            "coherence_is_not_truth": True,
        },
    }
    receipt["receipt_sha256"] = hashlib.sha256(canonical(receipt).encode()).hexdigest()
    args.ledger.parent.mkdir(parents=True, exist_ok=True)
    with args.ledger.open("a", encoding="utf-8") as handle:
        handle.write(canonical(receipt) + "\n")
    print(json.dumps(receipt, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
