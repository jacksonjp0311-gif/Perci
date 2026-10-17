"""Shared helpers for Perci's bug2eval verifiers.

Every verifier builds the Perci binary from the snapshot it is run in, runs it
in a clean environment (no inherited PERCI_* variables, a fresh PERCI_DATA, no
API keys) and exits 0 only when the bug is absent.

Environment knobs (all optional):
  PERCI_B2E_TARGET   shared cargo target dir (default: <tmp>/perci-b2e-target),
                     so before/after snapshots reuse compiled dependencies.
  CARGO              cargo executable (default: cargo).

Verifiers never touch the user's real Perci data or the network.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parents[2]
EXE = "perci.exe" if os.name == "nt" else "perci"


def log(message: str) -> None:
    print(f"[b2e] {message}", flush=True)


def fail(message: str) -> "NoReturn":  # type: ignore[name-defined]
    log(f"FAIL: {message}")
    sys.exit(1)


def ok(message: str) -> None:
    log(f"PASS: {message}")
    sys.exit(0)


def target_dir() -> Path:
    value = os.environ.get("PERCI_B2E_TARGET")
    return Path(value) if value else Path(tempfile.gettempdir()) / "perci-b2e-target"


def clean_env(extra: dict | None = None, data_dir: Path | None = None) -> dict:
    env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("PERCI_") and key not in {"OPENAI_API_KEY", "CARGO_TARGET_DIR"}
    }
    env["PERCI_HOME"] = str(WORKSPACE)
    env["PERCI_DATA"] = str(data_dir or Path(tempfile.mkdtemp(prefix="perci-b2e-data-")))
    env["NO_COLOR"] = "1"
    if extra:
        env.update(extra)
    return env


def cargo(args: list[str], env_extra: dict | None = None, timeout: int = 900, check: bool = False):
    env = clean_env(env_extra)
    env["CARGO_TARGET_DIR"] = str(target_dir())
    cmd = [os.environ.get("CARGO", "cargo"), *args]
    log("$ " + " ".join(cmd))
    proc = subprocess.run(
        cmd, cwd=WORKSPACE, env=env, capture_output=True, text=True,
        encoding="utf-8", errors="replace", timeout=timeout,
    )
    if check and proc.returncode != 0:
        sys.stdout.write(proc.stdout[-4000:])
        sys.stdout.write(proc.stderr[-4000:])
        fail(f"cargo {' '.join(args)} exited {proc.returncode}")
    return proc


def build() -> Path:
    """Build the perci binary for this snapshot and return its path."""
    cargo(["build", "-q", "--bin", "perci"], check=True)
    exe = target_dir() / "debug" / EXE
    if not exe.is_file():
        fail(f"built binary not found at {exe}")
    # Copy so a later build of the other snapshot cannot replace it mid-run.
    private = Path(tempfile.mkdtemp(prefix="perci-b2e-bin-")) / EXE
    shutil.copy2(exe, private)
    return private


def run_perci(exe: Path, args: list[str], cwd: Path | None = None, env_extra: dict | None = None,
              data_dir: Path | None = None, timeout: int = 120, stdin: str | None = None):
    env = clean_env(env_extra, data_dir)
    proc = subprocess.run(
        [str(exe), *args], cwd=cwd or WORKSPACE, env=env, input=stdin,
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout,
    )
    log(f"perci {' '.join(args)!r} -> exit {proc.returncode}")
    return proc


def ask(exe: Path, prompt: str, **kwargs) -> str:
    proc = run_perci(exe, ["ask", "--no-weights", prompt], **kwargs)
    out = proc.stdout + proc.stderr
    log(f"  output: {out.strip()[:300]!r}")
    return out


def probe(test_source: str, append_to: str | None = None, name: str = "b2e_probe",
          timeout: int = 900, env_extra: dict | None = None):
    """Compile and run a white-box test against a *scratch copy* of this snapshot.

    The workspace itself is never modified. With ``append_to`` (for example
    ``"src/agent.rs"``) the test is appended to that module so it can reach
    private items; otherwise it becomes ``tests/<name>.rs`` and uses the public
    ``perci::`` API. Returns the completed cargo process.
    """
    scratch = Path(tempfile.mkdtemp(prefix="perci-b2e-probe-"))
    for item in ("Cargo.toml", "Cargo.lock", "build.rs"):
        if (WORKSPACE / item).is_file():
            shutil.copy2(WORKSPACE / item, scratch / item)
    for folder in ("src", "config"):
        if (WORKSPACE / folder).is_dir():
            shutil.copytree(WORKSPACE / folder, scratch / folder)
    manifest = WORKSPACE / "models" / "PACKAGE_MANIFEST.json"
    if manifest.is_file():
        (scratch / "models").mkdir()
        shutil.copy2(manifest, scratch / "models" / manifest.name)
    if append_to:
        with open(scratch / append_to, "a", encoding="utf-8") as handle:
            handle.write(f"\n#[cfg(test)]\nmod {name} {{\n{test_source}\n}}\n")
        args = ["test", "-q", "--lib", name]
    else:
        (scratch / "tests").mkdir(exist_ok=True)
        (scratch / "tests" / f"{name}.rs").write_text(test_source, encoding="utf-8")
        args = ["test", "-q", "--test", name]
    env = clean_env(env_extra)
    env["PERCI_HOME"] = str(scratch)
    env["CARGO_TARGET_DIR"] = str(target_dir())
    cmd = [os.environ.get("CARGO", "cargo"), *args]
    log("$ " + " ".join(cmd) + f"   (scratch copy {scratch})")
    proc = subprocess.run(
        cmd, cwd=scratch, env=env, capture_output=True, text=True,
        encoding="utf-8", errors="replace", timeout=timeout,
    )
    # Test results and panic messages are on stdout; keep only cargo errors from stderr.
    shown = proc.stdout.strip().splitlines()[-30:]
    shown += [ln for ln in proc.stderr.splitlines() if ln.startswith("error")][:20]
    for line in shown:
        log("  " + line)
    shutil.rmtree(scratch, ignore_errors=True)
    return proc
