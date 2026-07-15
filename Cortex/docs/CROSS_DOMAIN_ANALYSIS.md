# Cross-domain analysis: attention, inhibition, and computational work

Cortex uses biological language as an engineering analogy only. It does not model consciousness, reproduce thalamic physiology, or measure biological or physical energy.

## Evidence-informed mapping

| Research observation | Cortex analogue | Measurable Cortex signal |
|---|---|---|
| Higher-order thalamic systems can coordinate task-relevant interactions across cortical regions. | Thalamus route plans weight source, tests, structure, documentation, Git, and runtime lanes. | Route weights, selected evidence lanes, top-k recall. |
| Reticular-thalamic inhibition can gate competing transmission. | Evidence inhibition suppresses generated, duplicate, out-of-route, and hard-excluded material. | Inhibition audit, pruned candidates, empty-packet fallback. |
| Pulvinar-cortical coupling is associated with coordination across functionally specialized networks. | Structural graph and sparse interlink expand direct retrieval only along evidence-bearing relationships. | Support paths, nodes considered, graph coverage. |
| Sparse context selection can reduce retrieval-augmented inference work while retaining useful context. | Cortex emits bounded provenance-backed packets instead of broad repository loading. | Token budget fraction, candidate count, node scan fraction. |

## Mechanical-efficiency interpretation

For Cortex, “energy” means computational work proxies—not watts, metabolic energy, or biological cost:

```text
work proxy = candidate vectors compared + graph nodes considered + context tokens emitted + storage bytes retained
```

The implementation reports these counters in each context packet under `efficiency`. Improvements must demonstrate a quality-preserving reduction in one or more counters; reducing work while harming recall is not an improvement.

## Design rules

1. Keep source, tests, and current runtime evidence above learned or compressed memory.
2. Use inhibition to reduce irrelevant work, but preserve a bounded fallback when a route is uncertain.
3. Evaluate retrieval with task-relevant file rank and downstream task evidence, not only latency.
4. Treat feedback as a ranking signal, never as truth or authority.
5. Keep the Governor independent: improved routing cannot relax read-only behavior under drift.

## Sources

- [Halassa & Kastner, *Thalamic functions in distributed cognitive control*](https://www.nature.com/articles/s41593-017-0020-1)
- [Acsády & Halassa, *Thalamic inhibition: diverse sources, diverse scales*](https://pmc.ncbi.nlm.nih.gov/articles/PMC5048590/)
- [Arcaro et al., *Organizing principles of pulvino-cortical functional coupling*](https://www.nature.com/articles/s41467-018-07725-6)
- [Zhu et al., *Accelerating Inference of Retrieval-Augmented Generation via Sparse Context Selection*](https://arxiv.org/abs/2405.16178)

These sources motivate analogies and measurement choices; they do not validate Cortex as a biological replica.
