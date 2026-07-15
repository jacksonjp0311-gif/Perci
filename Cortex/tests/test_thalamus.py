from __future__ import annotations

import unittest

from cortex.models import Hit
from cortex.store import Store
from thalamus import apply_feedback, record_feedback
from thalamus import inhibit, make_request, route


class ThalamusTests(unittest.TestCase):
    def test_debug_route_prioritizes_failure_and_test_lanes(self) -> None:
        repository = {"repository_id": "repo-1"}
        request = make_request(repository, "Fix the failing login test", 1200, recent_errors=("AssertionError",))
        plan = route(request, manifest_current=True)
        self.assertEqual(plan.primary_intent, "debug")
        self.assertGreater(plan.lane_weights["failures"], plan.lane_weights["documentation"])
        self.assertGreater(plan.evidence_budget["tests"], 0)
        self.assertFalse(plan.requires_refresh)

    def test_route_is_deterministic_and_inhibition_is_auditable(self) -> None:
        repository = {"repository_id": "repo-1"}
        first = route(make_request(repository, "Map the architecture", 900), manifest_current=True)
        second = route(make_request(repository, "Map the architecture", 900), manifest_current=True)
        self.assertEqual(first.to_dict(), second.to_dict())
        hit = Hit(1, "Demo", "src/app.py", 1, 2, "def main(): pass", "source", 1.0, "hash")
        gated = inhibit([hit], first.lane_weights)
        self.assertEqual(len(gated), 1)
        self.assertIn("thalamus", gated[0].metadata)
        self.assertLessEqual(gated[0].metadata["thalamus"]["inhibition"], 1.0)

    def test_generated_runtime_evidence_is_hard_excluded(self) -> None:
        hit = Hit(1, "Demo", ".cortex/runtime/context_latest.json", 1, 1, "{}", "runtime", 1.0, "hash")
        self.assertEqual(inhibit([hit], {"source": 1.0}), [])

    def test_feedback_is_bounded_and_changes_only_routing_score(self) -> None:
        import tempfile
        from pathlib import Path

        with tempfile.TemporaryDirectory() as temporary:
            store = Store(Path(temporary) / "cortex.db")
            try:
                recorded = record_feedback(store, "Demo", 1, "helpful")
                self.assertGreater(recorded["score"], 0.0)
                hit = Hit(1, "Demo", "app.py", 1, 1, "x", "source", 1.0, "hash")
                self.assertGreater(apply_feedback(store, "Demo", [hit])[0].score, 1.0)
            finally:
                store.close()


if __name__ == "__main__":
    unittest.main()
