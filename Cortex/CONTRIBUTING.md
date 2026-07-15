# Contributing

1. Create a focused branch.
2. Preserve the single-substrate invariant: no competing repository, episodic, or consolidation database.
3. Add or update tests for behavior changes.
4. Run `python -m compileall -q cortex tests`.
5. Run `python -m unittest discover -s tests -v`.
6. Run Bash syntax checks on `cortex.sh`, `cortex-all-one.sh`, and `scripts/bash/*.sh`.
7. Preserve the authority boundary: Cortex and its neural interlink may retrieve, route, and verify memory but may not authorize source mutation.
8. Keep parsers and environment detectors bounded, failure-tolerant, and explicit about unsupported syntax.
9. Keep neural plasticity bounded and restricted to existing compiled relationships.
