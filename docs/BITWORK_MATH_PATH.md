# Bitwork mathematical path to greater intelligence (no transformer)

**Premise:** Smarter need not mean “add a decoder stack.” Bitwork is already a
**sparse binary associative machine**. The ceiling today is less “not enough
compute” and more **underused algebra**: we use similarity, but almost no
**composition**, **mixture**, or **structure-preserving transforms**.

This document analyzes the **actual** Perci math, then ranks extensions that
stay **integer-hot-path, local, and low-latency**.

---

## 1. What Bitwork is mathematically (today)

### 1.1 Encoding map \(E\)

\[
\text{prompt} \;\xrightarrow{\;E\;}\; q \in \{0,1\}^{4096}
\]

Implementation (`src/cognitive.rs::encode`):

| Feature family | Symbol |
|----------------|--------|
| Bias + length bucket | fixed |
| Word unigrams | \(w:t\) |
| Prefix / suffix (3) | \(p:, s:\) |
| Bigrams | \(b:t_i\|t_{i+1}\) |
| Char trigrams on joined text | \(c:\) |
| Ask-type (why/how/compare/connect/what) | `ask:*` **(v0.5.6)** |
| Sorted content pairs (order-invariant) | `pair:a\|b` **(v0.5.6)** |
| Role heuristics (agent/domain/topic/focus) | `role:*` **(v0.5.6)** |
| Negation scope | `neg:*` **(v0.5.6)** |

Each feature string is hashed (FNV-1a); **4 bit positions** are set (OR into
the 64×`u64` packed vector). So \(E\) is a **random sparse feature map**
(Bloom-like), not a learned embedding.

Approximate activation density: if \(f\) features fire, at most \(4f\) bits
(with collisions fewer). Typical prompts stay sparse relative to 4096.

### 1.2 Expert routing (signed masks)

For domain \(d\), positive/negative masks \(m_d^+, m_d^- \in \{0,1\}^{4096}\):

\[
s_d = |q \land m_d^+| - |q \land m_d^-| + \text{lexical prior}
\]

Integer `AND` + `POPCOUNT`. Top domains win; then only those partitions are
scanned (latency control).

### 1.3 Prototype retrieval (nearest neighbor in intersection space)

Stored prototypes \(p_i \in \{0,1\}^{4096}\). Score:

\[
\text{overlap}(q,p_i) = |q \land p_i|
\]

Telemetry also uses Jaccard and chance-normalized z-score, but the **decision**
is essentially **1-NN under intersection / Hamming geometry**.

### 1.4 Response selection

Prototype → domain variant + optional concept id → **prose outside the binary
core** (operators, fluid voice, tools). Concepts are alias-gated; the bit
machine does **not** generate tokens.

### 1.5 What this *can* do well

| Property | Why |
|----------|-----|
| Fast | ~64 ANDs per comparison; mmap pack |
| Inspectable | bits, margins, domains are real |
| Transfer-ish | similar lexical geometry → similar route |
| Exact tools | routing ≠ calculation |

### 1.6 Structural bottlenecks (math, not “need GPT”)

| Bottleneck | Consequence |
|------------|-------------|
| **Bag-of-features** (order only via bigrams) | “A vs B” ≈ “B vs A”; weak role structure |
| **1-NN prototype** | One nearest card; no mixture of ideas |
| **Hash collisions** | Unrelated features share bits → confusable states |
| **No bind/unbind** | Cannot store “trust-as-role in systems” as algebra |
| **Prose outside bits** | Intelligence of *speech* lives in operators/voice unless we extend records |
| **4096 fixed** | Capacity vs collision tradeoff is hard-coded |

**Transformer lag is not the only path.** The open math is: make \(q\) and
retrieval **compositional** while keeping POPCOUNT hot.

---

## 2. Capacity sketch (why “more prototypes” alone plateaus)

Sparse binary codes in dimension \(n=4096\) with weight (active bits) \(w\):

- Pairwise random codes are nearly orthogonal when \(w \ll n\).
- Associative memories (Willshaw / SDM-style) store on the order of
  \(\sim n^2 / w^2\) or better with structure — **but only if addresses are
  well-separated and retrieval uses the right metric**.
- Perci currently stores **~4×10⁵ unique prototypes**. Collisions in \(E\) and
  1-NN mean many prompts map to the **same speech card** even when “near”
  several distinct ideas.

**Implication:** Raising IQ is less “store 10× more near-duplicates” (v1→v3
already learned uniqueness) and more **richer \(E\)** + **multi-prototype
readout** + **algebraic composition**.

---

## 3. Mathematical upgrade menu (Bitwork-native)

Ranked by **intelligence gain × feasibility × still-fast**.

### Tier A — High leverage, still integer, fits Perci soon

#### A1. Top-\(k\) prototype mixture (soft 1-NN → ensemble)

Instead of single \(p^*\):

\[
\{p_{(1)},\ldots,p_{(k)}\},\quad
\alpha_i = \frac{\text{overlap}(q,p_{(i)})}{\sum_j \text{overlap}(q,p_{(j)})}
\]

(Use integer ranks / fixed-point weights if you want zero float on hot path;
floats only for telemetry is fine.)

**Use:** blend **concept ids** and **response slots** (not free text inside the
pack at first):

- skeleton = merge entities from top-k concepts  
- fluid voice fills prose  

**Why smarter:** multi-facet answers (trust *and* systems *and* failure mode)
without a decoder.

**Latency:** \(k\) small (3–8); already scanning partitions.

#### A2. Binary Spatter / VSA ops on the same 4096-bit space — **shipped in v0.5.7**

Kanerva-style **Binary Spatter Codes** (integer-friendly):

| Op | Bit realization | Meaning |
|----|-----------------|--------|
| Bind | \(a \oplus b\) (XOR) | role–filler, “trust-in-systems” |
| Unbind | same as bind (XOR self-inverse) | recover filler given role |
| Bundle | majority / thresholded OR of several vectors | set of ideas |
| Permute | fixed bit permutation \(\rho\) | sequence / order |

**Encoding upgrade example:**

\[
q = \mathrm{bag}(E) \;\lor\; \mathrm{bundle}\big(
  \mathrm{bind}(r_{\text{agent}}, \rho(e_{\text{trust}})),\;
  \mathrm{bind}(r_{\text{domain}}, \rho(e_{\text{systems}})),\;
  \mathrm{bind}(r_{\text{ask}}, e_{\text{why}})
\big)
\]

where \(e_*\) are **deterministic sparse atomic hypervectors** (8 bits / symbol).

**Status (v0.5.7):** `encode_vsa_composition` in `src/cognitive.rs` —
atom / bind / bundle / permute; frame exposed as `composition[]` on classify;
fluid `weave_composition_frame` speaks “Bound as ask:why · agent:trust …”.

**Why smarter:** compositionality and systematicity — the missing piece vs
transformers’ soft binding — **without matmuls**.

**Latency:** a few XORs over 64 words ≈ noise compared to prototype scan.

**Honest limit:** prototypes remain bag-trained; VSA is a **query overlay** until
authorized pack rebuild re-encodes with the same algebra.

#### A3. Residual / multi-hop retrieval — **shipped in v0.5.6**

\[
\begin{align*}
p_1 &= \mathrm{NN}(q)\\
q_2 &= q \;\mathrm{ANDNOT}\; p_1 \quad\text{or}\quad q \oplus \mathrm{proj}(p_1)\\
p_2 &= \mathrm{NN}(q_2)
\end{align*}
\]

**Why smarter:** second concept that the first card hid (classic “also relevant”).

**Latency:** bounded residual re-scan of top expert partitions + pool re-score — still ms-scale.

**Status:** `classify` loads \(p^*\), builds \(q' = q \land \neg p^*\), nearest residual
support is marked `mixture[].residual=true` and voiced as “Latent residual…”.

#### A4. Structure features in \(E\) (still hash, but smarter features) — **shipped in v0.5.6**

Add features that hash **roles**, not only bags:

- `role:agent|trust`, `role:domain|systems` (from light patterns / POS-free heuristics)
- `ask:why`, `ask:how`, `ask:compare` as first-class bits
- dependency-lite: sorted `pair:a|b` (order-invariant vs/and pairs)
- negation / modal scopes: `neg:trust`

**Why smarter:** reduces “A vs B” ↔ “B vs A” and “why” vs “how” confusions.

**Latency:** encode cost negligible.

**Status:** `encode_structure_features` in `src/cognitive.rs`; no pack rebuild required
(hash features only affect query side; prototypes still bag-encoded at train time —
asymmetric lift until next authorized weight rebuild).

---

### Tier B — Medium research, still no transformer

#### B1. Willshaw / outer-product associative memory overlay

Store pairs \((q \mapsto c)\) for concept codes \(c\) via binary outer product
(OR of \(q_i \land c_j\) patterns). Retrieval: thresholded AND-POPCOUNT into
concept space.

**Why smarter:** direct **query → concept vector**, not only query → nearest
training prompt.

#### B2. Graph on prototypes (spreading activation)

Edge \(p_i\sim p_j\) if Jaccard \(> \tau\). At inference, activate neighborhood
of top-k (integer hops).  

**Why smarter:** analogical chains inside the pack (“trust” prototypes fire
“contract” and “interface” neighbors).

#### B3. Hierarchical codes

Coarse 1024-bit domain code + fine 4096-bit residual, or multi-resolution
hashes.  

**Why smarter:** better routing under paraphrase; less expert thrash.

#### B4. Controlled denser coding

Increase positions per feature (4 → 8) or bits (4096 → 8192) with uniqueness
budget.  

**Capacity** rises; **collision** falls; pack size and scan cost grow linearly
in words. Still POPCOUNT, still no GPU.

---

### Tier C — Deeper theory (optional later)

| Idea | Note |
|------|------|
| Discrete Hopfield / modern DAM | Energy minimization in binary/spin form |
| Hyperdimensional analogy transforms | Learn \(T\) s.t. \(T(a)\approx b\) from pairs (integer approx) |
| Sparse coding dictionary (OMP-like) | Heavier; may leave pure integer path |
| Full HRR with floats | Powerful; loses pure integer claim unless quantized |

---

## 4. What will *not* be solved by Bitwork math alone

Be explicit:

| Capability | Bitwork path | Still needs |
|------------|--------------|-------------|
| Novel fluent paragraphs on unseen topics | Partial (mixture + VSA + fluid voice) | Either huge curriculum or external LM |
| Multi-step symbolic proofs | Routing + operators + tools | Operator/tool expansion |
| Exact arithmetic | **Never** in prototypes | Keep tools |
| World action | Outside weights | Agent tools |

The goal is **high competence per millisecond**, not “decoder parity.”

---

## 5. Recommended Bitwork-only roadmap (ordered)

### Phase B0 — Measure the geometry (1–2 days)

- Log for hardness/live fails: margin, Jaccard, top-5 prototype ids, concept ids.
- Metric: **fraction of fails with second prototype within 10% overlap of first**  
  → if high, **A1 mixture** is free IQ.

### Phase B1 — Top-k mixture + multi-concept skeleton — **DONE (v0.5.4)**

- Runtime: top-3 prototypes per domain → global merge → primary + mixture[] 
- `concept_skeleton` + `weave_mixture_skeleton` for multi-facet fluid speech
- Classify schema `perci.classify.v3-mixture`
- Pack format unchanged; integer path only

### Phase B2 — Ask-type + role features in encode (1 week)

- Extend `encode` only; rebuild candidate pack optional if masks retrain.
- Immediate gain on why/how/compare without new model class.

### Phase B3 — VSA bind/bundle layer (2–3 weeks)

- Atomic lemma codes (table or hash-stable).
- `q_vsa = bundle(bind(role, filler), …)` mixed with or replacing bag code.
- Prototype store can remain; optionally store VSA keys in PERCIW04.

### Phase B4 — Residual second hop (days after B1)

- Cheap; stacks with mixture.

### Phase B5 — PERCIW04 only if B1–B3 prove transfer gain

- New format only with sealed hardness lift + authorize promote.

---

## 6. Formal “smarter” objective (so we don’t chase vibes)

Define improvement as simultaneous:

1. **Routing transfer** ≥ current (held-out domain accuracy).  
2. **Hardness** pass rate non-decreasing, cases harder over time.  
3. **User-token binding rate** (reply mentions user content) ↑  
4. **Unique concept utilization** (entropy of concept ids on open chat) ↑  
5. **p50 latency** for classify + fluid path still &lt; 50 ms warm.

If (3)–(4) rise and (5) holds, Bitwork got smarter **without** transformer lag.

---

## 7. Bottom line

Bitwork today is:

\[
\text{smartness} \approx \mathrm{NN}_{\land}\big(E_{\text{bag}}(x)\big) + \text{operators} + \text{tools}
\]

The high-IQ extension is:

\[
\text{smartness} \approx
\mathrm{Mix}_k\Big(
  \mathrm{NN}\big(
    \mathrm{VSA}\big(E_{\text{role}}(x)\big)
    \;\Vert\;
    E_{\text{bag}}(x)
  \big)
\Big)
+ \text{residual hop}
+ \text{operators/tools}
\]

All of that is **bit algebra + POPCOUNT**, not attention matmuls.

**There is a better way than bolting on a transformer first:**  
make the binary space **compositional** (bind/bundle), **multi-hypothesis**
(top-k), and **multi-hop** (residual) — then let fluid voice speak the
skeleton. That is how Perci stays dark-blood fast and still gets less generic.

---

## 8. Suggested first experiment (smallest proof)

1. Instrument classify to return **top-5** prototypes + overlaps.  
2. On open chat only, pass top-3 **concept insights** into fluid compose as a
   bullet of constraints (not a dump).  
3. Measure binding + “generic marker” rate on 30 live prompts.  

If that moves the needle, implement A2 (VSA) next — not a 7B model.
