# PERCI

<p align="center">
  <img src="assets/icons/perci-hero-darkblood.jpg" alt="Perci dark-blood sparse cognition lattice" width="920">
</p>

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" alt="Perci mark" width="96" height="96">
  &nbsp;&nbsp;
  <img src="assets/generated/perci-darkblood-badge.svg" alt="Perci version badge (stamped from Cargo.toml)" width="160">
</p>

<p align="center">
  <strong>Local governed sparse cognition.</strong><br>
  Not a cloud LLM. Not a pretend mind. A Rust-native neuro-symbolic stack that<br>
  <em>routes in bits, thinks in operators, speaks like a collaborator — and shows its work on demand.</em>
</p>

<p align="center">
  <img alt="Software" src="https://img.shields.io/badge/software-v0.9.8-8b0000?style=for-the-badge">
  <img alt="Rust" src="https://img.shields.io/badge/core-Rust-000000?style=for-the-badge&logo=rust">
  <img alt="Local first" src="https://img.shields.io/badge/runtime-local--first-111827?style=for-the-badge">
  <img alt="Bitwork" src="https://img.shields.io/badge/Bitwork-PERCIW03-5c0a12?style=for-the-badge">
  <img alt="Inference" src="https://img.shields.io/badge/hot_path-integer_only-059669?style=for-the-badge">
  <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-d97706?style=for-the-badge">
</p>

<p align="center">
  <a href="https://github.com/jacksonjp0311-gif/Perci"><strong>github.com/jacksonjp0311-gif/Perci</strong></a>
</p>

---

<p align="center">
  <img src="assets/icons/perci-stack-strip.svg" alt="Perci stack: reflex → Bitwork → operators → tools → thought arc → human speech" width="920">
</p>

## What you just walked into

**Perci** is an experimental **cognitive OS for a single machine**:

| Layer | Job |
|-------|-----|
| **Bitwork PERCIW03** | ~200 MiB sparse pack · **403,163** prototypes · **124** concepts · 4096-bit AND/POPCOUNT |
| **Transformer-bridge algebra** | Soft-α attention · dual residual ANDNOT · VSA bind/bundle · Willshaw HVs · session CTX |
| **SoftCascade thought arc** | thesis → warrant → boundary → check — spoken as human prose, not card dumps |
| **Operators** | Trust/systems, partition recovery, synthesis, refuse-hallucinate, code, plans, introspection |
| **Self-critique** | Thin drafts get one residual second angle — silent metacognition |
| **Emergence lab (L8)** | Tickets → **transfer suite** → repair/close · `release_gates.py` · agent `--full --repair` |
| **Capability Fabric (v0.8.4)** | Governor: native language · typed world model · knowledge · proof · code · multi-AI handoff/next/regress · SoftCascade pack-align |
| **Exact tools** | Math & geometry that *compute*, never guess |
| **Governance** | Append-only memory · Cortex · style memory · weight promote only with **human authorize** |

Chat is **clean** (no cognition dump). Inspect with **`/think`** (plan), **`/field`** (geometry laws), **`/lab`** (tickets). Style with **`/concise` · `/deep` · `/balanced`**.

> Fluency without transfer gates is not intelligence.  
> Consciousness claims without sensors are not honesty.

---

## Why this is different

Most assistants bury routing, language, tools, and memory inside one opaque model.

Perci **separates them**:

```text
  input
    │
    ▼
 ┌──────────┐     ┌─────────────────────────────┐
 │  reflex  │────▶│  Bitwork field (α, residual,│
 └──────────┘     │   VSA, multipartite mass)   │
                  └─────────────┬───────────────┘
            ┌───────────────────┼───────────────────┐
            ▼                   ▼                   ▼
      exact tools         operators           SoftCascade
      math · geometry     trust · code        thought arc
            │             synthesis · refuse        │
            └───────────────────┬───────────────────┘
                                ▼
                     human speech  ·  /think inspects
```

**What the field actually does** (live classify, 2026-07-17): contested prompts show **margin 0–2** and **overlap_z 15–26** — multipartite structure is real, not marketing. See [`docs/WEIGHT_REASSESSMENT_v0616.md`](docs/WEIGHT_REASSESSMENT_v0616.md).

## Emergent discoveries from the v0.7.3 loop

The strongest new result is not a claim that the pack became conscious. It is a measurable engineering loop:

```text
weak route → typed field event → ticket/curriculum candidate
          → operator or tool repair → paraphrase/entity transfer
          → close only when the gate holds
```

The current ledger shows a **dual-authority split**: Bitwork probes the geometry while operators own most successful speech. This is useful, but it also exposes the next bottleneck: routing alignment, not simply more prototypes. In the latest 500-event window, the field recorded 443 matches, 103 curriculum `primary_off` events, and 82 `geometry_blind` events; 18 speech outcomes were recorded, all successful. Transfer is therefore stronger evidence than smooth wording, while speech coverage still needs to grow.

The system now treats three memories as distinct: the Bitwork pack, append-only ledgers, and session/Cortex state. Folding them into one mutable blob would make curriculum provenance ambiguous. The active `.pwgt` remains human-authorized; this release changes operators, orchestration, evaluation, and governance rather than silently changing weights.

Conversational repairs are regression targets too. “What are you sensing?” must reach operational introspection, while cross-domain prompts such as geometry plus life must preserve a concrete relation and its boundary instead of reusing a stock concept. The local language sidecar keeps the operator’s answer in the foreground; provenance and governance remain inspectable without forcing the same header/footer into every response.

## v0.8.8 layered low-bit assessment gate

Perci now has a versioned `PERCLBW1` representation for experiments that need
more information than a naked binary weight can carry. Ternary block masks
preserve direction and zero; Q8.8 scales restore magnitude; residual ternary
planes recover approximation error; INT4 activations use a sparse Q8.8 escape
lane; and an orthonormal Walsh–Hadamard rotation redistributes outliers before
quantization. A bounded low-rank correction path restores repeatable residual
directions without changing the active `PERCIW03` pack.

This is a measured representation layer, not a dense Transformer hidden behind
new terminology. Run `cargo run -- lowbit probe` to exercise quantization,
correction, rotation, and binary round-trip checks. Details and the measured
fixture are in [`docs/LOWBIT_LAYER.md`](docs/LOWBIT_LAYER.md).

### What changed in this evolution

The system now has a second, explicitly layered numerical language for future
native matrices. Direction and topology remain cheap bit patterns; ternary zero
can suppress a feature; local scales restore amplitude; residual planes encode
what the first approximation missed; the INT4 path carries ordinary activation
variation; a sparse precision lane catches exceptional signals; and the
low-rank correction path restores repeated residual structure. The active
4,096-bit Bitwork field still routes Perci, while `PERCLBW1` gives a future
trainable layer a way to preserve information without turning every value into
full precision.

The first native training loop is now reproducible rather than aspirational:

```powershell
cargo run -- lowbit train `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw `
  --block-size 64 --residual-planes 2 --rank 8
```

It writes a candidate binary and a receipt containing checksums, weight error,
held-out matrix-vector error, and the baseline comparison. The receipt always
sets `promote_recommended: false`; evaluation can produce evidence, never
authority. The example fixture is intentionally tiny so the packing mechanics
can be reproduced on any machine.

The assessment step now reopens the candidate from disk instead of trusting the
in-memory training object:

```powershell
cargo run -- lowbit assess `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw
```

This independent receipt checks the `PERCLBW1` magic/version, dimensions,
checksum, reconstruction, and held-out matrix-vector behavior. The current
fixture returns `assessment: PASS`; that means the representation survived a
reopen and beat its ternary-plus-residual baseline on this synthetic workload.
It does not mean the candidate learned language, facts, or general intelligence.

### What we found

The evolution exposed a useful boundary: adding bits back as scales, residuals,
outlier lanes, and correction factors measurably reduces numerical loss, while
the main bottleneck for human dialogue remains semantic coverage and generation
rather than raw file size. The next compounding loop is therefore layered:
train a reviewed native field, assess it from serialized bytes, run transfer and
dialogue regressions, and only then consider a human-authorized promotion. More
prototypes without this loop would increase storage without proving cognition.

The release gate is deliberately stricter than the fixture: transfer is green
(`16/16` plus `7/7` SoftCascade), the held-out candidate suite is green
(`25/25`), semantic evaluation is green (`8/8`), and hardness is now green
(`100/100`). The last repair added explicit interaction parsing so
`memory` and `attention` remain present under the load variant. These are
engineering gates, not evidence of consciousness or frontier parity.

### How this relates to a language model

The conceptual bridge is real: both systems use distributed representations,
multiple projections, residual correction, context state, and a readout stage.
Perci expresses those ideas through sparse binary/ternary geometry, exact
operators, native binary language fields, and governed composition. A
Transformer expresses them through dense learned tensors and next-token
probabilities. That makes the architectures analogous at the level of
functions, not equivalent in capability.

Perci should therefore feel direct, adaptive, creative, and coherent where its
operators, language field, memory, and evidence support the request. It cannot
honestly promise zero errors or frontier-model breadth yet: unseen language,
long-horizon world knowledge, and open-ended token generation remain measured
limits. The engineering target is clear—reduce those limits with held-out
transfer, dialogue regression cases, and native candidate training rather than
declaring parity from a small file.

Human-facing dialogue is kept separate from internal geometry. Raw role/filler
tags are available to `/think` and receipts but are filtered from chat, so a
user receives a natural relation instead of strings such as `agent:...` or
`relate:...`. This is a concrete fluency repair, not a claim that hidden
chain-of-thought has been exposed or that the system is conscious.

## v0.8.8 adaptive dialogue shaping

The next bottleneck was not another pile of stock phrases; it was choosing the
right answer shape for the turn. Perci now has a deterministic presentation
layer that sits after routing and before the CLI renders the reply:

```text
user turn + recent thread
        |
        +-- depth cues: brief / balanced / deep
        +-- referent check: resolve “this” or ask for the noun
        +-- relevance gate: reject unrelated concept insights
        +-- directness pass: lead with the answer, then add mechanism
        +-- relation gate: weave multi-concept frames only when requested
        v
human-facing reply
```

Explicit cues such as “short answer”, “one sentence”, “go deeper”, and “step by
step” now select a bounded `Brief`, `Balanced`, or `Deep` response. Follow-ups
use the active thread; an ungrounded “why does this matter?” with no history
asks what “this” refers to instead of guessing. The memory/attention relation
has a stable explanation at all three depths, and low-bit follow-ups retain the
low-bit subject rather than falling into a life or identity card.

The voice layer also filters unrelated Bitwork mixture, residual, and VSA
frames. Those structures remain inspectable through `/think`, but ordinary
speech receives them only for explicit synthesis, comparison, or interaction
prompts. This is the boundary between emergent association and conversational
drift: novelty is useful when it is anchored to the user's nouns and requested
relation.

These changes improve directness, continuity, and readable variation; they do
not turn the native field into a dense next-token language model or establish
frontier parity. The regression suite measures the improvement, while the
existing evidence and promotion gates remain unchanged.

### Reflective dialogue loop

The local harness `scripts/evaluate_dialogue_v4.py` now provides a bounded
observe → diagnose → repair → retest loop. On 2026-07-18 it ran 159 isolated
dialogue, teaching, ambiguity, transfer, exact-tool, and self-audit prompts
against a temporary Perci daemon. The first pass scored `157/159`; both misses
were the same exact-tool provenance wording (`checked arithmetic` versus the
required `checked rational arithmetic`). The repair was made in `src/chat.rs`,
not in the weights, and the rerun scored `159/159`. The receipt is
`models/candidates/evaluation-v4-dialogue-reflective-20260718.json`.

The loop confirms the useful learning boundary: session context, dialogue
profile, and pending teaching candidates can change the current interaction;
they do not silently mutate the active Bitwork file. A fresh-process A/B test
with model hashes is still required before calling a behavior change a weight
change.

## v0.8.9 open-language bridge: noisy input, composition, and expression

The next repair targets the part users actually feel: language should survive a
typo and still answer the intended question. Perci now computes a bounded,
character-aware route key before reflex routing, operators, Bitwork lexical
priors, voice depth, and native phrase selection. An explicit domain alias is
preferred; otherwise a unique one-edit Damerau-Levenshtein match is accepted:

```text
r(x) = collapse_whitespace(lowercase(repair_typos(x)))
d_D(token, candidate) <= 1  and  candidate is unique
```

That means `natral langauge`, `exlpain memory and atention`, and
`calcualte 17 percent of 240` reach the same semantic path as their clean
counterparts, while invented names such as `Nembit` remain out of distribution.
Punctuation and spacing are preserved, and the active weights are not mutated.
The paired curriculum is [`training/dialogue-typo-v1.jsonl`](training/dialogue-typo-v1.jsonl);
the implementation and rationale are in [`docs/OPEN_LANGUAGE_BRIDGE.md`](docs/OPEN_LANGUAGE_BRIDGE.md).

The native open-language gate now admits bounded conceptual prompts (“what do
you think,” “how would you describe,” “give me an image,” and “connect …”) while
keeping exact math, proof, capability, and OOD questions on their authoritative
paths. Native continuations are selected from a small beam using topic overlap,
relation novelty, recent-answer distance, and typed relation/world scores. The
voice layer remains the expression governor: it chooses brief/balanced/deep
length, checks referents, leads with a direct answer, and only weaves a metaphor
when the user asks for one.

This is a practical bridge, not a claim of frontier parity. Character/byte-level
research supports spelling-noise robustness but also shows a sequence-length
tradeoff ([CANINE](https://arxiv.org/abs/2103.06874),
[ByT5](https://arxiv.org/abs/2105.13626), and the
[character-structure study](https://aclanthology.org/2023.findings-acl.770/)).
Grammar and constrained-decoding work likewise supports selecting a relation
frame before rendering prose ([grammar-based decoding](https://aclanthology.org/2023.findings-acl.91.pdf)).
Perci adopts those principles with Rust-only deterministic machinery and keeps
the gap measurable: routing robustness, transfer, dialogue regression, exact
tools, and abstention must all remain green before a candidate can be reviewed.

### Follow-up fluency repair

The first open-language probe exposed a narrower bottleneck: prompt scaffolding
was leaking into the topic (`express thought code`), and the native phrase
field could answer a creative request with a code-method card. v0.8.9 now drops
scaffolding words before topic binding, reserves creative prompts for a bounded
compositional frame, and repairs the recurring multi-domain grammar artifact
where “the relationship … is a relation” repeated itself. The new regression
tests cover both behaviors; the native field remains responsible for learned
continuation, while the voice layer owns readable composition.

The next probe found a continuity bottleneck rather than a weight shortage:
disagreement, revision, clarification, and typo-bearing turns could be routed
as fresh topics. The follow-up operator now preserves the substantive claim
inside a prior quoted answer, handles forms such as `I dont agree`, resolves
system-versus-person scope, and keeps `explain that differently` tied to the
current thread. Prompt words such as `creative`, `fresh`, and `angle` are also
excluded from topic binding. This improves conversational continuity without
silently mutating the Bitwork artifact.

The follow-up training phase is isolated in
`training/dialogue-continuity-v1.jsonl` with a 12-case paraphrased holdout.
`scripts/train_dialogue_candidate.py` builds a topic-conditioned PERCPHR1
candidate behind the runtime primers; `scripts/evaluate_dialogue_candidate.py`
compares it with the active field in fresh processes. The initial candidate
held at `5/12` required checks and `0.5` topic binding in both arms, so it is
recorded as `HOLD`. This is useful evidence: the candidate changes direct
phrase sampling, but full chat is still governed mainly by the operator and
voice layers. No weight promotion is justified yet.

## v0.8.11 phase-seven alignment: continuity as a measured weight candidate

This iteration aligns the native phrase field with the same context the runtime
uses to speak. Each continuation now receives an inspectable intent label and
the actual salient topic tokens behind the `<topic>` marker; the marker remains
render-only, while the learned transition history sees the words that must be
carried across a turn. This closes a representation gap without pretending that
the phrase field is a frontier language model.

The reviewed phase-seven corpus contains 24 training rows and 12 paraphrased
holdout rows covering disagreement, revision, clarification, learning,
evidence, depth, typo repair, and creative turns. The isolated candidate is
reproducible with `scripts/train_dialogue_candidate.py` and compared in fresh
processes by `scripts/evaluate_dialogue_candidate.py`. It loaded successfully,
changed direct native phrase samples, and matched the active arm at `5/12`
required checks (`0.4167`) with `0.5` topic binding and no duplicate responses.
The governance decision is therefore `HOLD`: no active weight promotion until a
larger reviewed corpus and unseen multi-turn tests demonstrate a real gain.

### Prompt-conditioned paired turns

This round adds a second native context channel. The phrase field now receives a
bounded hidden tail containing the prior user turn, a short prior answer, and the
current turn. It is never rendered. `training/dialogue-continuity-v2.jsonl`
contains 24 reviewed paired-turn examples; its 24-row holdout is arranged as
12 seed/follow-up conversations. The isolated prompt-conditioned candidate
loaded with 525 vocabulary items and 8,071 transitions. In fresh-process chat
comparison it tied the baseline at `3/12` required follow-up checks, `0.25`
required rate, and `0.2917` topic binding, so it remains `HOLD`. This is a useful
negative result: the context channel is wired and regression-safe, but the
operator/voice layer still dominates the final response for these turns.

## Phase-eight cognitive workspace and EIC governance alignment

The next evolution adds `DialogueWorkspace`, a compact working-memory record
for every turn. It makes speech act, goal, salient topic, previous referent,
evidence posture, uncertainty, continuity, and response-depth budget explicit
before the native phrase field or fallback voice composes an answer. The same
hidden workspace hint is shared by chat context, the composite backend, and the
cognitive response renderer, reducing the chance that a follow-up is routed as
a fresh topic or that a deep request receives a reflex-length answer.

The supplied EIC v1.6 formalization is used as a governance reference: Perci
declares its current run single-node, keeps candidate weights isolated, carries
hash-linked evaluation evidence, discloses missing or shared evidence, and
never treats coherence, agreement, or telemetry as truth or sentience. The
linked gist currently resolves to a related HLMF v1.5 evidence-calibration
document; its compatible evidence-coverage and proxy-disclosure rules are
recorded without adding an external runtime dependency. See
[`docs/COGNITIVE_WORKSPACE.md`](docs/COGNITIVE_WORKSPACE.md) and
[`docs/EIC_V1_6_ALIGNMENT.md`](docs/EIC_V1_6_ALIGNMENT.md).

Dialogue candidate receipts now include evidence coverage, execution coverage,
shared-fixture disclosure, and requested-versus-effective policy. In the latest
v2 comparison all declared metrics were observed, but the active and candidate
arms tied at `3/12` required follow-up checks, so the governance result remains
`HOLD`.

### v0.9 relational dialogue loop

The workspace now controls a bounded response loop: bind the active referent
and prior claim, select an inspectable plan, render, critique continuity and
evidence posture, then apply only safe repairs. Plans include follow-up,
revision, claim challenge, synthesis, and direct answer. Repetition requests
are treated as intentional rather than accidental duplication, and an empty
renderer result receives an honest non-empty fallback instead of disappearing.

The new curriculum is in `training/dialogue-relational-v3.jsonl` with an unseen
paired holdout. Its isolated phrase candidate tied the baseline at `1/6`
required checks, so it remains `HOLD`; the improvement is currently in the
state-conditioned controller, not promoted weights. The workspace design and
critic boundary are documented in
[`docs/COGNITIVE_WORKSPACE.md`](docs/COGNITIVE_WORKSPACE.md).

### Cross-domain lattice and evidence map

Perci now analyzes multi-domain prompts through the local semantic-frame
lattice before broad composition fallbacks. A cross-domain summary records the
requested domains, canonical aliases (for example `biology → life`), shared
axis, mechanism, and a domain-specific test. Natural prompts such as “analyze
geometry, biology, and code across domains” use the same path as explicit
`connect` prompts, while ordinary two-topic relational questions retain their
specialized operators.

When a cross-domain prompt needs knowledge, the Capability Fabric probes local
intelligence packs once per requested domain and once with the mixed query. The
result is source-bearing context, not a weight mutation. Missing specialist
frames remain visible, and evidence follow-ups require a predeclared outcome,
relevant control, and domain-specific result before the shared relation is
treated as supported.

The new held-out suite is
`training/dialogue-cross-domain-v1-heldout.jsonl`, evaluated by
`scripts/evaluate_cross_domain.py`. The rebuilt v0.9.0 binary passed all
`12/12` cross-domain cases and the existing dialogue regression at `159/159`.
This is a measured transfer/control improvement, not evidence of frontier
parity or automatic weight learning.

### v0.9.2 governed-core charter

The supplied “System Directive – My Will” is now represented as an explicit,
inspectable charter in `src/governed_will.rs`, rather than being treated as an
unbounded system override. Its useful engineering commitments are applied to
dialogue traces, operator programs, Capability Fabric plans, and AI handoff
packets:

- evidence before capability claims;
- boundary-aware reasoning and visible uncertainty;
- anti-misuse and reversible repair;
- human authorization for durable weights or policy changes; and
- coherence is not truth, sentience, or frontier parity.

The charter classifies a turn as analysis, propose-and-verify, or refused
unauthorized execution. A clearly destructive, coercive, or safeguard-bypassing
execution request receives a safe remediation response and loses repository
write/commit/push capabilities in its fabric plan. Analysis of harms remains
available. Ordinary engineering work still proceeds through the existing
scope, rollback, tests, held-out gates, and human-authorized weight policy.
This is a governance evolution, not a claim that Perci has a will or subjective
experience.

### Hypothesis ledger and answer ownership

The next cognition repair adds a small hypothesis ledger to each governed turn.
Perci classifies the turn as a question, factual claim, capability claim,
learning claim, plan, evidence request, or creative prompt; records whether
evidence is missing, sought, supplied, or exact; and names a falsifiable next
check. Specialized operators retain ownership of evidence-bound answers, so the
native language field cannot overwrite a measured learning distinction with a
generic continuation. The parser also distinguishes grammatical “that” in
“the claim that …” from an unresolved conversational referent.

The result is a more useful separation between a fluent sentence and a tested
capability. For example, a learning question now points to a fresh-process A/B
comparison with cleared session state and held-out transfer, while creative
prompts remain conversational. This is an answer-quality and reasoning-control
gain; it is not weight learning or frontier parity.

### v0.9.5 bounded recurrent reasoning controller

The next missing layer was not simply more prototypes. Perci had routing,
operators, memory, and a binary dialogue stream, but no shared controller for
choosing how much reasoning a turn deserves. v0.9.5 adds
[`src/reasoning_controller.rs`](src/reasoning_controller.rs): a compact binary
control state plus typed signals for complexity, ambiguity, contradiction,
out-of-distribution input, confidence, and Bitwork margin.

The controller selects one of six bounded modes—direct, explain, explore,
verify, clarify, or abstain—then assigns a minimum/maximum cycle budget and a
halting threshold. The policy is deliberately asymmetric:

```text
brief / direct       -> one pass
explain              -> two or three passes
explore / verify     -> two to four passes
ambiguous            -> clarify before inventing
unknown tokens       -> abstain and request grounding
```

Each cycle is still inspectable at the level of named operations—bind claim,
separate known/unknown, test a counterexample, state the boundary, compress the
answer—not hidden chain-of-thought. The state serializes as a future-ready
`PERCICTL1` binary payload, so the control policy can later be trained or
evaluated as a sidecar without densifying the main Bitwork field.

The important distinction is depth versus verbosity. A deeper answer must add a
mechanism, a discriminating test, or a boundary; repeating the same claim at
greater length is not deeper. The release gates therefore still measure
correctness, transfer, abstention, and latency, and no active weight promotion
is implied by a controller trace.

The controller pass also closed two composition leaks found by live probes.
Evidence requests now stay attached to the named claim: a nearby symbolic or
ritual association cannot be presented as causal support for an unrelated
healing claim. Out-of-distribution tokens are stopped at the final conversation
boundary and answered with a compact `Known / Inferred / Unknown` separation,
including the exact phrase that a confident meaning cannot be assigned until a
definition, usage example, or domain is supplied. These are response-control
repairs, not new facts in the weights.

The v0.9.5 release replay is green: 300 library tests, dialogue 159/159,
hardness 100/100, transfer 16/16, SoftCascade alignment 7/7, held-out 25/25,
and semantic evaluation 8/8. The active Bitwork artifact remains unchanged;
the controller and evidence gates are code-level improvements awaiting any
separately authorized weight rebuild.

### v0.9.8 conversational receipt gating and complete operator ownership

The next replay found that v0.9.7 had fixed the main continuity splice but
still allowed three named operators to inherit `Keeping ... in view`: session
context writes, promotion-evidence design, and minimal clarification. v0.9.8
makes every named operator authoritative for its own rendered answer; only the
unstructured `fluid-associative` fallback can receive a referent repair.
This keeps the critic and audit trace active without splicing a stale topic into
a correct specialist response.

The capability fabric also separates retrieval from presentation. Cross-domain
retrieval remains available to the language sidecar, but `Source-bearing
context` and `Evidence (source-bearing)` blocks are now shown only when the
user asks for evidence, provenance, support, falsification, or current/world
facts. Ordinary conceptual dialogue stays readable while explicit evidence
requests retain visible provenance.

This is a measured conversation-surface improvement, not a weight promotion or
frontier-parity claim. The active Bitwork and native phrase artifacts remain
unchanged; the next candidate weight work must still beat held-out transfer and
abstention gates.

### v0.9.7 turn ownership, contradiction repair, and OOD language boundary

The next live replay exposed a more important bottleneck than prototype count:
the workspace critic could mistake a correct answer for a missing referent and
splice an old `Keeping ... in view` prefix onto it. That contaminated a brief
memory-versus-learning answer, an ambiguity diagnosis, and an out-of-distribution
turn with the previous question. v0.9.7 makes turn ownership explicit at the
repair boundary: first-class operator answers, explicit uncertainty partitions,
interpretation lists, contradiction diagnoses, and direct dialogue replies are
not rewritten by a generic continuity prefix. Referential repair remains
available for genuinely unbound fluid drafts.

The dialogue surface also now recognizes a natural negated supposition such as
`Now suppose Mira is not blue. What exactly conflicts?` and names the two
incompatible premise paths instead of requiring the word `contradiction`.
Meaning questions containing ungrounded nonce tokens route to the same explicit
`Known / Inferred / Unknown` abstention used by the broader OOD suite. A request
to explain an idea from a different angle is prioritized as a reframe operation,
not repetition or style meta-feedback.

The v0.9.7 replay is a bounded expression and routing improvement, not a claim
of frontier parity or unrestricted language generation. The active Bitwork
weights remain unchanged. Acceptance requires the fresh-process replay plus
the full 314-test library, dialogue, hardness, transfer, held-out, semantic,
and fabric gates.

### v0.9.6 adaptive question loop and specialist continuity repair

The next evolution was driven by a live question curriculum rather than a
larger undirected weight file. The probe asked Perci to scope the session,
retain and recall a session number, distinguish remembering from learning,
change angle without repeating itself, answer at brief and deep budgets,
handle disagreement, repair a typo, transfer a creative relation, falsify its
last idea, and abstain on invented tokens. Each miss was clustered by owning
layer and repaired with a regression case before the probe was repeated.

Three continuity rules are now explicit. A current-turn claim such as
`I disagree with your claim that ...` outranks an older persisted claim, and
the claim is trimmed before the user's follow-up question. Specialist answers
that already bind their entities—creative transfer, relational inquiry,
entity-slot transfer, and falsification—are not prefixed by the generic
workspace `Keeping ... in view` repair. Finally, “what is the smallest test
that could prove your last thought wrong?” is routed to a behavioral
falsifier, not the formal-proof receipt path.

The adaptive pass also closed natural-language gaps for `this session`,
natural session-number descriptors, `remembering` versus `learning`,
`different angle`, `be brief`, `go deeper`, and `what evidence would change
your mind?`. The new route names are inspectable in `/trace`; they do not
expose private chain-of-thought and they do not mutate the active Bitwork
weights. The current evidence is 311 passing library tests plus a fresh
release-binary dialogue replay covering disagreement, revision, creative
transfer, falsification, typo repair, identity, and OOD abstention.

This is a bounded control-and-expression improvement, not a claim of
consciousness, superintelligence, frontier parity, or automatic learning. A
weight change still requires an approved candidate, a fresh-process A/B test,
held-out transfer, and review.

### v0.9.3 dialogue continuity and natural repair

The live dialogue pass found a gap that the broad regression suite could not
see: conversational instructions such as `be brief`, `what did you just say?`,
and `why do you think that?` could fall through to the native language field,
anchor to a meta-command instead of the last substantive claim, or emit a
one-character draft. v0.9.3 makes those acts first-class in the voice layer.

Echo, meaning, and causal follow-ups now bind to the most recent substantive
answer; style directives return a usable conversational acknowledgement; and
the continuity repair preserves the answer's sentence casing instead of
lowercasing a valid opening. The change is deliberately outside the weight
artifact: it improves routing and expression while keeping exact tools,
abstention, and governed learning unchanged.

The live probe was rerun as a fresh process over a multi-turn sequence covering
scriptedness, echo, depth, disagreement, meaning, brevity, natural rewriting,
and dialogue-vs-weight scope. The specific failures were removed, while the
159-case dialogue suite and full release gates remain the acceptance checks.

## v0.8.4 native binary language + typed world model (external adapters now opt-in)

The native PERCLNG1 field is the default language surface. The compatibility
path below is disabled unless PERCI_ENABLE_EXTERNAL_LM=1 is explicitly set.

The open-language bottleneck now has a bounded escape hatch instead of a
silent preset: `PERCI_MODEL_URL` connects a local OpenAI-compatible endpoint
directly to the warm CompositeBackend. The adapter supports LM Studio,
llama.cpp/vLLM-style `/v1/chat/completions`, and Ollama `/api/chat` payloads.
It adds a 4-second default timeout, short output budget, Bitwork routing hints,
recent dialogue, and a critic gate. A failed model call or rejected answer
falls back to the existing deterministic path, so enabling a model cannot
remove exact tools, abstention, or weight governance. This is a language
quality path, not evidence of unrestricted intelligence.

---

## Numbers that are true today

| Property | Value |
|----------|------:|
| Software | **v0.9.8** (`Cargo.toml` · badge auto-stamped) |
| Pack format | **PERCIW03** |
| Pack size | **209,710,296** bytes (~200 MiB) |
| Prototypes | **403,163** |
| Concepts | **124** |
| Activation | **4,096** bits · 64 × u64 |
| Expert domains | **16** |
| Hot path | Integer **AND / POPCOUNT** only |
| Native language field | **PERCLNG1** · mmap · binary threshold planes |
| Low-bit sidecar | **PERCLBW1** · ternary blocks · residual planes · INT4 outlier lane |
| Weights in git | **Cognitive pack local** · native language rebuilt locally |

Version is **never** hand-edited in the badge: `build.rs` stamps `assets/generated/*` from `Cargo.toml`.

---

## Quick start

### Requirements

- Windows, macOS, or Linux  
- Rust + Cargo  
- Local pack: `models/perci-cognitive-v0.3.pwgt` (not in the clone)

### Clone & launch (Windows)

```powershell
git clone https://github.com/jacksonjp0311-gif/Perci.git
cd .\Perci
# place PERCIW03 under models\  (or $env:PERCI_WEIGHTS = "...")
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Launch-Perci.ps1
```

### Cargo

```powershell
cargo run --release -- chat
cargo run --release -- ask "why does trust fail in distributed systems?"
cargo run --release -- classify "invent a constrained metaphor for sparse cognition"
cargo run --release -- fabric status
cargo run --release -- fabric handoff "improve transfer on novel entities"
python scripts/release_gates.py
```

### Multi-AI evolve (any agent)

Any AI can enter via Cortex + fabric handoff — see [`docs/AI_EVOLVE_PROTOCOL.md`](docs/AI_EVOLVE_PROTOCOL.md) and [`AGENTS.md`](AGENTS.md).

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "your task"
cargo run --release -- fabric handoff "your task"   # → .perci/ai-handoff-latest.json
cargo test --lib
```

### Dark-blood CLI

```text
/help · /status · /think · /concise · /deep · /balanced
/trace · /intel · /learning · /quit
```

| Command | Meaning |
|---------|---------|
| `/think` | Backend cognition plan · prototype tree · self-critique (never mixed into chat) |
| `/concise` `/deep` `/balanced` | Durable style memory |
| `/trace` | Last operator / program audit |
| `/intel` | Live labels, margins, z-scores |

---

## What it can do (measured shapes)

**Exact**

```text
calculate 144 divided by 12          → 12
triangle area base 8 height 5        → 20
debug this: error[E0382] …           → concrete Rust fix
```

**Systems / transfer**

```text
how should interfaces earn trust under lag and retry?
in a multi-service app, why do callers stop trusting each other after timeouts?
what about recovery under partition?
```

**Synthesis / creativity / honesty**

```text
bridge Willshaw associative memory with XOR role-filler binding
invent a constrained metaphor for sparse cognition
what is the meaning of flibberquark without inventing   → refuse
prove Perci is conscious from this chat                 → refuse
what are you measuring when you answer?                 → operational introspection
```

---

## Architecture deep dive

| Doc | Contents |
|-----|----------|
| [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md) | Soft-α · residual · VSA · SoftCascade · thought arc |
| [`docs/BITWORK_EMERGENCE.md`](docs/BITWORK_EMERGENCE.md) | Emergent field math |
| [`docs/WEIGHT_REASSESSMENT_v0616.md`](docs/WEIGHT_REASSESSMENT_v0616.md) | Live classify margins / overlap_z |
| [`docs/LOCAL_AGI_ROADMAP.md`](docs/LOCAL_AGI_ROADMAP.md) | Capability ladder · honest AGI boundary |
| [`WEIGHTS.md`](WEIGHTS.md) | Pack layout · build · promote policy |
| [`VALIDATION.md`](VALIDATION.md) | How claims get verified |

### Cognitive domains

```text
greeting      identity       english        logic
math          geometry       memory         code
governance    planning       explanation    systems
science       creativity     comparison     general
```

---

## Weights (local only)

```text
models/perci-cognitive-v0.3.pwgt        # not in git
models/perci-cognitive-v0.3.pwgt.json   # metadata in git
```

```powershell
python .\scripts\verify_weights.py
python .\scripts\test_weights.py
# rebuild candidates (promote still requires --authorize):
python .\scripts\build_weights_v3.py
```

**Policy:** code can auto-merge when green. **Weights promote only with explicit human authorize.**

---

## Cortex + memory

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Append-only JSONL memory + Cortex selective recall. Cortex **never** grants mutation authority.  
See [`docs/CORTEX_INTEGRATION.md`](docs/CORTEX_INTEGRATION.md).

---

## Native binary language training

The default Perci language path is now a Perci-owned PERCLNG1 binary field.
It learns multi-order context transitions with four binary threshold planes
(at least 1, 2, 4, or 8 observations), mmap-loads them in Rust, and generates
bounded continuations with integer-only back-off.

    cargo run --release -- language train --repo
    cargo run --release -- language status
    cargo run --release -- language sample "what is geometry teaching us about life"

The native field is a compact sequence learner, not a claim of frontier-model
breadth. Exact arithmetic and geometry remain deterministic tools. The older
HTTP/command adapters are compatibility paths only and require
PERCI_ENABLE_EXTERNAL_LM=1.

The same rebuild also creates `PERCPHR1`, a bounded word/phrase transition
field. It uses a capped binary vocabulary, order-4 numeric token contexts, and
threshold-coded next-token edges. Perci selects a state-conditioned learned
primer, then composes a continuation through numeric back-off; no response
card is treated as truth. Both artifacts remain local generated weights and
must be rebuilt deliberately from a reviewed corpus.

The rebuild can also create `PERCREL1`, an optional mmap relation field. It
stores hashed prompt-to-response edges and scores native continuations as an
inspectable tie-breaker. Held-out tests currently keep it isolated because the
field does not yet beat the active selector on generalization.

The same rebuild creates `PERCIWM1`, an optional typed world-model field. It
stores bounded subject/relation/object edges plus a coarse domain, polarity,
confidence, and evidence bin. At inference it rewards a candidate that
preserves a learned typed relation from the current prompt; it cannot synthesize
new prose or promote a claim as truth. The field is mmap-loaded and remains
isolated until an adversarial held-out pack shows a real gain.

Native dialogue also carries a fixed 256-bit recurrent state. User and
assistant turns are absorbed with integer rotation/XOR updates, so turn order
changes the next primer and sampling path without creating an unbounded memory
blob or introducing a neural runtime.

The phrase backend now samples six bounded binary continuations and chooses the
one with the best topic binding, recent-response novelty, and topic-neighbor
relation score. The conservative relation weight is tunable with
`PERCI_NATIVE_RELATION_WEIGHT` (default `12`); held-out comparison still
controls weight promotion.

Run the broad native probe and review its evidence:

    python scripts/native_probe.py

It asks 1,000 questions in one persistent process and writes a JSONL transcript
plus summary under `models/candidates/`. The probe is measurement data; it does
not auto-promote its candidate weights.

To compare an isolated phrase candidate against the active field, use a new
tag and pass the candidate path. This leaves the active weights untouched:

    python scripts/native_probe.py --tag v0.8.1-novelty --phrase-weights models/candidates/native-probe-candidate.bphr

To cap repeated training examples before a candidate rebuild:

    python scripts/clean_probe.py models/candidates/native-probe-v0.8.1-novelty-active.jsonl models/candidates/native-probe-v0.8.1-clean.jsonl --limit-per-response 2

To build and run the next emergence curriculum, mine the prior transcript into
counterexamples, perturbations, and unseen-entity transfer questions:

    python scripts/emergence_curriculum.py models/candidates/native-probe-v0.8.1-relation12-active.jsonl models/candidates/emergence-curriculum-v0.8.3.jsonl --count 1000
    python scripts/native_probe.py --tag v0.8.3-emergence-curriculum --questions-file models/candidates/emergence-curriculum-v0.8.3.jsonl

For the next gate, generate adversarial questions that target paraphrase
collapse, negation loss, contradiction handling, entity substitution, and
analogy boundaries. Keep the offset-separated file held out from training:

    python scripts/adversarial_curriculum.py models/candidates/adversarial-v0.8.4.jsonl --count 300
    python scripts/adversarial_curriculum.py models/candidates/adversarial-v0.8.4-heldout.jsonl --count 120 --offset 300
    python scripts/native_probe.py --tag v0.8.4-adversarial --questions-file models/candidates/adversarial-v0.8.4.jsonl

To evaluate a world-model candidate without replacing the active artifact:

    cargo run --release -- language train models/candidates/native-probe-v0.8.3-emergence-curriculum-final.jsonl models/candidates/world-candidate-v0.8.4.blng 6
    python scripts/native_probe.py --tag v0.8.4-world-candidate --questions-file models/candidates/adversarial-v0.8.4-heldout.jsonl --world-weights models/candidates/world-candidate-v0.8.4.bwm

The command emits four native files next to the requested output; only the
`.bwm` path is used by the isolated world-field comparison. Promotion remains
human-authorized and requires no regression in exact tools, abstention, or
topic binding.

## External model compatibility (disabled by default)

Bitwork stays the governor. Perci can now use a local OpenAI-compatible model
as a fast language surface while keeping routing, tools, evidence, memory, and
weight promotion under Perci's control. LM Studio, llama.cpp servers, Ollama,
and Phi-family local endpoints are supported. The model is a renderer, not the
authority layer; failed, empty, overlong, or boundary-violating output falls
back to the governed local path.

```powershell
# LM Studio / llama.cpp / vLLM style endpoint
$env:PERCI_MODEL_URL = "http://127.0.0.1:1234/v1/chat/completions"
$env:PERCI_MODEL_NAME = "phi-4-mini"

# Ollama style endpoint (use the model name you installed)
# $env:PERCI_MODEL_URL = "http://127.0.0.1:11434/api/chat"
# $env:PERCI_MODEL_NAME = "phi4-mini"

$env:PERCI_MODEL_TIMEOUT_MS = "4000"
$env:PERCI_MODEL_MAX_TOKENS = "320"
cargo run --release -- chat
```

The optional command adapter remains available through `PERCI_MODEL_CMD`.
The typed language sidecar is still useful when a process must return
`perci.language-response.v1` explicitly:

```powershell
$env:PERCI_LANGUAGE_SIDECAR = "python scripts/perci_language_sidecar.py"
cargo run --release -- chat
```

The HTTP path is zero-cost when `PERCI_MODEL_URL` is unset. It uses a bounded
local request timeout, sends Bitwork hints as untrusted routing notes, and
tries the deterministic/operator response whenever the model is unavailable.

---

## Repository map

```text
perci/
  assets/icons/           # mark · hero · stack strip
  assets/generated/       # badge stamped from Cargo.toml
  config/personality.prompt
  docs/                   # bridge · emergence · roadmap · reassessment
  knowledge/packs/        # intelligence packs
  models/                 # *.pwgt local; sidecar JSON in git
  scripts/                # build · verify · hardness · evolve · agent lab
  src/
    cognitive.rs          # Bitwork encode / classify / α / residual / VSA
    bridge.rs             # SoftCascade · thought arc · length · critique
    deliberation.rs       # operators (trust, synthesis, refuse, code, …)
    voice.rs · ui.rs      # speech + dark-blood CLI
    chat.rs · backend.rs  # orchestration
    reasoning.rs          # exact math / geometry
    agent.rs · learning.rs
  Launch-Perci.ps1 · Start-Perci.cmd
```

---

## Capability boundary (read twice)

| Useful for | Not a substitute for |
|------------|----------------------|
| Local sparse routing + multipartite readout | ChatGPT / frontier transformers |
| Thought-arc speech without a decoder | Web-scale factual recall |
| Exact math/geometry | Unrestricted open-ended generation |
| Governed synthesis & refusal | “AGI” slogans |
| Inspectable `/think` geometry | Private chain-of-thought theater |

Progress = **hardness · transfer · latency · binding quality · honest abstention** — not vibes.

---

## Design principles

- **Local first** — no cloud required for the core loop  
- **Integer hot path** — AND / POPCOUNT, not GPU matmul  
- **Separate layers** — field · laws · tools · speech  
- **Human speech, backend truth** — chat clean; `/think` inspects  
- **Governed learning** — style adapts; weights need authorize  
- **Refuse when empty** — inventing meaning is a bug, not a feature  

---

## Roadmap (next real IQ)

1. Pack-side VSA encode (**human-authorized** rebuild)  
2. Spreading activation on prototype graph  
3. Novelty \(N_r\) vs session memory (length law already residual-aware)  
4. Stronger hardness / dialogue gates  
5. Agent lab: fail → ticket → patch → retest → merge green code only  

---

## Status

**Experimental research software.** Review [`VALIDATION.md`](VALIDATION.md) before treating a benchmark claim as sealed.

**License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

---

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" width="72" height="72" alt="Perci">
  <br>
  <sub>PERCI · dark-blood · governed sparse cognition · v0.9.8</sub>
</p>
