#!/usr/bin/env python3
"""Tiny protocol example for PERCI_MODEL_CMD; not an intelligence model."""
import sys
payload = sys.stdin.read()
user = payload.split("USER:\n", 1)[-1].strip()
print(f"Perci model adapter received: {user}")
