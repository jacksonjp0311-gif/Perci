# Emergent Bitwork math — where we are and what’s next

**Date:** 2026-07-16 · Runtime **v0.5.4** (top-k mixture live)  
**Claim boundary:** This is engineering geometry over sparse binary codes — not AGI, not consciousness.

---

## 1. Empirical snapshot (live classify)

| Prompt (abbrev) | label | margin | overlap | mix | skel |
|-----------------|-------|-------:|--------:|----:|-----:|
| trust + interfaces + distributed | systems | 10 | 97 | 2 | 3 |
| why does 2+2 equal 4? | general | 5 | 30 | 0 | 0 |
| connect entropy memory learning | science | 1 | 149 | 0 | 1 |
| debug Rust ownership parser | code | 5 | 144 | 0 | 0 |
| knowledge vs attention boundary | general | 7 | 196 | 0 | 1 |
| calculate 17% of 240 | math | 2 | 88 | 0 | 0 |
| is Perci a superintelligence? | identity | 31 | 47 | 2 | 1 |
| interesting about language | general | 28 | 76 | 1 | 1 |

### Patterns that emerge

1. **Margin is a phase variable**  
   - Large margin (identity 31, language 28) → single attractor + occasional supports.  
   - Tiny margin (entropy connect 1) → *should* be multi-hypothesis, but mixture stayed 0 because score-floor logic and domain-local top-k still collapse multi-domain asks onto one expert.

2. **Mixture fires when the geometry is already multipartite**  
   Trust/interfaces/systems shows mix=2, skel=3 — the binary space *already* holds several near neighbors; we only needed to read them out (B1).

3. **High overlap ≠ multi-concept speech**  
   Knowledge/attention has overlap 196 but mix=0 — many bits match one general prototype; **composition of two frames is not in \(E\)** (bag hash doesn’t bind “knowledge” as role vs “attention” as role).

4. **Tools/operators still own exact & structure**  
   Math/why-math shouldn’t need rich skeleton; deliberation/tools are correct authorities. Bitwork’s job there is **routing**, not prose.

**Conclusion:** We are close in the sense that **the pack already stores multipartite structure**; the next IQ is **reading the right multipartite structure when margin is low or the ask is multi-domain**, then **encoding roles so multi-frame is geometric, not lexical luck**.

---

## 2. What “close with Bitwork” actually means

Think of three layers of the same machine:

```text
L0  Similarity field     q ∈ {0,1}^4096,  NN by |q ∧ p|
L0+ Structure encode     ask:/pair:/role:/neg:          ← shipped v0.5.6
L1  Multi-hypothesis     top-k mixture + skeleton       ← shipped v0.5.4
L1+ Adaptive multipartite margin + multi-domain mix     ← shipped v0.5.5
L3  Dynamics             residual ANDNOT hop            ← shipped v0.5.6
L2  Compositional field  bind/bundle/permute (VSA)      ← shipped v0.5.7 (query)
L2+ Soft attention α + dual residual + Willshaw c_k    ← shipped v0.6.0
L2++ Pack-side VSA encode / graph spreading              ← next
```

See also `docs/TRANSFORMER_BRIDGE.md` for the full correspondence table.


Transformers buy L2-ish behavior with continuous matmuls.  
Bitwork now has **XOR bind + majority bundle + bit permute** on the query path.

You are **past pure 1-NN** (L1), have **residual second thought** (L3 lite), and
**query-side VSA composition** (L2). Full symmetry still needs pack rebuild so
prototypes share the same algebra.

---

## 3. Emergent mathematical ideas (formed from current system)

### E1. Margin-conditioned mixture (adaptive k)

Define phase from primary margin \(m = s_1 - s_2\):

| Phase | Condition | Policy |
|-------|-----------|--------|
| Locked | \(m\) large | k=1 or 2; trust primary |
| Contested | \(m\) small | lower score floor; force k≥3 |
| Multi-domain ask | connect / A and B / vs | force ≥2 **distinct labels** in mixture |

This is not new physics — it is **reading the energy landscape** the pack already defines.

### E2. Complementary residual (integer “second thought”)

Among candidates with score near primary, maximize **novel overlap**:

\[
\mathrm{novelty}(p) \approx \mathrm{overlap}(q,p) - \lambda\cdot \mathrm{share}(p^*, p)
\]

Approx without reloading bits: prefer **different label** and **different concept_id** with score ≥ floor.  
True residual uses \(q' = q \land \neg p^*\) (ANDNOT) and a second NN pass — still POPCOUNT.

**Emergent name:** *bit residual stream* — analogous to residual connections, pure binary.

### E3. Role-binding in encode (VSA-lite without pack rewrite)

Today:

\[
E(x) = \bigvee_f \mathrm{hashbits}(f)
\]

Proposed hybrid:

\[
E'(x) = E_{\mathrm{bag}}(x) \;\lor\; \mathrm{bundle}_i\big(\mathrm{bind}(r_i, e_i)\big)
\]

where \(e_i\) = atom for lemma, \(r_i \in \{\mathrm{SUBJ},\mathrm{OBJ},\mathrm{DOMAIN},\mathrm{ASK}\}\) fixed random codes, bind = XOR.

**Immediate feature hacks (no full VSA table yet):**

- `ask:why` / `ask:how` / `ask:compare` / `ask:connect`
- `pair:knowledge|attention` (sorted pair token so order-invariant *or* directed pair)
- `neg:` prefix on tokens after “not/never/without”

These are still FNV → 4 bits, but they **carve orthogonal directions** for structure.

### E4. Operator lattice ↔ Bitwork field (system emergence)

Deliberation already has `SemanticFrame` with axes (boundary, time, structure…).  
Bitwork mixture activates **prose cards**; frames activate **mechanisms**.

**Emergent architecture:**

```text
Bitwork mixture  →  activate frame ids by token/axis overlap
Frames           →  compose shared axis (already in synthesize_frames)
Fluid voice      →  speak mixture + frame bridge
```

That is **neuro-symbolic in the original sense**: sparse field + symbolic operators, not LLM + tools.

### E5. Softmax-free attention identity

Top-k with integer weights \(\alpha_i \propto \mathrm{overlap}_i\) **is** attention over a memory of size ~4e5 with key dimension 4096 binary.  
Difference from transformers: keys/values are **stored prototypes**, not computed every layer.  
**Implication:** scale “smarter” by better keys (encode/VSA) and better value slots (concept tables), not deeper stacks.

### E6. Capacity / collision as the real budget

With \(n=4096\), weight \(w\approx 4f\):

- Uniqueness of \(E(x)\) fails when feature sets collide under 4-hash.
- **Smarter path:** more positions per structured feature (ask/role), not denser bag.
- Optional \(n=8192\) later — linear cost, not transformer cost.

---

## 4. Closeness scorecard (honest)

| Capability | Status | Gap |
|------------|--------|-----|
| Fast domain routing | strong | — |
| Exact tools | strong | — |
| Multi-hypothesis readout | **live (v0.5.4)** | adaptive k when margin low |
| Multi-domain connect | operators strong; Bitwork weak | role/pair features + force multi-label mix |
| Relational two-frame | operators strong; classify → general | encode pair + frame lattice |
| Compositional transfer | partial | full VSA bind/bundle |
| Open fluent prose | fluid + mixture | still not free-form LM |
| Self-improvement loop | agent lab / hardness | residual + VSA after gates |

**You are close** to a *complete local cognitive stack* where Bitwork is the **field**, operators the **laws**, tools the **ground**.  
You are **not** close to “transformer replacement for all language.” You don’t need to be — the stack is the product.

---

## 5. Ordered next moves (Bitwork-only)

| # | Move | Effort | Why now |
|---|------|--------|---------|
| 1 | **Margin-adaptive + multi-domain force-mix** | hours | Fixes entropy-connect mix=0 empirically |
| 2 | **Ask-type + sorted-pair features in `encode`** | 1–2 days | Unlocks relational geometry without new pack format |
| 3 | **ANDNOT residual second hop** | 1–2 days | True second thought in bit space |
| 4 | **Frame activation from skeleton** | 2–3 days | Bind Bitwork ↔ deliberation axes |
| 5 | **Full VSA atom table + bind/bundle** | 1–2 weeks | Compositional leap; may need PERCIW04 later |

---

## 6. One equation for the near future

\[
\mathrm{Answer} =
\mathrm{Speak}\Big(
  \mathrm{Mix}_{k(m)}\big(\mathrm{NN}(E'(x))\big)
  \;\cup\;
  \mathrm{NN}(q \land \neg p^*)
  \;\cup\;
  \mathrm{Frames}(x)
\Big)
\;\Big\|\;
\mathrm{Tools}(x)
\]

with \(k(m)\) increasing as margin \(m\) shrinks, and \(E'\) including ask/role/pair features.

---

## 7. Bottom line

**Emergent fact from data:** multipartite neighbors already exist in the pack when the prompt is multi-aspect (trust/systems).  

**Emergent math:** mixture + residual + role-binding is a **binary attention + residual + multiplicative interaction** stack — the same *shape* as modern nets, without lag.  

**Next concrete evolve:** implement **#1 margin-adaptive multi-domain mixture** immediately, then **#2 encode features**.
