# Thalamus routing

The root-level `thalamus` package is Cortex's local-first request routing organ. It is a software design inspired by attention and inhibitory gating, not a claim of biological fidelity.

For each normal context activation, Cortex creates a normalized request, deterministically classifies its intent, allocates weights across memory lanes, then gates retrieved evidence. The emitted context packet contains the route plan, confidence, uncertainty, per-lane budgets, and evidence-level inhibition audit data.

Hard exclusions suppress generated runtime packets and common build/vendor directories. Source provenance remains unchanged; routing only changes retrieval priority. Thalamus cannot mutate source code or grant authority.

Use `cortex thalamus --repo <repository> --task "<task>" --json` to preview the route without building a context packet.
