"""B2E-PERCI-006: Perci created memory/ and models/ in whatever folder it was run from.

Run from an empty temporary folder with PERCI_HOME set: nothing may be created there.
"""
import tempfile
from pathlib import Path

from _perci_b2e import build, fail, ok, run_perci

exe = build()
cwd = Path(tempfile.mkdtemp(prefix="perci-b2e-cwd-"))
for prompt in ("remember that the build server is atlas", "calculate 12 divided by 4", "hello"):
    run_perci(exe, ["ask", "--no-weights", prompt], cwd=cwd, env_extra={"PERCI_NO_WEIGHTS": "1"})
created = sorted(p.name for p in cwd.iterdir())
if created:
    fail(f"perci wrote into the current folder: {created}")
ok("running from another folder leaves it untouched")
