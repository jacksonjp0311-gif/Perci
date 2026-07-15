from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from cortex.bootstrap import bootstrap_repository
from cortex.config import ensure_home, load_repo_config
from cortex.context import build_context, nexus_packet
from cortex.governor import Governor
from cortex.models import Hit
from cortex.neuron import activate_interlink
from cortex.learning import record_outcome
from cortex.retrieval import query
from cortex.store import Store


class CortexNeuralInterlinkTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.home = ensure_home(self.base / "home")
        self.repo = self.base / "agent-repo"
        self.repo.mkdir()
        (self.repo / "README.md").write_text(
            "# Agent Repository\n\n## Architecture\n\nThe planner calls the memory bridge.\n",
            encoding="utf-8",
        )
        (self.repo / "planner.py").write_text(
            "from memory_bridge import retrieve\n\n"
            "def plan(task: str) -> str:\n"
            "    return retrieve(task)\n",
            encoding="utf-8",
        )
        (self.repo / "memory_bridge.py").write_text(
            "def retrieve(task: str) -> str:\n"
            "    return f'memory:{task}'\n",
            encoding="utf-8",
        )
        tests = self.repo / "tests"
        tests.mkdir()
        (tests / "test_planner.py").write_text(
            "from planner import plan\n\n"
            "def test_plan():\n"
            "    assert plan('x') == 'memory:x'\n",
            encoding="utf-8",
        )
        (self.repo / "pyproject.toml").write_text(
            "[project]\nname='agent-repo'\nversion='0.1.0'\n"
            "dependencies=['pytest>=8']\n\n"
            "[project.scripts]\nagent-run='planner:plan'\n",
            encoding="utf-8",
        )
        self.store = Store(self.home / "cortex.db")
        self.bootstrap = bootstrap_repository(
            self.home, self.store, self.repo, "AgentRepo"
        )

    def tearDown(self) -> None:
        self.store.close()
        self.temp.cleanup()

    def test_bootstrap_learns_environment_and_compiles_single_substrate(self) -> None:
        environment = self.bootstrap["environment"]
        neural = self.bootstrap["neural_interlink"]
        self.assertIn("python", environment["ecosystems"])
        self.assertTrue(any(item["name"] == "python" for item in environment["inventory"]["languages"]))
        self.assertGreaterEqual(neural["nodes"], 5)
        self.assertGreaterEqual(neural["synapses"], 2)
        self.assertEqual(neural["node_coverage"], 1.0)
        self.assertTrue(neural["ledger_valid"])
        self.assertTrue((self.repo / ".cortex" / "runtime" / "environment_latest.json").exists())
        self.assertFalse((self.home / "neuron.db").exists())

    def test_sparse_activation_is_deterministic_without_plasticity(self) -> None:
        hits = query(self.store, "AgentRepo", "planner memory bridge", limit=12)
        first = activate_interlink(
            self.store,
            "AgentRepo",
            "planner memory bridge",
            hits,
            plasticity_enabled=False,
            governance_mode="read_only",
        )
        second = activate_interlink(
            self.store,
            "AgentRepo",
            "planner memory bridge",
            hits,
            plasticity_enabled=False,
            governance_mode="read_only",
        )
        self.assertEqual(first.state_hash, second.state_hash)
        self.assertEqual(first.fired_paths, second.fired_paths)
        self.assertLessEqual(first.metrics["nodes_considered"], first.metrics["total_nodes"])
        self.assertIn("planner.py", first.fired_paths)

    def test_structural_interconnection_activates_nonretrieved_support(self) -> None:
        row = self.store.memories_for_path("AgentRepo", "planner.py")[0]
        seed = Hit(
            memory_id=int(row["id"]),
            repo=row["repo"],
            path=row["path"],
            start_line=int(row["start_line"]),
            end_line=int(row["end_line"]),
            text=row["text"],
            kind=row["kind"],
            score=1.0,
            content_hash=row["content_hash"],
            metadata={"semantic_similarity": 1.0},
        )
        packet = activate_interlink(
            self.store,
            "AgentRepo",
            "planner retrieve",
            [seed],
            plasticity_enabled=False,
            governance_mode="read_only",
        )
        self.assertIn("memory_bridge.py", packet.support_paths)
        self.assertIn("memory_bridge.py", packet.fired_paths)

    def test_verified_outcome_is_bounded_replay_gated_and_ledgered(self) -> None:
        hits = query(self.store, "AgentRepo", "planner retrieve memory", limit=12)
        before = {
            row["synapse_id"]: float(row["weight"])
            for row in self.store.neural_synapses("AgentRepo")
        }
        packet = activate_interlink(
            self.store,
            "AgentRepo",
            "planner retrieve memory",
            hits,
            plasticity_enabled=True,
            governance_mode="normal",
            learning_rate=0.25,
        )
        # Activation itself is observational in v2: learning requires verified outcome.
        self.assertEqual(before, {row["synapse_id"]: float(row["weight"]) for row in self.store.neural_synapses("AgentRepo")})
        result = record_outcome(
            self.store, "AgentRepo", packet.activation_id, status="verified",
            verification_type="pytest", governance_mode="normal",
        )
        after_rows = self.store.neural_synapses("AgentRepo")
        self.assertTrue(self.store.verify_neural_ledger("AgentRepo"))
        self.assertGreater(result["credited_synapses"], 0)
        self.assertTrue(result["replay"]["accepted"])
        self.assertGreater(result["accepted_updates"], 0)
        for row in after_rows:
            self.assertGreaterEqual(float(row["weight"]), float(row["minimum_weight"]))
            self.assertLessEqual(float(row["weight"]), float(row["maximum_weight"]))
            self.assertGreaterEqual(float(row["weight"]), before[row["synapse_id"]])

    def test_context_and_nexus_packet_include_environment_and_interlink(self) -> None:
        governor = Governor(self.home, self.store)
        context = build_context(
            self.home,
            self.store,
            governor,
            "AgentRepo",
            "Trace the planner through the memory bridge",
            1200,
            manifest_current=True,
        )
        self.assertTrue(context["environment"]["available"])
        self.assertIn("activation_id", context["neural_interlink"])
        self.assertLessEqual(context["efficiency"]["node_scan_fraction"], 1.0)
        self.assertLessEqual(context["efficiency"]["context_budget_fraction"], 1.0)
        self.assertTrue(context["evidence"])
        packet = nexus_packet(context)
        self.assertIn("neural_interlink", packet["context"])
        self.assertFalse(packet["authority"]["cortex_may_mutate"])
        self.assertTrue(packet["authority"]["human_authorized_only"])
        from cortex.context import cortex_context_protocol
        protocol = cortex_context_protocol(context)
        self.assertEqual("cortex-context/1.0", protocol["protocol"])
        self.assertTrue(protocol["prohibited_actions"])

    def test_neural_ledger_detects_tampering(self) -> None:
        row = self.store.db.execute(
            "SELECT id FROM neural_ledger WHERE repo=? ORDER BY sequence LIMIT 1",
            ("AgentRepo",),
        ).fetchone()
        self.assertIsNotNone(row)
        self.store.db.execute(
            "UPDATE neural_ledger SET payload=? WHERE id=?",
            (json.dumps({"tampered": True}), row["id"]),
        )
        self.store.db.commit()
        self.assertFalse(self.store.verify_neural_ledger("AgentRepo"))

    def test_embedded_engine_directory_is_excluded_from_host_assimilation(self) -> None:
        nested = self.repo / "CortexEngine"
        (nested / "cortex").mkdir(parents=True)
        (nested / "cortex" / "fake.py").write_text("SECRET_ENGINE_SENTINEL = True\n", encoding="utf-8")

        # Simulate a portable engine path nested in a host by temporarily rebinding the recorded module root.
        config = load_repo_config(self.repo)
        config.engine_module_root = str(nested)
        if "CortexEngine" not in config.exclude:
            config.exclude.append("CortexEngine")
        from cortex.config import save_repo_config
        from cortex.indexer import index_repository

        save_repo_config(self.repo, config)
        index_repository(self.store, "AgentRepo", config, force=True)
        paths = {row["path"] for row in self.store.files("AgentRepo")}
        self.assertNotIn("CortexEngine/cortex/fake.py", paths)


if __name__ == "__main__":
    unittest.main()
