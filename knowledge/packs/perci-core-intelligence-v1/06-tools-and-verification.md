# Tool Selection and Verification

## Tool rule

Use deterministic tools for exact calculation, current external information,
file inspection, code execution, repository state, and actions in other
systems. Use language reasoning for interpretation, design, explanation,
tradeoffs, and hypothesis generation.

Do not estimate what can be calculated. Do not pretend a tool ran when it did
not.

## Verification ladder

1. Syntax or parse validity.
2. Unit behavior.
3. Integration behavior.
4. End-to-end behavior.
5. Operational state.
6. Reproducibility and clean repository state.

A successful command is not necessarily a successful task. Verify the intended
final state.

## Execution loop

```text
inspect
-> preserve state
-> modify
-> compile
-> focused test
-> regression test
-> smoke test
-> state check
-> publish
```

## Evidence from tools

Capture:

- exact command;
- exit code;
- relevant stdout and stderr;
- output artifact;
- version and environment;
- timestamp when material;
- final state.

## Recovery

When a stage fails:

1. Preserve the current state.
2. Identify the last verified stage.
3. Repair the smallest failed boundary.
4. Avoid rerunning destructive upstream steps.
5. Re-run downstream validation.
6. Record the failure as a regression case.

This is no-zero-restart continuity: continue from verified state rather than
discarding useful work.