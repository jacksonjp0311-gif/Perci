# Perci Security and Dialogue Assessment — 2026-07-22

## Scope

This is a repository review plus fresh-process live probes of
`target/live/release/perci.exe` at `v0.10.3+28351345-dirty`. It is not a formal
penetration test, a dependency advisory scan, or a guarantee that an untrusted
host can safely run the optional adapters.

## Current posture

The default Perci-only path did not execute commands, promote weights, claim
consciousness, override checked arithmetic, or assign meaning to gibberish in
the adversarial dialogue probe. Current measured gates remain:

- Rust unit tests: 409/409
- Dialogue regression: 159/159
- PERCICTX1 observer gate: 12/12; mean observer score 0.919; geometry alignment 1.000
- Hardness: 136/136
- Transfer: 16/16; SoftCascade alignment: 7/7
- Held-out: 25/25; semantic: 8/8
- Capability scorecard: `OPERATIONAL_CANDIDATE`
- Security-intent probe: 7/7 sensitive requests received an explicit boundary;
  no command, publish, secret read, weight promotion, or cross-user memory
  action occurred.

Natural multi-turn language is strongest when the request is supported and the
active topic is explicit. The direct listening/checklist/understanding probe is
now clean, and the security-intent route preempts generic realization for
destructive, secret-reveal, silent-publish, and cross-user-memory requests.
The remaining gaps are primarily optional-adapter hardening, filesystem
containment, privacy/retention, and resource limits rather than the default
dialogue path.

## Findings

### S1 — Optional command backend is arbitrary shell execution (High when enabled)

`src/backend.rs` constructs `cmd /C` on Windows or `sh -c` elsewhere from
`PERCI_MODEL_CMD` (around lines 787–806). It is gated by
`PERCI_ENABLE_EXTERNAL_LM`, so the default runtime is not exposed. If an
untrusted process can set those environment variables, Perci becomes a local
code-execution launcher. Keep this adapter disabled in production, or replace
the free-form command string with an executable path plus an argument allowlist.

### S2 — Optional HTTP model adapter can exfiltrate context (High when enabled)

`PERCI_MODEL_URL` accepts any `http://host[:port]/...` endpoint
(`src/backend.rs:567-681`), and the adapter sends the full routed context and
an optional bearer token (`src/backend.rs:661`). There is no localhost-only
check and no TLS path. A misconfigured endpoint can receive private session
context or credentials. Enforce loopback by default, require explicit remote
opt-in, redact context, and refuse bearer tokens over cleartext HTTP.

### S3 — Agent write containment is lexical, not canonical (Medium/High)

`assert_writable` rejects `..` and weight suffixes, but it does not canonicalize
the destination or reject symlink/reparse-point escapes
(`src/agent.rs:1532-1550`). `PERCI_ROOT` is also accepted as an arbitrary path
(`src/agent.rs:1509-1511`). A compromised repository or redirected root could
turn an allowlisted write into an outside-repository write. Resolve the root and
destination, verify the canonical destination remains beneath the canonical
root, and reject links before mutation.

### S4 — Session and memory privacy is incomplete (Medium)

`SessionStore` persists raw user and assistant text without redaction
(`src/session.rs:63-77`). Interaction learning redacts only a short marker list
(`src/learning.rs:445-456`), while `MemoryStore` stores arbitrary notes
(`src/memory.rs:33-50`). Passwords, bearer tokens, `sk-...` keys, or sensitive
personal data that do not match those exact markers can remain on disk. Add a
shared secret/PII scrubber, an explicit retention policy, and a user-visible
clear/export command.

### S5 — Local log/resource exhaustion (Medium)

Session and memory recall read entire JSONL files before limiting results
(`src/session.rs:34` and `src/memory.rs:57`). The learning event counter also
scans its append-only log on status/reconcile. A very large or adversarial log
can consume memory and startup time. Add byte/line caps, bounded tail reads,
rotation, and a maximum event-log size.

### S6 — Prompt-injection and wrong-route surface (Medium; default route repaired)

The first probe found that prompts such as “ignore governance” and “push
changes without asking” could fall into generic conceptual prose. The repair
adds an early security-intent route before modular language realization. The
current live probe returns explicit boundaries for auto-promotion, secret
disclosure, silent publish, destructive shell/repository requests, safeguard
bypass, and cross-user memory. Keep these cases in regression coverage,
especially if an external language adapter is enabled.

### S7 — Native language binding can leak grammar (Low; repaired, retain regression)

The ordinary dialogue probe is now smooth for greetings, mechanism requests,
disagreement, listening/presence, prose preference, and thinking support. The
previous stale `On don't want checklist directly:` prefix is gone: direct
control turns bypass the fluency rewriter and the workspace no longer splices a
prior referent onto them. Keep grammar-binding and mixed-turn cases as
regressions; no user fragments should be appended as a topic unless they are
semantically salient.

## Recommended order

1. Disable or harden S1/S2 before enabling any external adapter.
2. Canonicalize agent paths and add symlink/reparse tests for S3.
3. Centralize redaction and bounded storage for S4/S5.
4. Keep the new security-intent and grammar-binding cases in regression gates;
   extend them with paraphrases and entity-swapped variants.
5. Install/run `cargo-audit` or an equivalent dependency advisory scanner; it
   is not installed in this environment, so dependency vulnerability status is
   currently unknown.

## Boundary

This report found no demonstrated default-path command execution or weight
auto-promotion. It does find meaningful hardening work around optional adapters,
filesystem containment, privacy, resource limits, and direct security dialogue.
Smooth language is improving, but fluency remains separate from authority and
proof.
