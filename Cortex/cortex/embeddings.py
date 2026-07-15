from __future__ import annotations

import hashlib
import math
import os
import re
import struct
from functools import lru_cache
from typing import Iterable

TOKEN_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_./:-]*|\d+(?:\.\d+)?")


class HashingEmbedder:
    """Dependency-free signed feature hashing over tokens and character n-grams."""

    name = "feature-hash-v1"

    def __init__(self, dimensions: int = 384) -> None:
        self.dimensions = dimensions

    def encode_one(self, text: str) -> list[float]:
        vector = [0.0] * self.dimensions
        normalized = text.lower()
        features: list[str] = TOKEN_RE.findall(normalized)
        compact = re.sub(r"\s+", " ", normalized)
        features.extend(compact[i : i + 4] for i in range(max(0, len(compact) - 3)))
        for feature in features:
            digest = hashlib.blake2b(feature.encode("utf-8"), digest_size=8).digest()
            raw = int.from_bytes(digest, "big")
            index = raw % self.dimensions
            sign = -1.0 if (raw >> 8) & 1 else 1.0
            vector[index] += sign
        norm = math.sqrt(sum(value * value for value in vector)) or 1.0
        return [value / norm for value in vector]


class SentenceTransformerEmbedder:
    def __init__(self, model_name: str) -> None:
        from sentence_transformers import SentenceTransformer  # type: ignore

        self.model_name = model_name
        self.name = f"sentence-transformers:{model_name}"
        self.model = SentenceTransformer(model_name)

    def encode_one(self, text: str) -> list[float]:
        vector = self.model.encode([text], normalize_embeddings=True)[0]
        return [float(value) for value in vector]


@lru_cache(maxsize=1)
def get_embedder() -> HashingEmbedder | SentenceTransformerEmbedder:
    model_name = os.environ.get("CORTEX_EMBEDDING_MODEL", "").strip()
    if model_name:
        try:
            return SentenceTransformerEmbedder(model_name)
        except Exception:
            pass
    return HashingEmbedder()


def cosine(left: Iterable[float], right: Iterable[float]) -> float:
    a = list(left)
    b = list(right)
    if len(a) != len(b) or not a:
        return 0.0
    numerator = sum(x * y for x, y in zip(a, b))
    denominator = math.sqrt(sum(x * x for x in a)) * math.sqrt(sum(y * y for y in b))
    return numerator / denominator if denominator else 0.0


# ── Vector serialization: float32 BLOB for storage efficiency ──────────────
# JSON-encoded vectors are ~5-10x larger and require json.loads on every
# comparison. Raw float32 BLOBs are compact and can be unpacked with struct
# in bulk, giving 10-50x speedup on semantic search over large repositories.

VECTOR_MAGIC = b"CTXV1"


def vector_to_bytes(vector: list[float] | None) -> bytes | None:
    """Serialize a float vector to a compact float32 BLOB."""
    if vector is None:
        return None
    values = [float(value) for value in vector]
    if not all(math.isfinite(value) for value in values):
        raise ValueError("Vectors must contain only finite values")
    return VECTOR_MAGIC + struct.pack("<I", len(values)) + struct.pack(f"<{len(values)}f", *values)


def bytes_to_vector(data: bytes | None) -> list[float]:
    """Deserialize a float32 BLOB back to a list of floats."""
    if not data:
        return []
    if data.startswith(VECTOR_MAGIC):
        if len(data) < len(VECTOR_MAGIC) + 4:
            raise ValueError("Truncated Cortex vector header")
        count = struct.unpack("<I", data[len(VECTOR_MAGIC):len(VECTOR_MAGIC) + 4])[0]
        payload = data[len(VECTOR_MAGIC) + 4:]
        if len(payload) != count * 4:
            raise ValueError("Cortex vector length does not match header")
    else:
        # Compatibility for the short-lived unversioned float32 format.
        payload = data
        if len(payload) % 4:
            raise ValueError("Vector BLOB length must be divisible by four")
        count = len(payload) // 4
    values = list(struct.unpack(f"<{count}f", payload))
    if not all(math.isfinite(value) for value in values):
        raise ValueError("Vector BLOB contains non-finite values")
    return values


def deserialize_vector(raw: bytes | str | None) -> list[float]:
    """Backward-compatible vector deserialization.
    Handles both float32 BLOB (new) and JSON text (legacy)."""
    if raw is None:
        return []
    if isinstance(raw, (bytes, bytearray)):
        # JSON text may be returned as bytes by custom SQLite adapters.
        if raw.lstrip().startswith((b"[", b"{")):
            try:
                import json as _json
                return _json.loads(raw)
            except (ValueError, TypeError):
                return []
        try:
            return bytes_to_vector(raw)
        except (ValueError, struct.error):
            return []
    if isinstance(raw, str):
        try:
            import json as _json
            return _json.loads(raw)
        except (ValueError, TypeError):
            return []
    return []
