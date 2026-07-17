# Perci Intelligence Packs

Perci intelligence packs are curated, inspectable procedural knowledge indexed
by Cortex. They do not copy a language model's hidden weights or private
reasoning. They encode observable high-value behavior as explicit operators,
decision protocols, counterexample checks, evidence rules, and recovery loops.

## Runtime path

```text
prompt
  -> explicit command parser
  -> exact tools when supported
  -> zero-context fast path for trivial conversation
  -> Bitwork cognitive classification
  -> warm Cortex retrieval for substantive work
  -> bounded intelligence-pack guidance
  -> optional external language backend
  -> response
```

The first substantive prompt lazily starts a persistent Python process. Cortex
and SQLite remain warm for the rest of the Perci session. Repeated retrievals
are served by a bounded TTL cache.

## Trust boundary

Intelligence packs are curated references, not automatic truth. Cortex returns
content hashes and source locations. Current source, executable tests,
measurements, and explicit human authority remain controlling.

## Packs shipped

| Pack id | Role |
|---------|------|
| `perci-core-intelligence-v1` | Foundational control, evidence, engineering, math/science, memory |
| `perci-deep-intelligence-v2` | Compressed operators: math · coding · reasoning · science · language · introspection · self-awareness · governance · learning · transfer |
| `perci-ops-security-v1` | Security ops · network/Windows · emergence-watch |

Offline retrieval (`intel_packs`) ranks cards by keyword + domain priors even when the Cortex daemon is cold. Warm Cortex still indexes the same markdown under `knowledge/packs/`.

Install into Lumen cortex (selective discoveries):

```text
perci packs
perci packs install
perci packs probe "self awareness math"
```

## Pack integrity

Every pack includes `manifest.json` with SHA-256 hashes. Validate with:

```powershell
python scripts/verify_intelligence_pack.py knowledge/packs/perci-deep-intelligence-v2
python scripts/verify_intelligence_pack.py knowledge/packs/perci-core-intelligence-v1
```

## Performance controls

```text
PERCI_CORTEX_MODE=auto    default; skip trivial prompts
PERCI_CORTEX_MODE=always  retrieve for every language prompt
PERCI_CORTEX_MODE=off     disable Cortex retrieval
PERCI_CORTEX_CACHE_SECONDS=45
PERCI_CORTEX_CACHE_ITEMS=24
PERCI_DIAGNOSTICS=1       show Bitwork score details
```