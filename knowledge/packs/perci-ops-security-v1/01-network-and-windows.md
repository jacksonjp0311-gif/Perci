# Network & Windows Ops Card (compressed)

## Network hygiene
- Resolve before connect; prefer timeouts; treat untrusted hosts as hostile.
- Cache is not truth; live probe when decisions depend on reachability.
- Local-first: offline path must still work for core shell features.

## Windows path reality
- `\` is a path separator users paste; tokenizers must not treat it as escape.
- OneDrive/Desktop paths are long; avoid brittle CWD assumptions.
- Locks on `target\release\*.exe` are common — rebuild debug or kill holders.

## Process / host
- List before kill; scope process tools under permission.
- Environment variables may carry secrets — never dump wholesale into cortex.
- Installers/launchers (`run.cmd`) beat PATH ceremony.

## Mesh telemetry transfer (PulseMesh idea)
Normalize sensors → health/anomaly → ledger. Agents act on evidence packets, not vibes.

## Escalation
Deep network forensics, kernel, or enterprise policy → full mind + human authority.
