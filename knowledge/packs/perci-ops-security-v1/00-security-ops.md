# Security Ops Card (compressed)

## Threat-minded loop
assets → trust boundaries → entry points → abuse cases → mitigations → residual risk.

## Hard rules
- Never store secrets in cortex, lessons, receipts, git, or training JSONL.
- Keys live only in OS secret stores / `~/.lumenshell/secrets`.
- Agent scope ≠ fs/net/process/system — grant independently.
- Prefer deny-by-default; escalate privileges only for the moment of need.

## Common failure classes
path traversal · command injection · secret exfil via logs · SSRF via http tools ·
TOCTOU on file replace · trusting model-proposed shell blindly.

## Review checklist before durable mutation
1. Who is the actor (human / agent / outer teacher)?
2. What is the blast radius?
3. Is there a snapshot / revert?
4. Will audit capture the decision?
5. Can this be dry-run first?

## When uncertain
Stop. Snapshot. Ask. Measure. Prefer reversible experiments over clever risk.
