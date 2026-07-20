# Open-language bridge (v0.9.0)

Perci's open-language gap is not solved by making the Bitwork pack denser. The
current bridge separates three jobs:

1. **Noisy input** is repaired before routing.
2. **Meaning and authority** stay in the existing operators, exact tools, and
   governed memory.
3. **Expression** is selected from the native binary phrase field and the
   structured voice/composition layer.

## Spelling-robust routing

For an input string `x`, Perci computes a bounded route key:

```text
r(x) = collapse_whitespace(lowercase(repair_typos(x)))
```

`repair_typos` first checks an explicit Perci-domain alias table. If no alias
matches, it accepts a candidate only when the Damerau-Levenshtein distance is
exactly one and the match is unique in the small routing vocabulary:

```text
d_D(word, candidate) <= 1,  |{candidate}| = 1
```

The accepted operations are one insertion, deletion, substitution, or adjacent
transposition. Unknown names and invented tokens remain unchanged. This is a
deliberate precision/recall trade: a missed typo is preferable to silently
rewriting a proper noun or an out-of-distribution phrase.

The raw turn remains available to the session/evidence path; the repaired view
feeds reflex routing, operator matching, Bitwork lexical priors, voice depth,
and native phrase selection. Exact arithmetic therefore still reaches the
exact tool when the user writes `calcualte`, while a fabricated name such as
`Nembit` is not “corrected.”

## Expression path

The native path is still a binary word-transition field, not a hidden dense
language model. Its bounded open-language gate now admits conceptual prompts
such as “what do you think,” “how would you describe,” “give me an image,” and
“connect …” after rejecting exact-tool, proof, capability, and out-of-
distribution requests. A six-way native beam is scored by topic overlap,
relation novelty, binary relation/world evidence, and recent-answer distance.

The structured voice layer remains the safety rail: it controls answer depth,
checks deictic referents, rejects unrelated concept cards, and keeps a direct
answer before a mechanism or analogy. This makes the output more human-readable
without pretending that a short binary field has frontier-model coverage.

## Fluency probe and repair

The first targeted probe found a specific failure mode rather than a missing
concept: prompt verbs such as “express” and “say it differently” entered the
topic list, and a creative code/music request could receive a code-debugging
card. The repair removes those scaffolding tokens from voice binding, routes
creative prompts through a constrained relation frame, and collapses the
repeated “relationship … is a relation” phrase into a direct shared-structure
statement. This is a presentation/composition repair; it does not mutate the
Bitwork pack or claim new factual knowledge.

## Follow-up continuity repair

The next adversarial sweep showed that the remaining failures clustered around
speech acts: `I dont agree`, `what would change your mind?`, clarification, and
rephrasing. Perci now extracts the substantive quoted claim from the previous
answer before composing a revision or challenge, preserves those operator
answers through the topic-binding post-pass, and filters creative scaffolding
from topic tokens. The change is intentionally an operator-level repair: it
improves context continuity and typo tolerance without claiming autonomous
learning or changing the native weights.

## Topic-conditioned candidate training

The first native candidate experiment exposed a representation boundary. The
phrase field was using `<topic>` only as a rendering marker, so its transition
history never contained the actual salient words. Runtime now injects those
words into the continuation history while preserving the marker for rendering.
The candidate builder mirrors the same primer contexts and trains only on
reviewed response continuations.

The initial 24-example candidate loaded successfully, but the fresh 12-case
chat comparison stayed equal to the active field (`5/12` required checks and
`0.5` topic binding in both arms). The candidate is therefore held. The result
shows that the native field can change direct sampling, but the current voice
and operator layers dominate full dialogue; a larger, more varied corpus is
needed before promotion is meaningful.

## Paired-turn context channel

The next experiment adds a hidden continuity tail after the semantic primer:
the previous user turn, a short previous answer, and the current user turn. The
tail is capped before entering the order-four transition history and is never
rendered. A prompt-conditioned candidate is built from
`training/dialogue-continuity-v2.jsonl` and evaluated on 12 seed/follow-up
conversations in fresh processes.

The candidate loaded correctly and remained regression-safe, but tied the
baseline at `3/12` required follow-up checks (`0.25`) and `0.2917` topic binding.
That is evidence that the representation boundary is repaired, not evidence
that the weight field has learned general conversation. The candidate remains
isolated until broader paired-turn holdouts show a reproducible gain.

## Why this design

Character- and byte-level language research consistently finds that spelling and
tokenization noise can damage both generation and reasoning, while character
structure helps with unseen words. The cost is longer sequences and a larger
search surface, so Perci uses a narrow correction front-end rather than replacing
the whole native field with an unbounded character model. See [CANINE](https://arxiv.org/abs/2103.06874),
[ByT5](https://arxiv.org/abs/2105.13626), and the [character-structure study](https://aclanthology.org/2023.findings-acl.770/).

Compositional generation is also evaluated separately from memorized phrasing.
Perci's typed operators and bounded composition frames play the role of a small
grammar: roles and relations are selected first, then rendered. This follows the
same general lesson as [grammar-based decoding](https://aclanthology.org/2023.findings-acl.91.pdf)
and compositional-generalization benchmarks, while remaining local and
dependency-free.

## Test set

`training/dialogue-typo-v1.jsonl` is a paired clean/noisy curriculum. It covers
conversation, reasoning, low-bit mathematics, cross-domain synthesis, and an
out-of-distribution control. The fixture is evidence for routing robustness,
not a claim that Perci learned the underlying facts. Run the fast checks with:

```powershell
cargo test --lib --no-default-features -- --test-threads=1
cargo build --release
@'
what do you think about perci inteligence?
why is natral langauge hard?
can you exlpain memory and atention?
how does low bit evoluton preserve magnitute?
calcualte 17 percent of 240
/quit
'@ | .\target\release\perci.exe
```

The exact-tool control should still return `40.8 (exactly 204/5)`. A language
answer is accepted only as a bounded local continuation; no external model is
enabled and no weight file is promoted by this change.
