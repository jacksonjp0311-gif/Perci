"""B2E-PERCI-005: un-pulled Git LFS weights gave a cryptic 'unknown magic signature' error.

The snapshot's models/*.pwgt are LFS pointer files. `perci ask hi` must exit 2 and
tell the user to run `git lfs pull`.
"""
from _perci_b2e import WORKSPACE, build, fail, ok, run_perci

pointer = WORKSPACE / "models" / "perci-cognitive-v0.1.pwgt"
if not pointer.is_file() or not pointer.read_bytes().startswith(b"version https://git-lfs"):
    fail("snapshot must contain the LFS pointer models/perci-cognitive-v0.1.pwgt")
exe = build()
proc = run_perci(exe, ["ask", "hi"], cwd=WORKSPACE)
out = proc.stdout + proc.stderr
print(out[-1000:])
if proc.returncode != 2:
    fail(f"expected exit code 2 for LFS pointer weights, got {proc.returncode}")
if "git lfs pull" not in out:
    fail("error does not mention 'git lfs pull'")
ok("LFS pointer weights fail clearly with exit 2")
