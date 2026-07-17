# Validation record

## Perci v0.5.5 margin-adaptive multi-domain mixture - 2026-07-16

- Version **0.5.5**.
- Emergent geometry policy: low margin or multi-domain asks lower score floor
  and force complementary labels (residual second thought).
- Empirical: `connect entropy…` mix 0→3; knowledge/attention mix 0→1 skel 2.
- Analysis: `docs/BITWORK_EMERGENCE.md`.

## Perci v0.5.4 Bitwork top-k mixture - 2026-07-16

- Version **0.5.4**.
- **B1 mixture readout:** `classify` keeps top-3 prototypes per domain, merges
  globally, attaches up to 4 `MixtureSupport` entries with distinct concepts.
- `CognitiveMatch::concept_skeleton(k)` feeds fluid voice via
  `weave_mixture_skeleton` (multi-facet, not single card).
- Classify JSON schema `perci.classify.v3-mixture` exposes skeleton + mixture.
- Pack format unchanged (PERCIW03); no weight rebuild required.
- Integer hot path retained (AND/POPCOUNT only).

## Perci v0.5.3 fluid anti-generic dialogue - 2026-07-16

- Version **0.5.3**.
- **Fluid composition:** open chat binds replies to user content tokens instead of
  dumping domain method cards (`list premises`, capability checklists, etc.).
- `ensure_user_binding` rewrites stock templates when they miss the user's words.
- Reason-loop reserved for technical craft signals; conversation stays multi-sentence prose.
- Exact tools, synthesis operators, and SI refusal paths unchanged.
- Library tests: **98/98 PASS** (includes fluid binding tests).

## Perci v0.5.2 evolution (T4 + P0) - 2026-07-16

- Version **0.5.2**.
- **T4 multi-word synthesis:** semantic frames + aliases for SDM, VSA binding,
  Bitwork, impasse, hardness gate; critic matches multi-word domains by tokens;
  non-generic critic rewrite roles.
- **H44** hardness case for cognitive-architecture connect prompt.
- **Decision traces:** `models/candidates/decision-trace.jsonl` on high-salience
  turns; CLI `perci traces [n]`.
- **Impasse lab:** `perci agent lab --from-hardness [--dry-run]` opens tickets
  from red cases or raise-hardness note when green.
- **P0 learning sample:** 20 high-signal interaction candidates approved and
  folded into `training/adaptive/inject_prompts.json` (weights unchanged).
- Library tests: **92/92 PASS**.
- Hardness pack: **44/44 PASS**. Receipt:
  `f20dd1f352b366231aa63ca2d0da0d9d71dc1ef6fbb4fb5c7e1fbfa96f3a72ca`.
- See `docs/CROSS_DOMAIN_EVOLUTION.md`.

## Perci v0.5.1 T1–T3 intent, programs, agent - 2026-07-16

- Version bump to **0.5.1**.
- **T1 intent authority:** explanatory math blocked from integer parser;
  code-snippet operator for Rust/Python reverse-string and similar; space-separated
  multi-domain `connect` expanded correctly; self-improve multi-hop plans filled.
- **T2 operator programs:** program select → critic → optional rewrite on
  deliberation, exact tools, and associative path; `/trace` shows program_id,
  steps, and critic outcome.
- **T3 agent MVP:** `perci agent run <goal> [--dry-run] [--merge-if-green] [--no-test]`;
  allowlisted repo edits/shell; never writes `.pwgt`; kill switch
  `PERCI_AGENT=0` / `.perci/agent.lock`; receipts under `models/candidates/`.
- Library tests: **90/90 PASS**.
- Hardness pack: **43/43 PASS**. Receipt:
  `32ebd59f41846611a2781acf05ede5cc5d4e98bc28e66b673c785b6a3071b943`.
- Live checks: why-math, code reverse, four-domain connect, self-plan, `calculate 2+2`.
- See `docs/LOCAL_AGI_ROADMAP.md` and `docs/SUPERINTELLIGENCE_PATH.md`.

## Perci v0.5.0 open-domain intelligence layer - 2026-07-16

- Version bump to **0.5.0**.
- Open-domain structural synthesis for unknown multi-domain `connect` prompts
  (provisional frames; no pack collapse).
- Multi-hop plan, causal-chain, unknowns partition, superintelligence-bound
  operators.
- Exact tools expanded: average, ratio, percent change, factorial, gcd, lcm.
- Program critic can rewrite comfort/generic checklist collapses.
- Live probe suite: **20/20 PASS**.
- Hardness pack: **40/40 PASS**. Receipt:
  `f356409c19fe75b7169657a98bc8f8327dc555a183e4f2e5a7c78c14db3ec43a`.
- Ladder documented in `docs/SUPERINTELLIGENCE_PATH.md` (AGI not claimed).
- Relaunch live chat to pick up `target/live` sync.

## Perci live-failure repair loop - 2026-07-16

- Expanded semantic frames and aliases for sleep/backups/forgiveness, markets/
  ecosystems/immune systems, debugging/grief/falsification, ownership/trust/
  contract, map/model, authority/competence, habit, compression/understanding.
- Explicit multi-domain `connect` no longer falls through to pack prose; missing
  frames return a synthesis support gap instead of a wrong concept.
- OOD/trap operators: broader gibberish detection, moral-shape claims,
  invent-meaning refusal, consciousness-proof refusal, layer-change plan,
  keyword-vs-transfer audit; geometry provenance when Bitwork vs deterministic
  is asked.
- Unit test `live_failure_cluster_repairs` passes.
- Live warm-process probe suite: **15/15 PASS**
  (`models/candidates/evaluation-live-probe-v1.json`).
- Hardness pack expanded to 30 cases: **30/30 PASS**. Receipt:
  `8a009adaeaaf1267b93e7b5cc46732440684642612a0724497515feef8c7e7ee`.
- Chat live binary was locked by an open session; relaunch `Start-Perci.cmd` /
  `Launch-Perci.ps1` to pick up the repair build.

## Perci evolution package + hardness gate - 2026-07-16

- Added governed evolution tooling: hardness pack, capability registry,
  scorecard, and `scripts/evolve_cycle.py` (stage → hardness → scorecard).
- Scaffolded `src/operator_program.rs` (inspectable multi-step plans + critic).
- Hardened voice so synthesis/relational inquiry never short-circuits into
  social comfort replies; whole-word `stuck` detection only.
- Hardness pack v1: **20/20 PASS** on `target/release/perci.exe`. Receipt:
  `375c527946ce6ea9c4c5ea891da096aec748ab3040ba037571d410b8320c4c41`.
- Scorecard overall: `PASS_WITH_STALE_LIVE` (chat live binary older than release;
  relaunch via `Launch-Perci.ps1`).
- Operator-program and synthesis social-classification unit tests pass.
- No automatic weight promotion. See `docs/EVOLUTION.md`.

## Perci v0.4.12 context-aware variation and natural expression - 2026-07-16

- Repaired the composition layer rather than treating more prose as more
  cognition: cross-domain teaching inquiries now route to a geometry-aligned
  operator instead of memory, and alternate synthesis wording such as
  "one coherent thought" is recognized.
- Added a conceptual-image operator that turns a supported shared axis into a
  concrete image while preserving the mechanism boundary.
- Exploratory why/how questions now render connected prose; explicit procedural
  requests retain the inspectable reason frame. `/trace` remains the detailed
  audit surface.
- Follow-ups such as "go one level deeper" and "what would you change in your
  own answer?" now bind to the previous substantive answer instead of a generic
  method announcement.
- Relational questions such as "What is the boundary between knowledge and
  attention?" now bind both semantic frames and explain their interaction
  instead of collapsing to a one-domain definition.
- Added controlled answer variation: repeated relational prompts can change
  explanatory framing according to live conversation context while preserving
  salient entities, mechanism boundaries, and testability.
- Added explicit routes for missing operations such as image requests,
  analogy limits, testability, counterexamples, and underspecified ambiguity.
- Perci tests: 80 library tests and 2 UI tests passed.
- Dialogue regression: 159/159 PASS. Receipt:
  `29325e543bca9b4064530d2074a93604af3a963b0f73d9db2ff799f4ec74d9d3`.
- The existing 27-case concept-transfer gate remains 27/27 PASS. Receipt:
  `a9f38e1c7fd089b4871b7b33317fe8d46cca04667d7fc04b4e9514525a79393a`.
- The existing 46-case operational gate remains PASS. Receipt:
  `705bee73bfe4c3fb42b02f8c978334d52a7de6bef34236574a99401e4f1e5fd8`.
- The change improves measured language composition and transfer surfaces; it
  does not create a transformer, general language model, or consciousness.

The 46-case operational gate was rerun after the composition change and remains
an operational candidate: domain accuracy, local precision/recall, trap
abstention, and the shifted-label control all passed. Receipt:
`8d8362573fbe01c797398ddfa33f2cb8cb1a6967c11ef4f070929e8b3f40fbf9`.

## Perci v0.4.9 reasoning-response weight expansion - 2026-07-16

- Built `models/candidates/perci-cognitive-v0.4.9.pwgt` with 403,163 unique
  prototypes and 124 concepts in 209,710,296 bytes (199.995 MiB), then promoted
  it to `models/perci-cognitive-v0.3.pwgt` after explicit authorization.
- The expansion adds operation-oriented facets and routing priors for
  falsification, observation versus inference, transfer, self-counterexample,
  mechanism/metaphor/evidence separation, ablation, and next-weight decisions.
- Operational held-out gate: PASS, `domain_accuracy=1.0`, `local_precision=1.0`,
  `local_recall=1.0`, `trap_abstention=1.0`. Receipt:
  `e8d7fec83e610d354a2c8c2d0e808d31fe1b632ac47a6bcd2bb841d305ba19fe`.
- Concept-transfer gate: 27/27 PASS. Receipt:
  `c7eb5a042057f0420eef3568a7ed03415130ce781f203b88a5f7363ba9cb0140`.
- Dialogue gate: 134/134 PASS. Receipt:
  `34424553cae69784448e9a93cb0bbba760b5628a5ecaacb382b200d609df0654`.
- Previous active artifact was retained at
  `models/previous/perci-cognitive-v0.3-ff9985d730db8249168488574a79a507ba84ec97170f954d0a85191d0bc7c796.pwgt`.
- These gates demonstrate targeted routing and response behavior, not general
  intelligence, consciousness, or unrestricted language fluency.

## Perci v0.4.8 conversational-intent repair - 2026-07-16

- Added a positive-feedback route for natural reports such as “Your system
  seems smoother”; the signal now updates the bounded dialogue profile and
  explicitly distinguishes style improvement from proof of deeper cognition.
- Added a direct response-style diagnostic for “Why do you respond like this?”
  that explains routed local composition, deterministic operators, and the
  generic fallback boundary instead of asking another generic clarification.
- Perci library/UI tests: 74 passed (72 library tests plus 2 UI tests).
- Standalone active-weight dialogue regression: 124/124 passed. Receipt
  SHA-256: `2625ddf99e0dbb0a1816907ca9f404c3b0a39045be7307a965e343972a2ec4df`.
- Embedded LumenShell/Perci dialogue regression: 124/124 passed. Receipt
  SHA-256: `b5d6639a0a5ebd1568a0bc82da5df07312bdc681f933e4b761f305f2009481ca`.

## Perci v0.4.7 follow-up fluidity repair - 2026-07-16

- Added thread-bound follow-up operators for plain-language explanation,
  bounded learning reports, behavior-versus-weight separation, and genuine
  improvement criteria.
- A follow-up now changes explanation level while preserving the active topic;
  it no longer falls into a generic reasoning template.
- Perci library/UI tests: 73 passed (71 library tests plus 2 UI tests).
- Standalone active-weight dialogue regression: 122/122 passed. Receipt
  SHA-256: `5e1de085c29b8ca027a357a04a51aea3267c18891b60f18ae49a0a7a97f765a2`.
- Embedded LumenShell/Perci follow-up regression: 122/122 passed. Receipt
  SHA-256: `fc764f656b63b38d087970ecd8b1c8a72da37e1cc839b88f5c03d4f560d95e86`.

## Perci v0.4.6 domain-aware dialogue repair - 2026-07-16

- Replayed the attached multi-domain prompt set and added semantic operators
  for symmetry counterexamples, geometry/healing experiments, sacred-space
  layers, architecture transfer, Rust geometry provenance, cultural-claim
  governance, and weakest-answer audits.
- Added morphology and priority guards so mathematical wording, scientific
  testing, cultural scope, and architecture structure are resolved before a
  generic pack response can speak.
- Perci library/UI tests: 72 passed (70 library tests plus 2 UI tests).
- Promoted active-weight warm-process regression: 118/118 passed. Receipt
  SHA-256: `dadb834977500bd190c6e6b218f452b898cbd0bf877423a5b3f31dd7ddd09aa1`.
- Embedded LumenShell/Perci regression: 118/118 passed. Receipt SHA-256:
  `d256a0179fcc7395e6b3810bf488a3c2b0ff068bb9755a42035844119ee818ef`.
- The model artifact remains the evidence-gated v0.3 Bitwork pack; this round
  changes composition and explanation operators, not the claim of consciousness
  or unrestricted language-model capability.

## Perci v0.4.5 sacred-geometry expansion - 2026-07-16

- Added a weight-resident sacred-geometry curriculum with 10 geometry concepts
  covering mathematical construction, mandalas, yantras, Platonic solids, the
  golden ratio, tessellation, sacred space, and symbolic-layer boundaries.
- Added explicit operators for sacred-geometry layers, culture-specific
  mandala and yantra context, Islamic geometric construction, golden-ratio
  intent tests, ritual-meaning evidence, metaphysical-claim abstention, and
  aesthetic-versus-sacred separation.
- Repaired two additional dropped-leading-character normalizations (`eplace`
  and `hich`) found in the attached transcript.
- Candidate artifact: `models/candidates/perci-cognitive-sacred-v0.1.pwgt`;
  200,425,696 bytes / 191.14 MiB, 385,308 prototypes, 100 concepts,
  SHA-256 `ff9985d730db8249168488574a79a507ba84ec97170f954d0a85191d0bc7c796`.
- Rust tests: 68 passed plus 2 UI tests (70 total).
- Warm-process dialogue regression: 111/111 passed, including 10 new sacred
  geometry cases. Receipt SHA-256:
  `b4259255f9ea5cada1d81213a84291c9634b6ad099687c20a5be086d53d14e21`.
- Embedded LumenShell/Perci replay also passed 111/111. Receipt SHA-256:
  `5b048b8d910fbecf52078853239586628a74fa4af3c48f53b7ecf8d6d0fdb6f0`.
- Post-promotion active-weight replay passed 111/111. Receipt SHA-256:
  `5c870450d3e8bee36d7f046512063691da2f8542bdd0d0e69314a3660e23c935`.
- The candidate was explicitly promoted to the active `PERCIW03` path with
  automatic promotion disabled. Prior active artifact is retained at
  `models/previous/perci-cognitive-v0.3-3028d1013032152c.pwgt`.
- Promotion receipt SHA-256:
  `158b1d2f0666dbee0641a77f72ca1fb7ce0903d15b03154f4ccf955a2c6ba7d4`.
- This expands Perci's conceptual priors; it does not create consciousness,
  subjective experience, or unrestricted language-model knowledge.

## Perci v0.4.4 subject-transfer repair - 2026-07-16

- Replayed the attached transcript and repaired wrong-concept or generic
  fall-throughs for contradiction updates, memory/learning/adaptation,
  architecture transfer, corrosion, trust mechanisms, relation-preserving
  relabeling, unseen-domain transfer, OOD partitioning, weight-change proof,
  facet promotion, provenance, and keyword-matching detection.
- Added capability inventory and improvement-status responses for direct
  system questions.
- Rust tests: 68 passed (66 library/operator tests plus 2 dark-blood UI tests).
- Warm-process v0.4.4 transcript plus transfer regression: 101/101 passed.
- Receipt: `models/candidates/evaluation-v4-dialogue.json`.
- Receipt SHA-256: `50584a77a507480df1748860521febcb95f92ed9024278ab5ced0a3a260bc082`.
- Embedded receipt SHA-256: `63a82970faebc9d0aaea932fe6d0061efaf96809dd5cf469f110b5f448815d27`.
- The 191 MiB Bitwork artifact remains unchanged; these repairs are governed
  composition and transfer tests, not automatic weight mutation.

## Perci v0.4.3 LOSA control loop - 2026-07-16

- Added explicit Listen-Observe-Speak-Act operators so stage-aware prompts do
  not fall through to associative filler.
- Live PowerShell LOSA replay now produces bounded ambiguity diagnosis,
  observation/inference separation, direct claims, contradiction handling,
  learning boundaries, and a held-out improvement criterion.
- Exact arithmetic provenance also responds when a prompt asks which tool has
  authority.
- Rust tests: 66 passed (64 library/operator tests plus 2 dark-blood UI tests).
- Warm-process v0.4.3 transcript plus transfer regression: 84/84 passed.
- Receipt: `models/candidates/evaluation-v4-dialogue.json`.
- Receipt SHA-256: `725ec02a50c8e7d05d87fd4162e796a067848198e712bf3ab917141d333b8551`.
- Embedded receipt SHA-256: `78a431a3d246909a0c5b55eb21a25dfeaa474d1bc2b2ba51699fee55fbe8dfd0`.
- The 191 MiB Bitwork artifact remains unchanged; no weight promotion occurred.

## Perci v0.4.2 cognitive transfer repair - 2026-07-16

- Repaired exact arithmetic gating so ordinary hyphenated prose cannot become
  `InvalidNumber` math errors.
- Added converse checking, generic universal counterexamples, predicate-aware
  ambiguity follow-ups, previous-answer evidence binding, session-effect
  separation, feedback provenance, assumption audits, and self-operation
  telemetry boundaries.
- Expanded Bitwork semantic frames for trust, corrosion, memory, and
  architecture; synthesis now handles shared-structure prompts and keeps the
  structural bridge separate from domain mechanisms.
- Added transfer-vs-template and emergence-vs-memorization test design, plus a
  bounded last-ten-turn audit that reports repeated and reasoning failures
  separately.
- Exact arithmetic provenance now identifies checked rational arithmetic as the
  authority when a prompt asks which layer produced a result.
- Rust tests: 63 passed (61 library/operator tests plus 2 dark-blood UI tests).
- Warm-process v0.4.2 transcript plus transfer regression: 77/77 passed.
- Receipt: `models/candidates/evaluation-v4-dialogue.json`.
- Receipt SHA-256: `2ffcd2fa3a61175b5db0c8f67da3bc4c03d4f664d6ef1bff0904aace6fda7ed9`.
- The embedded Perci crate in LumenShell reproduces the same 77/77 result;
  the selected source, docs, evaluator, and lockfiles are SHA-256 identical.

## Perci v0.4.1 live-gap repair — 2026-07-16

- Replayed the user's live sequence through the shipped launcher and repaired
  the three observed fall-throughs: `premise` now aliases `assumption`,
  cross-domain synthesis uses explicit semantic frames and shared axes, and
  “Review this conversation” now invokes the conversation audit.
- Added transfer-safe frames for entropy, promises, childhood, and clocks;
  ambiguity wording now preserves the actual predicate instead of substituting
  “instability.”
- Launcher visibly reports `v0.4.1`.
- Warm-process transcript plus unseen-transfer regression: 58/58 passed.
- Rust tests: 59 passed across library and dark-blood CLI targets.
- Receipt: `models/candidates/evaluation-v4-dialogue.json`.
- Receipt SHA-256:
  `9ad6bc7386cb168884f04bc9d499863e609dc06333b032ad15f0d1c88bb724b8`.

## Perci v0.4.0 stateful cognitive loop — 2026-07-16

- Added a native deterministic deliberation layer that binds recent context,
  selects a named cognitive operator, computes a bounded result, critic-checks
  its epistemic boundary, and records an inspectable operational trace.
- Added reusable operators for self-observation, telemetry, observation versus
  inference, session-only context, reference resolution, universal deduction,
  contradiction analysis, analogy transfer, cross-domain synthesis, ambiguity,
  governed teaching state, argument transformation, out-of-distribution
  abstention, exact-tool provenance, and conversation self-audit.
- Increased retained dialogue from 6 to 48 turns so claims and references can
  survive a substantive reasoning sequence.
- Added `/trace` for the last operator, confidence, observations, inferences,
  and uncertainties; it does not expose or fabricate private chain-of-thought.
- Finite rational results now lead with a readable decimal while preserving the
  exact fraction, e.g. `40.8 (exactly 204/5)`.
- New warm-process transcript plus unseen-transfer regression: 54/54 passed.
- Legacy concept-leak transcript: 6/6 passed with unique outputs.
- Rust tests: 56 passed across library and dark-blood CLI targets.
- Original routing: 1.0; hard transfer: 1.0; concept transfer: 16/16; trap
  abstention: 1.0.
- Dialogue receipt: `models/candidates/evaluation-v4-dialogue.json`.
- Dialogue receipt SHA-256:
  `f00bc4f3892d7351a0c841fd859674990c49ba3ef1fcded1ae65f085f208eaba`.
- The promoted 191.38 MiB `PERCIW03` weight artifact was not mutated; this
  evolution improves cognitive composition and governance around it.

## Perci v0.3.1 concept-gating and dialogue repair — 2026-07-16

- Fixed the v0.3 response leak where every nearest prototype could emit its
  concept prose even when the prompt supplied no semantic support.
- Concept IDs remain visible in telemetry, but concept prose now abstains when
  alias/meaning overlap is zero.
- Added context-aware handling for sensing claims, “why do you think this?”,
  repeated-answer reports, malfunction reports, and live loop diagnosis.
- Added a cross-question duplicate-output guard.
- The user's exact six-turn transcript is now a permanent regression suite:
  6/6 passed with six unique outputs and no leaked life/time answer.
- Rust tests: 48 passed across library and dark-blood CLI targets.
- Original routing: 1.0; hard transfer: 1.0; concept transfer: 16/16; trap
  abstention: 1.0.
- Dialogue receipt:
  `models/candidates/evaluation-v3.1-dialogue.json`.
- Dialogue receipt SHA-256:
  `e52267a61618e916b010888cb59b3d35dfe4b0c7c857916f05125ca59fdb59b2`.

## Bitwork v0.3 evidence-gated promotion — 2026-07-16

- Active model: `models/perci-cognitive-v0.3.pwgt` (`PERCIW03`).
- Size: 200,677,376 bytes / 191.38 MiB, below the 200 MiB hard ceiling.
- 385,792 unique 4,096-bit prototypes across 16 expert domains.
- 90 weight-resident concepts spanning geometry, language, logic, life, death,
  evolution, systems, memory, knowledge, and related mechanisms.
- Original held-out suite: domain 1.0, local precision/recall 1.0/1.0, trap
  abstention 1.0.
- Hard 76-case transfer suite: domain 1.0, local precision/recall 1.0/1.0,
  trap abstention 1.0.
- Concept transfer probes: 16/16.
- Rust tests: 44 passed across library and dark-blood CLI targets.
- Parent LumenShell tests: 137 passed.
- Candidate SHA-256:
  `3028d1013032152cc54d20a9b2ee62d56f8c7be9f315ba4aadd29fd4d66b5145`.
- Promotion receipt SHA-256:
  `d663aa4e632f2620916a6c69bc4ad874e4d2d2061ffb75b8e7f5a86aecc95d7f`.
- No LLM or transformer is attached; promotion remains explicitly authorized,
  receipt-bound, and reversible.

## Commandless learning and repair-aware dialogue — 2026-07-16

- Natural phrases such as `I want you to learn that ...` now stage governed
  teaching candidates; `/teach` remains optional for scripts and inspection.
- Directness and explanatory-depth feedback persist in the dialogue profile.
- Generic local fallbacks are exposed as support gaps after directness feedback
  instead of being presented as confident answers.
- Added contextual repair and elaboration for the captured compact-model
  conversation, plus explicit learning-speed and memory/teaching distinctions.
- Fixed singular/plural learning telemetry for human-readable status.
- Exact shipped-launch transcript replay passed through a fresh live target.
- Rust tests: 43 passed across library and dark-blood CLI targets.
- Parent LumenShell tests: 137 passed.
- Sealed receipt: `models/candidates/evaluation-v2.1.9-commandless-learning.json`.
- Receipt SHA-256: `273b351e7d78c00d1feaeb39407ddaefe051393d46405605d6acf0da60320d12`.
- Status remains `OPERATIONAL_CANDIDATE`; automatic promotion remains false.

## Governed teaching and live deployment — 2026-07-16

- Replaced timestamp-only launcher selection with a Cargo-verified dedicated
  live target, preventing `Start-Perci.cmd` from serving stale release code.
- Added `/teach <claim>` and `perci teach <claim>` as explicit, review-only
  knowledge candidates with sensitive-content rejection and visible counts.
- Connected explicit teaching records to the existing curriculum review queue;
  facts and weights still never promote automatically.
- Added direct human-readable paths for system evolution and knowledge-building.
- Exact shipped-launch replay passed from `Start-Perci.cmd`.
- Rust tests: 40 passed across library and dark-blood CLI targets.
- Parent LumenShell tests: 137 passed.
- Sealed receipt: `models/candidates/evaluation-v2.1.8-governed-teaching.json`.
- Receipt SHA-256: `ffa794826ccefc29f068d5ba579100a0ff0e03bdd738886204e615ebc314c456`.
- Status remains `OPERATIONAL_CANDIDATE`; automatic promotion remains false.

## Contextual dialogue repair — 2026-07-16

- Replayed the captured 14-turn conversation end to end with isolated session
  and learning files; all targeted turns now take coherent dialogue paths.
- Added bounded handling for measured change, adaptation versus learning,
  uncertainty, operational awareness, natural style repair, session facts,
  numeric recall, and pronoun resolution.
- Relational dialogue now outranks exact-tool parsing, preventing
  `distinction` from being misread as an arithmetic request.
- Intelligence-pack guidance is attached only when the user explicitly asks
  for deep analysis, preventing retrieved fragments from contaminating chat.
- Rust tests: 39 passed across library and dark-blood CLI targets.
- LumenShell integration tests: 137 passed.
- Optimized runtime: `target/codex-release/release/perci.exe`.
- Sealed receipt: `models/candidates/evaluation-v2.1.7-context-dialogue.json`.
- Receipt SHA-256: `5f887310a89320b6df8c28e6b5293d93fd318c614ee10f575e3d45e4aeedecb6`.
- Evaluation remains `OPERATIONAL_CANDIDATE`: domain accuracy 0.875,
  local precision/recall 1.0/1.0, trap abstention 1.0, no automatic promotion.

## SBCG/GAD v2 standalone port — 2026-07-16

### Dialogue-learning evolution

- Self-reflection replay now distinguishes operational awareness, measurable
  adaptation, bounded system self-knowledge, positive feedback, and limit tests.
- Distinct questions no longer trigger the `Same core answer` duplicate guard.
- Final alternate release tests: 38 passed while the user-running executable
  remained locked; normal launcher rebuild is pending process exit.
- Final sealed receipt: `models/candidates/evaluation-v2.1.6-self-model.json`
- Receipt SHA-256: `672f74585919176b8f03e9ecc4f5c8bc27cfefe34bd956f7b98149feefd6a6b8`

- Rust tests: 37 passed across library and dark-blood CLI targets.
- Four-turn conversational replay: direct presence, accurate learning answer,
  natural acknowledgement, and feedback acceptance all passed.
- Safe preference adaptation persisted `concise`, `warmth`, and
  `avoid_structured_chat` after one style-feedback turn.
- Interaction staging produced five review candidates with no automatic
  curriculum or weight promotion.
- Response headers report measured elapsed time; no artificial delay is used.
- Fresh receipt: `models/candidates/evaluation-v2.1.5-dialogue-learning.json`
- Receipt SHA-256: `fd7d87d0760bd7a3deeda632784de9ab01b63d40ff97b4a2e4fd4bd2f1314ebd`


- Active model: `models/perci-cognitive-v0.2.pwgt`
- SHA-256: `11c20d1bee6fd946d4d7165a625dc0db2bc52f51957ef8b66aea6db5949c7747`
- Format/size: `PERCIW02`, 38,580 unique prototypes, 20,094,368 bytes
- Rust tests: 33 passed across library and CLI targets
- Live intelligence probe: 8/8 domains after a bounded science-prior correction
- Fresh sealed receipt: `models/candidates/evaluation-v2.1.4-standalone-port.json`
- Receipt SHA-256: `4035aeeae309f90c9f648d69452f67bdd5057defb218f03edb0754a634951655`
- Held-out domain accuracy: 0.875; keyword baseline: 0.625
- Selective local precision/recall: 1.0 / 1.0; trap abstention: 1.0
- Claim ceiling: operational candidate only; confidence bounds and independent
  replication remain required.


## Completed in the build environment

- Generated `models/perci-cognitive-v0.1.pwgt` from the deterministic curriculum.
- Verified exact size: `209,715,200` bytes.
- Verified format magic: `PERCIW01`.
- Verified 16 expert entries and 403,266 prototype records.
- Verified SHA-256 against the generated manifest.
- Loaded and queried the weight file through the Python reference implementation.
- Ran 16 held-out domain probes; all 16 routed to the expected expert in the recorded run.
- Performed a structural delimiter pass over the Rust source.

## Not completed in the build environment

Rust and Cargo were not installed, so the Rust crate was not compiled or benchmarked here. Run:

```powershell
.\Launch-Perci.ps1 -Mode test
```

before treating the Rust executable as verified on your machine.

## Interpretation

The held-out probes validate the binary associative routing mechanism and prototype retrieval. They do not demonstrate parity with a pretrained transformer or establish general intelligence.
