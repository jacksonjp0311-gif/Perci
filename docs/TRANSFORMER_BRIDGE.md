# Bridging toward transformer-style intelligence (Bitwork algebra)

**Premise:** A transformer is not “magic intelligence.” It is a stack of
**soft associative retrieval + residual dynamics + compositional mixing**.
Perci can implement the same *functions* with integer bit algebra — not by
importing matmuls.

---

## 1. Cross-domain mathematical correspondence

| Transformer primitive | Bitwork / Perci analog | Status |
|----------------------|------------------------|--------|
| Q·Kᵀ attention scores | \|q ∧ p\| POPCOUNT overlap | L0 shipped |
| Softmax α over heads | Fixed-point **permille** α ∝ overlap | **v0.6.0** |
| Multi-head readout | Top-k mixture + multi-domain pool | L1 shipped |
| Residual stream | q′ = q ∧ ¬p\* (ANDNOT hops) | L3 dual-hop **v0.6.0** |
| Soft binding in residual MLP | VSA bind (XOR) + bundle (majority) | L2 query shipped |
| Positional encoding | Bit permute ρ(slot) | L2 shipped |
| KV cache / context | CTX role-bind of recent lemmas | **v0.6.0** |
| Value projection | Concept Willshaw HVs \|q ∧ c\| | **v0.6.0** |
| Layer stack depth | Residual hop depth 1→2 | **v0.6.0** |
| FFN / decode | Operators + fluid voice (outside bits) | shipped |
| Gradient training | Human-gated weight rebuild only | governance |

**What transformers still win at:** open-domain fluent generation, huge
parametric knowledge in continuous space. **What Bitwork wins at:**
inspectability, exact tools, local ms latency, compositional roles without
opaque matmuls.

---

## 2. Emergent math (what “smarter” means here)

### Soft multi-hypothesis attention

\[
\alpha_i = \frac{\mathrm{overlap}(q,p_i)}{\sum_j \mathrm{overlap}(q,p_j)}
\quad\text{(stored as permille)}
\]

Primary + mixture cards share one attention budget. Skeleton speech prefers
**higher α** first — multi-facet without a decoder.

### Residual stream (binary)

\[
\begin{align*}
q_0 &= E_{\mathrm{bag+VSA+CTX}}(x)\\
p_1 &= \mathrm{NN}(q_0)\\
q_1 &= q_0 \land \neg p_1\\
p_2 &= \mathrm{NN}(q_1)\\
q_2 &= q_1 \land \neg p_2\\
p_3 &= \mathrm{NN}(q_2)
\end{align*}
\]

Each hop is a *second thought* the first prototype masked — analogous to
residual connections that let later layers correct earlier features.

### VSA compositionality

\[
q_{\mathrm{vsa}} = \mathrm{bundle}_i\big(\mathrm{bind}(r_i,\rho_i(e_i))\big)
\]

Systematicity (why vs how, agent vs domain) without soft attention layers.

### Willshaw-lite concept memory

Concept insights become bag hypervectors \(c_k\). Selection score:

\[
s_k = \mathrm{alias}(x,c_k) + 5\cdot\mathrm{word}(x,c_k) + 4\cdot|q \land c_k|
\]

This is **query → concept** associative memory on top of prototype NN —
closer to transformer value pathways than pure 1-NN prose cards.

### Session CTX bind

Recent content lemmas \(\ell_t\):

\[
q \;\gets\; q \lor \mathrm{bundle}_t\big(\mathrm{bind}(r_{\mathrm{CTX}},\rho_t(e_{\ell_t}))\big)
\]

Dialogue continuity without a full language model state.

---

## 3. Implementation map (v0.6.0)

| Piece | Location |
|-------|----------|
| Soft attention α | `MixtureSupport.attention_pm`, `primary_attention_pm` |
| Dual residual | `classify_with_context` residual loop hop=1..2 |
| Concept HVs | `CognitiveWeights.concept_hvs` at load |
| CTX bind | `encode_session_context` + backend `classify` |
| Classify schema | `perci.classify.v5-attn` |

---

## 4. Honest ceiling (still no decoder)

| Gap | Next evolution (not yet) |
|-----|--------------------------|
| Pack bag-trained, query VSA | Authorized PERCIW04 with VSA-encoded prototypes |
| No generative token path | Larger operator lattice + curriculum, not fake GPT |
| Concept HVs are bag, not VSA | Encode concepts with same structure map |
| Depth=2 residual | Spreading activation graph on prototypes (Tier B) |

---

## 5. Objective function (measure, don’t vibe)

Simultaneous:

1. Routing transfer ≥ baseline  
2. Hardness non-decreasing  
3. Multi-domain connect quality (folded phrases, no critic footer noise)  
4. Unique concept utilization ↑  
5. Warm classify p50 still interactive (&lt; ~50–100 ms target on local SSD)

---

## 6. Bottom line

Transformer-style intelligence here means:

\[
\mathrm{Mix}_{\alpha}\big(\mathrm{NN}(q_{\mathrm{VSA+CTX}})\big)
+ \mathrm{ResidualStream}_{h\le 2}
+ \mathrm{Willshaw}(c)
+ \mathrm{operators/tools}
\]

All integer-hot-path. That is the bridge — not bolting on a decoder first.
