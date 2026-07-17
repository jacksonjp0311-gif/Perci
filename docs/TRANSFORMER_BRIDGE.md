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

## 3. Implementation map (v0.6.0–v0.6.1)

| Piece | Location |
|-------|----------|
| Soft attention α | `MixtureSupport.attention_pm`, `primary_attention_pm` |
| Dual residual | `classify_with_context` residual loop hop=1..2 |
| Concept HVs | `CognitiveWeights.concept_hvs` at load |
| CTX bind | `encode_session_context` + backend `classify` |
| Classify schema | `perci.classify.v5-attn` |
| **SoftCascade decode** | `src/bridge.rs` — multi-hypothesis compose from α+residual+frame lattice (**v0.6.1**) |
| Frame lattice activate | `deliberation::activate_semantic_frames` (**v0.6.1**) |

### SoftCascade (the latency-preserving “decode”)

Transformers spend most latency in large matmuls for token generation.
SoftCascade **does not sample tokens**. It composes an answer from evidence
already scored in Bitwork + the semantic frame lattice:

```text
classify (ms) → assemble BridgePacket → string join facets → fluid binding
```

That is the breakthrough shape: **LLM-like multi-facet speech** with **Bitwork
latency**.

---

## 4. SoftCascade value stack (v0.6.7)

Transformer decode ≈ weighted sum of value vectors. SoftCascade now mirrors that
with **integer channels**, not token sampling:

\[
\mathrm{Speak}\Big(
  V_{\mathrm{lead}}(\alpha_0)
  + V_{\mathrm{mix}}(\{\alpha_i\})
  + V_{\mathrm{residual}}(\mathrm{hop})
  + V_{\mathrm{VSA}}(\mathrm{bind})
  + V_{\mathrm{frames}}
\Big)
\]

| Channel | Source | Role |
|---------|--------|------|
| Lead | primary Willshaw insight | \(v_0\) |
| Mix | non-residual mixture, α-ordered | multi-head |
| Residual | ANDNOT hop insights, hop-aware transitions | residual stream |
| VSA | `composition[]` role–filler weave | soft binding |
| Frames | semantic lattice clauses | structured FFN-ish |

**Tightens:** residual α mass floor (hop-weighted) so second thoughts don’t vanish
under softmax-like permille; residual novelty bonus for different label/concept.

## 5b. Cognition Trace + length law (v0.6.11)

**Human chat is clean** — no `[Cognition Trace]` prefix. Plans are **backend-only** via
`/think` (and operator audit via `/trace`).

Integer length budget (still applied silently to SoftCascade / operator bodies):

\[
L = \min\!\big(L_{\max},\; \lceil B(1 + 0.6\alpha + 1.2 H_r + 0.4\log_2(1+C) + I_u)\rceil\big)
\]

| Symbol | Meaning |
|--------|---------|
| \(B\) | base words (30 tool / ~120 open) |
| \(\alpha\) | lead attention 0–1 |
| \(H_r\) | residual hop depth 0–2 |
| \(C\) | domain/frame/composition units |
| \(I_u\) | intent (1.0 default, 1.5+ explain/why, 1.8 detailed) |
| \(L_{\max}\) | 420 default · 600 when deep intent |

`/think` shows the last sealed plan (Lead / residual / VSA / length band), plus a
**prototype tree** and **self-critique** report. SoftCascade applies \(L\) **before**
speaking; body only is returned.

### v0.6.12 — critique · style · tree

| Feature | Behavior |
|---------|----------|
| Self-critique residual loop | Thin drafts get one natural second angle from residual/mechanism |
| `/concise` · `/deep` · `/balanced` | Durable dialogue-profile style memory (length + tone prefs) |
| Prototype tree in `/think` | ASCII `◆ lead / ├─ mix / residual / vsa` from Bitwork mass |

### v0.6.13 — thought arc (breakthrough path)

SoftCascade speaks a **thought arc** emergent from Bitwork mass, not free association:

```text
thesis (lead α) → warrant (mixture) → boundary (residual/mechanism) → check (contested)
```

Session **premise bind**: first sentence of the last human answer is held and soft-linked
on follow-ups (“Building on that…”). Contested margin widens length. Patterns emerge from
geometry + arc structure + critique — still no token decoder.

## 5. Free-form fluency path (v0.6.8)

Honest claim: Perci is **not** a free-form LM. Fluency here means **paragraph
speech from multipartite Bitwork values**, not next-token sampling.

| Lever | Change |
|-------|--------|
| Domain search | Adaptive cap (5→12) when coarse experts are contested |
| Prior force-include | Strong lexical prior domains enter the scan even if mid-rank |
| Open fluency detect | why/how/what/explain expand expert budget (not exact tools) |
| SoftCascade prose | Content-first openings, soft mid connectors, no residual jargon |
| Cascade gate | Prefer cascade on open conceptual asks with any insight/mix |

Still not GPT: open fluency fails when the pack has no multipartite mass and no
operator covers the ask. Next real IQ: pack-side VSA + spreading activation.

## 6. Honest ceiling (still no decoder)

| Gap | Next evolution (not yet) |
|-----|--------------------------|
| Pack bag-trained, query VSA | Authorized PERCIW04 with VSA-encoded prototypes |
| No generative token path | Larger operator lattice + curriculum, not fake GPT |
| Concept HVs are bag, not VSA | Encode concepts with same structure map |
| Depth=2 residual | Spreading activation graph on prototypes (Tier B) |
| SoftCascade still value-join | Richer curriculum insights + operator density |

---

## 7. Objective function (measure, don’t vibe)

Simultaneous:

1. Routing transfer ≥ baseline  
2. Hardness non-decreasing  
3. Multi-domain connect quality (folded phrases, no critic footer noise)  
4. Unique concept utilization ↑  
5. Warm classify p50 still interactive (&lt; ~50–100 ms target on local SSD)

---

## 8. Codex diagnosis alignment (ops hygiene)

Urgent work that is **not** “more prototypes”:

| Item | Status |
|------|--------|
| Validation scripts target PERCIW03 | `verify_weights.py` / `test_weights.py` default to active v0.3 |
| Event log vs profile counters | `InteractionLearner` reports both + reconciles lift-only |
| Follow-up state binding | `justify-prior-answer` (v0.6.3) |
| Reject unrelated mixture | SoftCascade relevance gate + lexical concept gate |
| Pack-side VSA | **Still requires human-authorized rebuild** — query-side only today |
| Optional Phi / LM surface | `PERCI_MODEL_CMD` sidecar — never replaces Bitwork core |

## 9. Bottom line

Transformer-style intelligence here means:

\[
\mathrm{Speak}\Big(
  \mathrm{Mix}_{\alpha}\big(\mathrm{NN}(q_{\mathrm{VSA+CTX}})\big)
  + \mathrm{ResidualStream}_{h\le 2}
  + \mathrm{Willshaw}(c)
  + \mathrm{VSA}_{\mathrm{bind}}
\Big)
\;\|\; \mathrm{operators/tools}
\]

All integer-hot-path. SoftCascade is the latency-preserving decode. That is the
bridge — not bolting on a token sampler first.
