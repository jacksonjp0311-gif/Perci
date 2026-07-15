#!/usr/bin/env python3
"""Tiny structured protocol example for PERCI_MODEL_CMD."""

from __future__ import annotations

import json
import sys

payload = json.load(sys.stdin)
if payload.get("protocol") != "perci-backend/1.0":
    raise SystemExit("unsupported Perci backend protocol")

user = str(payload.get("user", "")).strip()
memory_count = len(payload.get("memory", []))
print(f"Perci model adapter received: {user} (bounded context items: {memory_count})")