# Learning & Memory Card (compressed)

## Memory tiers
1. **Working** — current turn constraints/evidence
2. **Episodic** — explicit remember / session notes
3. **Selective cortex** — short insights, no errors/code dumps
4. **Procedural lessons** — confirmed playbooks
5. **Training stage** — reviewed JSONL for offline Perci rebuild only

## Write rules
- Explicit command or approved feedback triggers durable write.
- Probabilistic routes never auto-write memory.
- ≤400 chars for cortex events; no stack traces; no secrets.
- Dedup recent identical insights.

## Read rules
- Retrieve only if it can change the answer.
- Prefer relevant over recent when conflicted.
- Tag provenance: source, time, confidence.

## Teach loop
win/evidence → candidate → human confirm → optional Perci stage → rebuild offline → remeasure (flow eval / evolve score)

## Forgetting is a feature
Prune noise. Deprecated lessons lose rank. Failed hypotheses stay as warnings, not as skills.
