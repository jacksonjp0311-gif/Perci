"""B2E-PERCI-004: weight-dependent unit tests silently passed when the weights were missing.

This snapshot has no Bitwork weights. With PERCI_REQUIRE_WEIGHTS=1 the weight tests
must FAIL with a clear "weights required" message instead of passing vacuously.
"""
from _perci_b2e import WORKSPACE, cargo, fail, ok

for pack in (WORKSPACE / "models").glob("*.pwgt"):
    fail(f"snapshot unexpectedly contains weights: {pack}")
proc = cargo(
    ["test", "-q", "--lib", "cognitive::tests::classify_returns_mixture_when_weights_present"],
    env_extra={"PERCI_REQUIRE_WEIGHTS": "1"},
)
out = proc.stdout + proc.stderr
if proc.returncode == 0:
    fail("weight test passed without weights under PERCI_REQUIRE_WEIGHTS=1")
if "weights required" not in out:
    print(out[-3000:])
    fail("weight test failed, but not with the 'weights required' message")
ok("weight tests refuse to pass vacuously")
