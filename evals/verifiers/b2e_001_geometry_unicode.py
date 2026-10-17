"""B2E-PERCI-001: exact geometry printed mojibake (Ï€, âˆš, Â²) instead of π, √, ²."""
from _perci_b2e import ask, build, fail, ok

MOJIBAKE = ("Ï€", "âˆš", "Â²", "Ã", "Â")
exe = build()
circle = ask(exe, "what is the area of a circle with radius 3")
if any(bad in circle for bad in MOJIBAKE):
    fail("circle answer contains mojibake")
if "9π" not in circle and "9 π" not in circle:
    fail("circle answer does not state the exact value 9π")
ascii_out = ask(exe, "what is the area of a circle with radius 3", env_extra={"PERCI_ASCII": "1"})
if any(bad in ascii_out for bad in MOJIBAKE):
    fail("PERCI_ASCII answer contains mojibake")
ok("exact geometry prints clean π")
