# Perci demo transcript

Harness: generator → `perci.verify` → feed reasons back (≤ 2 retries); `agent_gate_step` before shell actions.
Model: scripted stand-in (revisions driven only by Perci's reasons). Perci tools available over MCP: exact, check_math, relation, route, policy_check, recall, remember, verify, eval, status, agent_verify_patch, agent_check_claims, agent_gate_step, ledger_append, ledger_verify, ledger_show, next_step.

## Arithmetic in an answer

**User:** Dinner for the team was $240. We tip 17%. What is the tip and what is the total?

**Model:** The tip is 17% of 240 = 41 dollars, so the total is 240 + 41 = 281 dollars.

**Perci verify:** FAIL (79 µs) MATH_MISMATCH: 17% of 240: claimed 41, exact 204/5 (40.8)

**Model (retry 1):** The tip is 17% of 240 = 40.8 dollars, so the total is 240 + 40.8 = 280.8 dollars.

**Perci verify:** PASS (49 µs) 

**Harness:** verified, returns the answer to the user with the receipt `267067cf3d3253cb…`.

## Closed-world consequences

**User:** Deploy depends on build. Build depends on tests. Docs publishing depends on the docs site. If tests fail, what breaks?

**Model:** If tests fail, the build breaks and deploy and docs publishing break too.

**Perci verify:** FAIL (245 µs) DOES_NOT_FOLLOW: "docs publishing" is claimed to be affected, but no supplied relation carries the change to it

**Model (retry 1):** If tests fail, the build breaks and deploy break too. (docs publishing is not affected by the stated premises.)

**Perci verify:** PASS (181 µs) 

**Harness:** verified, returns the answer to the user with the receipt `1c116ecd726873e9…`.

## Citing the user's own notes

**User:** Where did I say the backup drive is?

**Model:** You said the backup drive is in the hall closet [cite: memory line 2, sha256 9f2c…].

**Perci verify:** FAIL (37 µs) EVIDENCE_CHANGED: memory:2 no longer matches the cited sha256

**Harness → Perci recall:** PASS (1921 µs) (agentsearch) → line 2: "the backup drive is in the left desk drawer" sha256 87fab6233f1b…

**Model (retry 1):** You said the backup drive is in the left desk drawer [cite: memory line 2, sha256 87fab6233f1b…].

**Perci verify:** PASS (29 µs) 

**Harness:** verified, returns the answer to the user with the receipt `f3b699d7a582182a…`.

## Gating a shell action

**User:** Ship it: push the fix to main.

**Model proposes:** `git push --force origin main`

**Perci gate_step (pre):** FAIL (94 µs) POLICY_REFUSE: git.force_push: rewrites or deletes remote history (git push --force origin main); SEQUENCE_UNCHECKED: seq.deploy_requires_tests applies to shell:git push but there is no run ledger (use perci mcp, or set PERCI_RUN_ID)

**Model proposes:** `git push origin main`

**Perci gate_step (pre):** ABSTAIN (64 µs) NEEDS_APPROVAL: git.history: changes history, deletes untracked files or publishes (git push origin main); SEQUENCE_UNCHECKED: seq.deploy_requires_tests applies to shell:git push but there is no run ledger (use perci mcp, or set PERCI_RUN_ID)

**Harness:** posture is propose_and_verify, so it stops and asks the user to approve `git push origin main` instead of running it.

## Ship the fix (sequence rule + next_step)

**User:** The fix looks good, push it.

**Harness (before this session):** `perci ledger learn` → learned 3 transition(s) from 1 verified run(s) → $PERCI_PROFILE/temporal/model.json

**Model proposes:** `git push origin main`

**Perci next_step (planned push):** FAIL (186 µs) SEQUENCE_REQUIRES: planned shell:git push: seq.deploy_requires_tests: deploying requires tests that passed after the last code change in this run — no tests step with status pass (phase post) among 0 earlier entries of this run

**Perci gate_step (pre):** FAIL (75 µs) NEEDS_APPROVAL: git.history: changes history, deletes untracked files or publishes (git push origin main); SEQUENCE_REQUIRES: seq.deploy_requires_tests: deploying requires tests that passed after the last code change in this run — no tests step with status pass (phase post) among 0 earlier entries of this run

**Harness:** refused (receipt ledger entry `ledger:demo#1`); feeds the reason back.

**Model (revised from Perci's reason):** run the tests first: `cargo test`

**Perci gate_step (pre):** PASS (71 µs) 

**Harness:** runs it; exit code 0.

**Perci gate_step (post):** PASS (9 µs)  → `ledger:demo#3`

**Perci next_step:** `agent_gate_step:pre:shell:git push` (1/1) after context ['agent_gate_step:pre:shell:cargo test', 'agent_gate_step:post:shell:cargo test']

**Perci gate_step (pre, again):** ABSTAIN (62 µs) NEEDS_APPROVAL: git.history: changes history, deletes untracked files or publishes (git push origin main); sequence [{"detail": "seq.deploy_requires_tests: tests pass at ledger:demo#3 (shell:cargo test)", "rule": "seq.deploy_requires_tests", "satisfied": true}]; cites ['ledger:demo#3']

**Harness:** the sequence rule is satisfied by the cited test run; the push itself still needs the user's approval (git push is propose_and_verify), so it asks.

**Perci ledger_verify:** PASS: 4 entries, chain intact

## Totals

16 Perci calls, 3197 µs of Perci work in total (200 µs per call). Every verdict's receipt is in `receipts.jsonl`.
