# Profiles, weight packs and run ledgers

Everything that customizes one installation lives in a **profile directory**
outside this repository: a policy overlay, sequence rules, private weight
packs, memory and run ledgers. The public repo ships generic defaults only.

```
PERCI_PROFILE=/path/outside/the/repo/my-profile perci mcp
```

`PERCI_PROFILE` defaults to `PERCI_DATA`, so an installation without a profile
behaves exactly as before. `perci tool status` reports the profile directory
and warns (`profile.inside_git_checkout`) if it sits inside a git checkout.

## Layout

```
my-profile/
  profile.json            perci.profile.v1 (optional; names the pieces below)
  policy.json             perci.policy.v1 overlay ("extends": "base") or a full replacement
  sequence-rules.json     perci.sequence.v1: classes + rules, merged into the policy's "sequence"
  packs/                  weight packs, one folder each (perci-pack.json), or a perci.packs.v1 index
    acme-router/
      perci-pack.json
      router.pwgt
  memory/perci.jsonl      cited-memory journal (PERCI_MEMORY overrides)
  ledger/<run>.jsonl      hash-chained step ledgers, one per run
  temporal/model.json     n-gram model learned from verified ledgers (perci ledger learn)
```

Nothing is required: missing optional files fall back to the generic
defaults. A file that **is** named in `profile.json` but cannot be read, or a
file that exists but is invalid, is an error (never silently ignored).

## profile.json (`perci.profile.v1`)

```json
{
  "schema": "perci.profile.v1",
  "name": "example",
  "policy": "policy.json",
  "sequence_rules": "sequence-rules.json",
  "packs": ["packs/acme-router"],
  "packs_index": "packs/packs.manifest.json",
  "tool_pack": "acme-router",
  "pins": {"acme-router": "<sha256 of its manifest>"},
  "require_pins": true,
  "calibration": {"acme-router": "calibration/acme-router.json"},
  "route": {"min_confidence_permille": 800, "min_margin": 4},
  "memory_journal": "memory/perci.jsonl",
  "tools": {"enabled": ["exact", "check_math", "route", "policy_check", "agent_gate_step", "ledger_append", "next_step"]}
}
```

Paths are relative to the profile directory unless absolute. Unknown keys are
ignored, so a host can keep its own settings in the same file. `packs` may
also be an object `name → {"path": "x.pwgt", "manifest": "x.pack.json"}`
where the manifest is a `perci.pack-manifest.v1` file (`checksum_sha256`,
`byte_length`). Environment variables win over the file: `PERCI_POLICY`,
`PERCI_TOOL_PACK`, `PERCI_TOOLS`, `PERCI_MEMORY`.

## Policy overlay

`policy.json` with `"extends": "base"` is merged onto the shipped
`config/policy.json`: objects merge key by key, rule lists append, and a rule
with the same `id` replaces the base rule. Without `extends` the file replaces
the base entirely. `PERCI_POLICY` points at one exact file instead (no merge).

## Sequence rules (`perci.sequence.v1`)

```json
{"schema": "perci.sequence.v1",
 "classes": {"lint": ["shell:cargo clippy"], "release": ["shell:make publish"]},
 "rules": [{"id": "site.release_requires_lint", "before": "release", "requires": "lint",
            "status": "pass", "phase": "post", "not_before": "code_change",
            "posture": "refuse", "reason": "lint before releasing"}]}
```

A step is a token such as `shell:git push`, `shell:cargo test`, `write` or
`delete` (`agent_gate_step` derives it from the action; wrappers like `sudo`
and env assignments are skipped). A class lists step patterns: a step matches
when it equals the pattern or starts with it followed by a space. A rule fires
when the step about to run is in `before`; it is satisfied by an earlier
ledger entry in `requires` with the given `status` (and `phase`), after the
last `not_before` entry. Unmet rules make the gate stricter (`posture`), never
looser; met rules are cited as `ledger:<run>#<seq>` evidence. The base policy
ships two: deploy requires passing tests after the last code change, and a
destructive delete requires a backup first. A sequence file with another
schema makes every gated step abstain with `SEQUENCE_RULES_INVALID`.

## Weight packs

```
perci packs list [--json]                     # every pack Perci can see, pinned or not
perci packs verify [NAME…] [--json]           # hash every file against its manifest
perci packs manifest DIR --name N --version V # write DIR/perci-pack.json for the files in DIR
perci packs calibrate NAME CASES.jsonl… [--out FILE]   # accuracy by margin bucket on held-out cases
```

Packs are found, first name wins: built-ins `v0.1`–`v0.3` (hashes from the
committed `models/PACKAGE_MANIFEST.json`), `PERCI_WEIGHT_PACKS` (path list of
pack folders, index files or folders of packs), `PERCI_PROFILE/packs/`, then
`profile.json` `packs` / `packs_index`. (`PERCI_PACKS` is the knowledge-note
root used by `intel_packs`, not weight packs.)

`perci-pack.json` (`perci.weight-pack.v1`):

```json
{"schema": "perci.weight-pack.v1", "name": "acme-router", "version": "1.0.0",
 "kind": "bitwork-router", "entry": "router.pwgt",
 "categories": ["billing", "support", "sales"],
 "files": [{"path": "router.pwgt", "bytes": 5057536, "sha256": "…"}],
 "lexical_priors": "off",
 "calibration": {"schema": "perci.route-calibration.v1", "buckets": [ … ]}}
```

An index (`perci.packs.v1`, `packs.manifest.json`) lists several packs as
`{pack_id, file, bytes, sha256, labels, sidecar, sidecar_sha256, default, role,
format}`; the pin of an index entry is the sha256 of the entry's canonical JSON.

Loading verifies the manifest (safe relative paths, 64-hex hashes, the entry
listed, the pin), hashes every file (the weights are hashed from the mapped
bytes that classification reads), and checks that the weights' labels equal
`categories`. A missing, modified or mispinned pack is an **error**
(`PACK_UNAVAILABLE`); Perci never falls back to another pack. Pins come from
`profile.json` `pins`; `"require_pins": true` rejects unpinned external packs.
Signatures are not supported (no crypto dependency): a `signature` field is
reported as unverified and the pin is the trust anchor.

**Lexical priors** are keyword boosts written for the built-in packs' label
names. They default to `off` for every other pack (a private label that
happens to be called `memory` would otherwise be boosted). A pack may ship its
own table: `"lexical_priors": {"points": 48, "labels": {"billing": ["invoice", "refund"]}}`.

**Calibration**: the score margin alone is a weak confidence signal (wrong
labels with large margins happen). `perci packs calibrate` measures accuracy
per margin bucket on labelled held-out cases (`text`|`prompt` +
`label`|`expected_domain`) and reports the Wilson 95% lower bound per bucket.
Put the result in the manifest's `calibration` or in `profile.json`
`calibration.<pack>`. `route` then reports `bitwork.confidence.permille` (the
lower bound for that margin) and abstains (`ROUTE_LOW_CONFIDENCE`) on an `llm`
route below `min_confidence_permille` (default 700). Uncalibrated packs report
`decision: "uncalibrated"` and gate only on an explicit `min_margin`.

## Run ledgers

Every verdict served by `perci mcp` is appended to `ledger/<run>.jsonl`
(`PERCI_RUN_ID`, else `mcp-<unix>-<pid>`; `PERCI_LEDGER=off` disables it). One-shot
`perci tool …` records only when `PERCI_RUN_ID` is set. Entries are
`perci.ledger.v1`: `run_id, seq, ts_unix, tool, phase, step, status,
input_sha256, output_sha256, receipt_sha256, cache_hit, note, prev_hash, hash`,
where `hash = sha256(canonical entry without hash)` and the first `prev_hash`
is `sha256("perci.ledger.v1|<run>")`. Editing, reordering, dropping or moving
an entry breaks the chain (`ledger_verify`, `perci ledger verify`); truncating
the tail is only detected against a known head (`expect_head`). A run whose
chain is broken is never appended to again. One writer per run.

```
perci ledger list | show RUN | verify [RUN…] | learn | refresh
```

`learn` (alias `refresh`) rebuilds `temporal/model.json` from every run that
verifies; `next_step` adds the current run live.
