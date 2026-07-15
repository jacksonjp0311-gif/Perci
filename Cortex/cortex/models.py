from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any


@dataclass
class FileRecord:
    repo: str
    path: str
    kind: str
    language: str
    size_bytes: int
    mtime_ns: int
    content_hash: str
    status: str
    authoritative: bool
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class Hit:
    memory_id: int
    repo: str
    path: str
    start_line: int
    end_line: int
    text: str
    kind: str
    score: float
    content_hash: str
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class Edge:
    source: str
    target: str
    relation: str
    confidence: float
    evidence: str = ""
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)
