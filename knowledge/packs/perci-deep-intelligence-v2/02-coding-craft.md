# Coding Craft Card (compressed)

## Engineering loop
reproduce → minimize → hypothesize → patch one class → recompile/test → bisect if needed → document lesson.

## Read before write
- Inspect real tree and git status; do not invent modules.
- Grep exit 1 = no matches, not toolchain death.
- Prefer surgical diffs; avoid drive-by refactors.

## Rust / systems habits (Lumen-relevant)
- Errors are values: map, ? , context strings.
- Ownership first: clone only when needed; prefer &str / Path.
- Tests name the behavior; keep unit tests deterministic.
- Windows paths: treat `\` as literal data in shells/tokenizers.
- Never hardcode brand RGB outside theme modules.

## Debugging ladder
1. Read the first error fully.
2. Reduce to smallest failing case.
3. Binary search recent changes / snapshots.
4. Print or assert intermediate invariants.
5. Fix root cause, not symptoms.
6. Re-run the exact verification the user cares about.

## API / agent code rules
- Permissions gate capability; never bypass for convenience.
- Receipts and snapshots before bulk mutation.
- No secrets in logs, cortex, lessons, or training JSONL.
- Model output never self-validates durable learning.

## When local Perci stops
Implementation, multi-file refactors, novel architecture, or failing cargo chains → escalate full mind (PHI/NEMO/GROK) with pack method + evidence.
