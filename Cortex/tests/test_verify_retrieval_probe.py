from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from cortex.verify import _retrieval_probes


class FakeStore:
    def symbols(self, repo: str):
        return [{"name": "PerciSymbol", "path": "src/perci_symbol.rs"}]


class RetrievalProbeTests(unittest.TestCase):
    def test_targeted_path_fallback_preserves_strict_global_health(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "README.md").write_text(
                "# Perci\n\nRepository intelligence.\n",
                encoding="utf-8",
            )

            global_hit = SimpleNamespace(path="knowledge/other.md")

            def targeted(store, repo, text, paths, limit=1):
                return [SimpleNamespace(path=paths[0])]

            with patch("cortex.verify.query", return_value=[global_hit]):
                with patch("cortex.verify.support_hits", side_effect=targeted):
                    result = _retrieval_probes(
                        FakeStore(),
                        "Perci",
                        root,
                    )

            self.assertEqual(result["probe_count"], 2)
            self.assertEqual(result["pass_rate"], 1.0)
            self.assertTrue(all(item["passed"] for item in result["results"]))
            self.assertTrue(
                all(item["selection"] == "targeted" for item in result["results"])
            )

    def test_empty_global_retrieval_still_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "README.md").write_text("# Perci\n", encoding="utf-8")

            with patch("cortex.verify.query", return_value=[]):
                with patch(
                    "cortex.verify.support_hits",
                    return_value=[SimpleNamespace(path="README.md")],
                ):
                    result = _retrieval_probes(
                        FakeStore(),
                        "Perci",
                        root,
                    )

            self.assertLess(result["pass_rate"], 1.0)
            self.assertTrue(any(not item["passed"] for item in result["results"]))


if __name__ == "__main__":
    unittest.main()