from __future__ import annotations

import re
from collections import Counter


INTENT_SIGNALS: dict[str, tuple[str, ...]] = {
    "debug": ("bug", "error", "fail", "failure", "exception", "crash", "broken", "regression"),
    "code_change": ("add", "implement", "change", "modify", "refactor", "fix", "build"),
    "architecture": ("architecture", "design", "how does", "flow", "component", "module", "map"),
    "documentation": ("document", "docs", "readme", "explain", "guide"),
    "testing": ("test", "pytest", "unittest", "coverage", "assert"),
    "release": ("release", "publish", "version", "package", "wheel", "changelog"),
    "configuration": ("config", "configuration", "setting", "environment", "env", "toml", "yaml"),
    "security_review": ("security", "vulnerability", "secret", "credential", "auth", "permission"),
    "historical_inquiry": ("history", "yesterday", "previous", "when", "commit", "changed"),
    "repository_orientation": ("where", "find", "locate", "entrypoint", "orient", "overview"),
    "memory_maintenance": ("consolidate", "replay", "memory", "maintain", "prune"),
}


def classify(task: str, *, recent_errors: tuple[str, ...] = (), active_files: tuple[str, ...] = ()) -> tuple[str, tuple[str, ...], float]:
    """Return a deterministic intent ranking using lexical and local request signals."""

    tokens = set(re.findall(r"[a-z0-9_./-]+", task.lower()))
    scores = Counter({intent: len(tokens.intersection(signals)) for intent, signals in INTENT_SIGNALS.items()})
    if recent_errors or any(token in tokens for token in {"traceback", "stack", "error"}):
        scores["debug"] += 2
    if any(path.startswith(("tests/", "test_")) or "/test_" in path for path in active_files):
        scores["testing"] += 1
    ranked = sorted(scores, key=lambda intent: (-scores[intent], intent))
    primary = ranked[0] if scores[ranked[0]] else "unknown"
    secondary = tuple(intent for intent in ranked[1:] if scores[intent] > 0 and intent != primary)
    total = sum(scores.values())
    confidence = min(0.95, 0.42 + (scores[primary] / max(1, total)) * 0.53) if primary != "unknown" else 0.2
    return primary, secondary, round(confidence, 6)
