from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from cortex.activation import activate_repository
from cortex.bootstrap import bootstrap_repository
from cortex.bridge import consolidate
from cortex.config import ensure_home, load_repo_config
from cortex.embeddings import VECTOR_MAGIC, deserialize_vector
from cortex.indexer import should_exclude
from cortex.parsers import language_for, parse_structure
from cortex.governor import Governor
from cortex.graph import neighborhood
from cortex.hippocampus import remember
from cortex.retrieval import query
from cortex.store import Store
from cortex.verify import verify_repository


class CortexIntegrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.home = ensure_home(self.base / "home")
        self.repo = self.base / "demo-repo"
        self.repo.mkdir()
        (self.repo / "README.md").write_text(
            "# Demo Project\n\n"
            "## Architecture\n\nThe service imports a helper and exposes a greeting.\n\n"
            "## Usage\n\nRun the application through app.py.\n",
            encoding="utf-8",
        )
        (self.repo / "helper.py").write_text(
            "def format_name(name: str) -> str:\n    return name.strip().title()\n",
            encoding="utf-8",
        )
        (self.repo / "app.py").write_text(
            "from helper import format_name\n\n"
            "def greet(name: str) -> str:\n"
            "    return f'Hello, {format_name(name)}'\n",
            encoding="utf-8",
        )
        tests = self.repo / "tests"
        tests.mkdir()
        (tests / "test_app.py").write_text(
            "from app import greet\n\n"
            "def test_greet():\n    assert greet('james') == 'Hello, James'\n",
            encoding="utf-8",
        )
        self.store = Store(self.home / "cortex.db")

    def tearDown(self) -> None:
        self.store.close()
        self.temp.cleanup()

    def bootstrap(self) -> dict:
        return bootstrap_repository(self.home, self.store, self.repo, "DemoProject")

    def test_bootstrap_installs_integration_and_issues_certificate(self) -> None:
        result = self.bootstrap()
        certificate = result["certificate"]
        self.assertEqual(certificate["status"], "verified", certificate)
        self.assertTrue(certificate["checks"]["database_integrity"])
        self.assertTrue(certificate["checks"]["manifest_integrity"])
        self.assertGreaterEqual(certificate["coverage"]["index_coverage"], 0.98)
        self.assertTrue((self.repo / ".cortex" / "config.json").exists())
        self.assertTrue((self.repo / ".cortex" / "bootstrap_certificate.json").exists())
        self.assertTrue((self.repo / ".cortex" / "bin" / "cortex.ps1").exists())
        self.assertTrue((self.repo / ".cortex" / "bin" / "cortex.sh").exists())
        config = json.loads((self.repo / ".cortex" / "config.json").read_text(encoding="utf-8"))
        self.assertEqual(Path(config["engine_python"]).resolve(), Path(sys.executable).resolve())
        self.assertTrue(Path(config["engine_module_root"]).exists())
        self.assertEqual(config["cortex_home"], str(self.home.resolve()))
        bash_wrapper = (self.repo / ".cortex" / "bin" / "cortex.sh").read_text(encoding="utf-8")
        powershell_wrapper = (self.repo / ".cortex" / "bin" / "cortex.ps1").read_text(encoding="utf-8")
        self.assertIn(str(Path(sys.executable)), bash_wrapper)
        self.assertIn(str(Path(sys.executable)), powershell_wrapper)
        self.assertNotIn("__CORTEX_", bash_wrapper)
        self.assertNotIn("__CORTEX_", powershell_wrapper)
        agents = (self.repo / "AGENTS.md").read_text(encoding="utf-8")
        self.assertIn("CORTEX:MANAGED:BEGIN", agents)
        self.assertIn(r".\.cortex\bin\cortex.ps1 activate -Task", agents)

    def test_query_returns_provenance_and_structural_neighbors(self) -> None:
        self.bootstrap()
        hits = query(self.store, "DemoProject", "format greeting name", limit=5)
        self.assertTrue(hits)
        self.assertIn("app.py", {hit.path for hit in hits})
        self.assertTrue(all(hit.content_hash for hit in hits))
        graph = neighborhood(self.store, "DemoProject", ["app.py"], limit=30)
        relations = {edge["relation"] for edge in graph}
        self.assertIn("resolves_to", relations)
        self.assertIn("tested_by", relations)

    def test_activation_refreshes_repository_drift(self) -> None:
        self.bootstrap()
        (self.repo / "app.py").write_text(
            "from helper import format_name\n\n"
            "def greet(name: str) -> str:\n"
            "    return f'Welcome, {format_name(name)}'\n",
            encoding="utf-8",
        )
        governor = Governor(self.home, self.store)
        result = activate_repository(
            self.home,
            self.store,
            governor,
            "DemoProject",
            "Why did the greeting change?",
            refresh="auto",
        )
        self.assertEqual(result["bootstrap_status"], "verified", result)
        self.assertIsNotNone(result["refresh"])
        self.assertTrue(result["manifest_current"])
        self.assertEqual(result["context"]["governor"]["mode"], "constrained")
        self.assertIn(result["context"]["thalamus"]["primary_intent"], {"code_change", "historical_inquiry"})
        self.assertTrue((self.repo / ".cortex" / "runtime" / "context_latest.json").exists())

    def test_refresh_never_forces_read_only_on_drift(self) -> None:
        self.bootstrap()
        (self.repo / "helper.py").write_text("def format_name(name):\n    return name.upper()\n", encoding="utf-8")
        governor = Governor(self.home, self.store)
        result = activate_repository(
            self.home,
            self.store,
            governor,
            "DemoProject",
            "Inspect the helper change",
            refresh="never",
        )
        self.assertFalse(result["manifest_current"])
        self.assertEqual(result["context"]["governor"]["mode"], "read_only")

    def test_current_degraded_certificate_controls_governor(self) -> None:
        self.bootstrap()
        governor = Governor(self.home, self.store)
        result = governor.evaluate("DemoProject", manifest_current=True, certificate={"status": "degraded"})
        self.assertEqual(result["mode"], "read_only")

    def test_session_events_consolidate_to_discovery_card(self) -> None:
        self.bootstrap()
        governor = Governor(self.home, self.store)
        activation = activate_repository(
            self.home, self.store, governor, "DemoProject", "Document greeting ownership"
        )
        session_id = activation["session"]["session_id"]
        remember(
            self.home,
            self.store,
            "DemoProject",
            "decision",
            "app.py owns the public greeting interface.",
            session_id,
        )
        remember(
            self.home,
            self.store,
            "DemoProject",
            "outcome",
            "The ownership decision was documented and verified against source.",
            session_id,
        )
        result = consolidate(self.home, self.store, "DemoProject", session_id)
        self.assertTrue(result["created"])
        self.assertTrue(Path(result["path"]).exists())
        hits = query(self.store, "DemoProject", "public greeting interface ownership", limit=10)
        self.assertIn("discovery_card", {hit.kind for hit in hits})


    def test_rebootstrap_preserves_custom_repository_configuration(self) -> None:
        self.bootstrap()
        config_path = self.repo / ".cortex" / "config.json"
        config = json.loads(config_path.read_text(encoding="utf-8"))
        config["exclude"].append("private-notes")
        config["context_budget"] = 777
        config_path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
        result = bootstrap_repository(self.home, self.store, self.repo, "DemoProject")
        persisted = json.loads(config_path.read_text(encoding="utf-8"))
        self.assertIn("private-notes", persisted["exclude"])
        self.assertEqual(persisted["context_budget"], 777)
        self.assertEqual(result["certificate"]["status"], "verified")

    def test_reusing_name_for_different_repository_clears_old_memory(self) -> None:
        self.bootstrap()
        other = self.base / "other-repo"
        other.mkdir()
        (other / "README.md").write_text("# Other Project\n\n## Unique\n\nSecond repository.\n", encoding="utf-8")
        (other / "unique.py").write_text("def second_only():\n    return True\n", encoding="utf-8")
        result = bootstrap_repository(self.home, self.store, other, "DemoProject")
        self.assertEqual(result["certificate"]["status"], "verified")
        paths = {row["path"] for row in self.store.files("DemoProject")}
        self.assertIn("unique.py", paths)
        self.assertNotIn("app.py", paths)

    def test_verify_reports_unsupported_surface(self) -> None:
        (self.repo / "archive.bin").write_bytes(b"\x00\x01\x02")
        self.bootstrap()
        config = load_repo_config(self.repo)
        certificate = verify_repository(
            self.home, self.store, "DemoProject", config, write_certificate=False
        )
        unresolved = certificate["coverage"]["unresolved_files"]
        self.assertTrue(any(item["path"] == "archive.bin" for item in unresolved))
        self.assertEqual(certificate["status"], "verified")

    def test_secret_exclusions_and_kotlin_parsing(self) -> None:
        config = load_repo_config(self.repo) if (self.repo / ".cortex" / "config.json").exists() else None
        if config is None:
            from cortex.config import RepoConfig
            config = RepoConfig()
        self.assertTrue(should_exclude("services/.env.production", config))
        self.assertTrue(should_exclude("keys/server.pem", config))
        self.assertEqual(language_for(Path("Feature.kt")), "kotlin")
        symbols, edges = parse_structure("import demo.core.Service\nclass Feature\nfun start() = 1\n", "Feature.kt", "kotlin")
        self.assertEqual([edge.target for edge in edges], ["demo.core.Service"])
        self.assertIn("Feature", {symbol["name"] for symbol in symbols})

    def test_vector_migration_preserves_legacy_semantics(self) -> None:
        self.store.attach("VectorProject", "vector-project", self.repo)
        self.store.db.execute(
            "INSERT INTO memories(repo,path,chunk_index,start_line,end_line,kind,text,content_hash,vector,embedding_model,metadata,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
            ("VectorProject", "legacy.py", 0, 1, 1, "source", "x", "legacy", json.dumps([0.25, -0.5]), "legacy", "{}", 1.0, 1.0),
        )
        self.store.commit()
        result = self.store.migrate_vectors("VectorProject")
        row = self.store.db.execute("SELECT vector FROM memories WHERE repo=?", ("VectorProject",)).fetchone()
        self.assertEqual(result["migrated"], 1)
        self.assertTrue(row["vector"].startswith(VECTOR_MAGIC))
        self.assertAlmostEqual(deserialize_vector(row["vector"])[0], 0.25, places=6)
        self.assertEqual(self.store.vector_format_status("VectorProject")["legacy_or_invalid"], 0)

    def test_repository_sensitive_exclusions_extend_defaults(self) -> None:
        from cortex.config import RepoConfig
        config = RepoConfig.from_dict({"sensitive_exclude_patterns": ["private/token.txt"]})
        self.assertTrue(should_exclude("private/token.txt", config))
        self.assertTrue(should_exclude("keys/client.key", config))


class CortexGitTelemetryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.base = Path(self.temp.name)
        self.home = ensure_home(self.base / "home")
        self.repo = self.base / "git-repo"
        self.repo.mkdir()
        self._git("init")
        self._git("config", "user.email", "cortex@example.test")
        self._git("config", "user.name", "Cortex Test")
        (self.repo / "README.md").write_text("# Git Project\n\n## History\n\nTelemetry test.\n", encoding="utf-8")
        (self.repo / "main.py").write_text("def run():\n    return 1\n", encoding="utf-8")
        (self.repo / "util.py").write_text("def helper():\n    return 2\n", encoding="utf-8")
        self._git("add", ".")
        self._git("commit", "-m", "initial")
        (self.repo / "main.py").write_text("from util import helper\ndef run():\n    return helper()\n", encoding="utf-8")
        (self.repo / "util.py").write_text("def helper():\n    return 3\n", encoding="utf-8")
        self._git("add", ".")
        self._git("commit", "-m", "link main and util")
        self.store = Store(self.home / "cortex.db")

    def _git(self, *args: str) -> None:
        subprocess.run(
            ["git", "-C", str(self.repo), *args],
            check=True,
            capture_output=True,
            text=True,
        )

    def tearDown(self) -> None:
        self.store.close()
        self.temp.cleanup()

    def test_git_history_and_cochange_are_ingested(self) -> None:
        result = bootstrap_repository(self.home, self.store, self.repo, "GitProject")
        telemetry = result["telemetry"]
        self.assertTrue(telemetry["available"])
        self.assertGreaterEqual(telemetry["commits_ingested"], 2)
        self.assertGreaterEqual(len(self.store.file_telemetry("GitProject")), 2)
        edges = self.store.edges("GitProject", relation="co_changed", limit=100)
        self.assertTrue(edges)

    def test_telemetry_refuses_an_ancestor_worktree(self) -> None:
        nested = self.repo / "nested-target"
        nested.mkdir()
        (nested / "app.py").write_text("def nested():\n    return True\n", encoding="utf-8")
        result = bootstrap_repository(self.home, self.store, nested, "NestedProject")
        self.assertFalse(result["telemetry"]["available"])
        self.assertIn("not the Git work-tree root", result["telemetry"]["reason"])


class CortexCliTests(unittest.TestCase):
    def test_cli_and_repository_platform_wrapper(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            base = Path(temp)
            home = base / "home"
            repo = base / "cli-repo"
            repo.mkdir()
            (repo / "README.md").write_text(
                "# CLI Project\n\n## Commands\n\nThe command lives in tool.py.\n",
                encoding="utf-8",
            )
            (repo / "tool.py").write_text("def command():\n    return 'ok'\n", encoding="utf-8")
            env = os.environ.copy()
            env.pop("PYTHONPATH", None)
            # A repository-local wrapper must remain bound to the home recorded at
            # bootstrap rather than being silently redirected by an inherited value.
            env["CORTEX_HOME"] = str(base / "unrelated-global-home")
            fake_python = base / "unrelated-python"
            fake_python.write_text("#!/usr/bin/env sh\nexit 99\n", encoding="utf-8")
            fake_python.chmod(0o755)
            env["CORTEX_PYTHON"] = str(fake_python)
            bootstrap = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "cortex",
                    "--home",
                    str(home),
                    "bootstrap",
                    str(repo),
                    "--name",
                    "CliProject",
                    "--json",
                ],
                check=True,
                capture_output=True,
                text=True,
                env=env,
            )
            payload = json.loads(bootstrap.stdout)
            self.assertEqual(payload["certificate"]["status"], "verified")
            if os.name == "nt":
                wrapper_command = [
                    "powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
                    str(repo / ".cortex" / "bin" / "cortex.ps1"), "-Command", "activate",
                    "-Task", "Where is the command?",
                ]
            else:
                wrapper_command = [
                    str(repo / ".cortex" / "bin" / "cortex.sh"), "activate", "--task",
                    "Where is the command?",
                ]
            wrapper_result = subprocess.run(
                wrapper_command,
                check=True,
                capture_output=True,
                text=True,
                env=env,
                cwd=repo,
            )
            activated = json.loads(wrapper_result.stdout)
            self.assertEqual(activated["bootstrap_status"], "verified")
            self.assertTrue(activated["context"]["evidence"])


if __name__ == "__main__":
    unittest.main()
