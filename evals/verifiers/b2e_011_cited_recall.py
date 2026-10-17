"""B2E-PERCI-011: recall must return citations whose sha256 matches the note file.

remember -> recall --json -> check every citation against the files on disk:
  * sha256 equals the SHA-256 of the cited note file,
  * the journal (append-only source of truth) holds the record at journal_offset
    and its line hashes to record_sha256,
  * an edited note is reported by `perci memory verify` (exit 1).

AgentSearch is used when PERCI_AGENTSEARCH is set (e.g. "python -m agentsearch");
PERCI_B2E_REQUIRE_AGENTSEARCH=1 makes the substring fallback a failure.
"""
import hashlib
import json
import os
import tempfile
from pathlib import Path

from _perci_b2e import build, fail, log, ok, run_perci

extra = {}
if os.environ.get("PERCI_AGENTSEARCH"):
    extra["PERCI_AGENTSEARCH"] = os.environ["PERCI_AGENTSEARCH"]
require_as = os.environ.get("PERCI_B2E_REQUIRE_AGENTSEARCH") == "1"
data = Path(tempfile.mkdtemp(prefix="perci-b2e-011-"))
exe = build()


def perci(*args, stdin=None):
    return run_perci(exe, ["memory", *args], data_dir=data, env_extra=extra, stdin=stdin)


for note in ("dentist moved to Tuesday 3pm", "the backup drive is in the left drawer",
             "backup schedule runs nightly at 2am"):
    if perci("remember", note).returncode != 0:
        fail("perci memory remember failed")
proc = perci("recall", "where is the backup drive", "--json")
if proc.returncode != 0:
    fail(f"perci memory recall failed: {proc.stderr[-300:]}")
try:
    result = json.loads(proc.stdout)
except ValueError:
    fail(f"recall --json did not print JSON: {proc.stdout[:300]!r}")
log(f"backend={result.get('backend')} hits={len(result.get('hits', []))}")
if require_as and result.get("backend") != "agentsearch":
    fail(f"AgentSearch required but backend is {result.get('backend')}: {result.get('warnings')}")
hits = result.get("hits") or []
if not hits or hits[0]["text"] != "the backup drive is in the left drawer":
    fail(f"best hit is wrong: {hits[:1]}")
journal = None
for hit in hits:
    c = hit["citation"]
    note = Path(c["note_path"])
    if not note.is_file():
        fail(f"cited note missing: {note}")
    if hashlib.sha256(note.read_bytes()).hexdigest() != c["sha256"]:
        fail(f"citation sha256 does not match {note}")
    journal = Path(c["journal_path"]).read_bytes()
    line = journal[c["journal_offset"]:].split(b"\n", 1)[0]
    if hashlib.sha256(line).hexdigest() != c["record_sha256"]:
        fail("journal_offset/record_sha256 do not point at the cited record")
    if hit["text"].encode() not in line and json.dumps(hit["text"])[1:-1].encode() not in line:
        fail("cited journal record does not contain the recalled text")
if perci("verify", stdin=json.dumps(hits[0])).returncode != 0:
    fail("verify rejected an untouched citation")
Path(hits[0]["citation"]["note_path"]).write_text("tampered\n", encoding="utf-8")
if perci("verify", stdin=json.dumps(hits[0])).returncode != 1:
    fail("verify did not detect an edited note")
ok("recall citations match the note files and the journal; edits are detected")
