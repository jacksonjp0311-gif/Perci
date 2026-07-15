from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

DEFAULT_EXCLUDES = [
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    ".venv",
    "venv",
    "env",
    "node_modules",
    "dist",
    "build",
    "target",
    "coverage",
    ".coverage",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "__pycache__",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    ".cortex/bin",
    ".cortex/runtime",
    ".cortex/config.json",
    ".cortex/bootstrap_certificate.json",
    ".cortex/README.md",
    ".cortex/.gitignore",
    "reports/*_latest.json",
    "state/*_latest.json",
    "ledger",
    "ledgers",
    "Tesseract Neural Network/memory",
    "Tesseract Neural Network/state",
    # ── Secret / credential files — never index these ──
    ".env",
    ".env.local",
    ".env.production",
    ".env.staging",
    ".env.development",
    ".aws/credentials",
    ".aws/config",
    ".ssh/id_rsa",
    ".ssh/id_ed25519",
    ".ssh/id_ecdsa",
    ".ssh/id_dsa",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".docker/config.json",
    "credentials.json",
    "credentials.plist",
    "service-account.json",
    "firebase-adminsdk*.json",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.keystore",
    "*.jks",
    ".gcloud/credentials.db",
    ".kube/config",
    ".git-credentials",
    ".npm/_token*",
    ".dvc/config",
]

DEFAULT_TEXT_EXTENSIONS = [
    ".py", ".pyi", ".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs",
    ".java", ".kt", ".kts", ".go", ".rs", ".c", ".h", ".cpp", ".hpp",
    ".cs", ".fs", ".fsx", ".rb", ".php", ".swift", ".scala", ".lua",
    ".ps1", ".psm1", ".psd1", ".sh", ".bash", ".zsh", ".fish",
    ".md", ".mdx", ".rst", ".txt", ".adoc", ".tex",
    ".json", ".jsonc", ".yaml", ".yml", ".toml", ".ini", ".cfg", ".conf",
    ".xml", ".html", ".css", ".scss", ".sass", ".less", ".sql", ".graphql",
    ".proto", ".dockerfile", ".gradle", ".properties", ".env.example",
]

SPECIAL_TEXT_FILES = {
    "Dockerfile", "Makefile", "Rakefile", "Gemfile", "Procfile", "Justfile",
    "AGENTS.md", "CODEOWNERS", "LICENSE", "NOTICE", "README", "CHANGELOG",
}

RUNTIME_EVIDENCE_HINTS = {
    "reports", "logs", "ledger", "ledgers", "state", "handoff", "handoffs",
    "artifacts", "results", "failures", "wounds", "certificates", "telemetry",
}


@dataclass
class RepoConfig:
    schema_version: str = "1.0"
    repository_name: str = ""
    repository_id: str = ""
    repository_root: str = "."
    engine_python: str = ""
    engine_module_root: str = ""
    cortex_home: str = ""
    memory_mode: str = "index_first"
    context_budget: int = 1200
    query_before_action: bool = True
    refresh_on_activation: str = "auto"
    max_file_bytes: int = 2_000_000
    chunk_chars: int = 4_800
    chunk_overlap_lines: int = 8
    git_commit_limit: int = 500
    semantic_scan_limit: int = 5000
    environment_learning_enabled: bool = True
    thalamus_enabled: bool = True
    thalamus_min_lane_relevance: float = 0.25
    sensitive_exclude_patterns: list[str] = field(default_factory=list)
    neural_interlink_enabled: bool = True
    neural_activation_depth: int = 2
    neural_max_nodes: int = 64
    neural_plasticity_enabled: bool = True
    neural_learning_rate: float = 0.025
    bootstrap_thresholds: dict[str, float] = field(default_factory=lambda: {
        "index_coverage": 0.98,
        "manifest_integrity": 1.0,
        "retrieval_probe_pass_rate": 0.75,
    })
    exclude: list[str] = field(default_factory=lambda: list(DEFAULT_EXCLUDES))
    include_extensions: list[str] = field(default_factory=lambda: list(DEFAULT_TEXT_EXTENSIONS))
    authoritative_paths: list[str] = field(default_factory=lambda: [
        "README.md", "AGENTS.md", "pyproject.toml", "package.json", "Cargo.toml",
        "go.mod", "pom.xml", "build.gradle", "Makefile", "Dockerfile",
    ])
    runtime_evidence_paths: list[str] = field(default_factory=lambda: [
        "reports", "logs", "state", "ledger", "handoff", "artifacts",
    ])

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "repository_name": self.repository_name,
            "repository_id": self.repository_id,
            "repository_root": self.repository_root,
            "engine_python": self.engine_python,
            "engine_module_root": self.engine_module_root,
            "cortex_home": self.cortex_home,
            "memory_mode": self.memory_mode,
            "context_budget": self.context_budget,
            "query_before_action": self.query_before_action,
            "refresh_on_activation": self.refresh_on_activation,
            "max_file_bytes": self.max_file_bytes,
            "chunk_chars": self.chunk_chars,
            "chunk_overlap_lines": self.chunk_overlap_lines,
            "git_commit_limit": self.git_commit_limit,
            "semantic_scan_limit": self.semantic_scan_limit,
            "environment_learning_enabled": self.environment_learning_enabled,
            "thalamus_enabled": self.thalamus_enabled,
            "thalamus_min_lane_relevance": self.thalamus_min_lane_relevance,
            "sensitive_exclude_patterns": self.sensitive_exclude_patterns,
            "neural_interlink_enabled": self.neural_interlink_enabled,
            "neural_activation_depth": self.neural_activation_depth,
            "neural_max_nodes": self.neural_max_nodes,
            "neural_plasticity_enabled": self.neural_plasticity_enabled,
            "neural_learning_rate": self.neural_learning_rate,
            "bootstrap_thresholds": self.bootstrap_thresholds,
            "exclude": self.exclude,
            "include_extensions": self.include_extensions,
            "authoritative_paths": self.authoritative_paths,
            "runtime_evidence_paths": self.runtime_evidence_paths,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "RepoConfig":
        base = cls()
        for key, value in data.items():
            if hasattr(base, key):
                setattr(base, key, value)
        # Configuration is additive across releases so older repositories receive
        # newly introduced volatile-surface protections without losing custom rules.
        base.exclude = list(dict.fromkeys([*base.exclude, *DEFAULT_EXCLUDES]))
        base.exclude = list(dict.fromkeys([*base.exclude, *base.sensitive_exclude_patterns]))
        return base


def cortex_home() -> Path:
    configured = os.environ.get("CORTEX_HOME")
    return Path(configured or (Path.home() / ".cortex")).expanduser().resolve()


def ensure_home(path: Path | None = None) -> Path:
    home = (path or cortex_home()).expanduser().resolve()
    for child in ["cards", "packets", "logs", "certificates", "sessions"]:
        (home / child).mkdir(parents=True, exist_ok=True)
    return home


def repo_config_path(root: Path) -> Path:
    return root / ".cortex" / "config.json"


def load_repo_config(root: Path) -> RepoConfig:
    path = repo_config_path(root)
    if not path.exists():
        raise FileNotFoundError(f"Cortex repository config not found: {path}")
    return RepoConfig.from_dict(json.loads(path.read_text(encoding="utf-8")))


def save_repo_config(root: Path, config: RepoConfig) -> Path:
    path = repo_config_path(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(config.to_dict(), indent=2) + "\n", encoding="utf-8")
    return path
