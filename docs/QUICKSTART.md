# Perci quickstart

> **Preview (0.11.x).** The tool layer is tested on Linux, macOS and Windows
> CI runners and in the host CLIs listed under [Host test status](#host-test-status).
> Release binaries are **not code-signed** yet; see
> [Verifying downloads](#verifying-downloads-unsigned-preview-builds).

Perci is a fast, deterministic verifier and gate for AI agents. Your harness
calls it before it trusts an answer or runs a step. It recomputes the math,
checks claimed consequences and citations, gates shell commands and writes,
and keeps a hash-chained ledger of what the run did. Every call returns a
verdict (`pass` / `fail` / `abstain` / `error`) with reasons and a replayable
receipt, usually in well under a millisecond. It runs locally, needs no
API key and sends nothing anywhere.

## 1. Install

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.sh | sh
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.ps1 | iex
```

The installer downloads the release for your OS/CPU, checks it against the
release's `SHA256SUMS`, unpacks it to `~/.perci` (Windows:
`%LOCALAPPDATA%\Perci\app`), adds `bin` to your PATH, downloads and
sha256-checks the weights (`perci fetch-weights`), and runs `perci doctor`.
Open a new terminal afterwards.

| option | sh | PowerShell |
|---|---|---|
| tool layer + route pack only (~20 MB instead of ~240 MB) | `sh -s -- --weights tools` | `$env:PERCI_INSTALL_WEIGHTS="tools"` before `irm` |
| no weights (exact, check_math, policy, ledger still work) | `--weights none` | `$env:PERCI_INSTALL_WEIGHTS="none"` |
| pin a version | `--version v0.11.8` | `$env:PERCI_VERSION="v0.11.8"` |
| install folder | `--dir DIR` | `$env:PERCI_INSTALL_DIR="DIR"` |
| leave shell startup files / user PATH alone | `--no-modify-path` | `$env:PERCI_NO_MODIFY_PATH="1"` |

With no version pinned the installers take the newest release, previews
included (0.x releases are published as GitHub prereleases).

Prebuilt targets: Linux x64 (static musl), macOS arm64 and x64, Windows x64.
Elsewhere, build from a clone: `git clone https://github.com/jacksonjp0311-gif/Perci`,
`git lfs pull`, `cargo build --release` (Rust 1.75+), and use `target/release/perci`. To install by hand, download
`perci-<target>.tar.gz|zip` and `SHA256SUMS` from the
[release page](https://github.com/jacksonjp0311-gif/Perci/releases), verify
the archive, unpack it anywhere, and run `perci fetch-weights`. Perci finds its
home two levels above the executable, so no `PERCI_HOME` is needed.

### Verifying downloads (unsigned preview builds)

Every release publishes `SHA256SUMS`, the sha256 of every asset, and both
installers refuse an archive that does not match it. To check a manual
download:

```sh
# macOS / Linux, in the folder with the archive and SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS        # macOS: shasum -a 256 --ignore-missing -c SHA256SUMS
```

```powershell
# Windows: compare with the line for the zip in SHA256SUMS
(Get-FileHash .\perci-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLower()
```

The binaries are not code-signed or notarized yet (no signing certificate),
so the operating system may warn when you run a copy downloaded with a
browser. Files fetched by `install.sh` (curl) and `install.ps1`
(`Invoke-WebRequest`) normally carry no quarantine mark, so the installers
usually see no prompt.

* **macOS Gatekeeper**: "“perci” cannot be opened because the developer
  cannot be verified" (or "Apple could not verify…"). After checking the
  sha256, clear the quarantine flag: `xattr -d com.apple.quarantine
  ~/.perci/bin/perci` (or `xattr -dr com.apple.quarantine ~/.perci`), or
  allow it under System Settings → Privacy & Security → "Open Anyway".
* **Windows SmartScreen**: "Windows protected your PC … unrecognized app".
  After checking the sha256, choose **More info → Run anyway**, or unblock
  the zip before unpacking: `Unblock-File .\perci-x86_64-pc-windows-msvc.zip`.

Building from source (below) avoids both warnings.

## 2. Connect your agent (MCP over stdio)

### Host test status

| host | config | status (0.11.8, Linux box) |
|---|---|---|
| Claude Code CLI 2.1.197 | `claude mcp add --scope user perci -- perci mcp` | **tested**: `claude mcp list` launched the server and reports `✔ Connected` (no login needed to list) |
| Claude Code, project `.mcp.json` | JSON below | **tested** (parsed): listed as project scope, `⏸ Pending approval` until you run `claude` once and approve it (Claude Code asks for every new project server) |
| Codex CLI 0.163.0 | `~/.codex/config.toml` below, or `codex mcp add perci -- perci mcp` | **config tested**: `codex mcp list` / `get` parse it (`enabled`, stdio). `codex mcp list` does not start servers, so the launch itself was checked with the MCP inspector |
| Cursor CLI (`cursor-agent` 2026.10.16) | `~/.cursor/mcp.json` or `.cursor/mcp.json` below | **tested**: `cursor-agent mcp list` → `perci: ready`; `cursor-agent mcp list-tools perci` lists all 14 tools. A project file needs `cursor-agent mcp enable perci` (or approval in the editor) first. The Cursor editor reads the same files but was not run |
| Claude Desktop | `claude_desktop_config.json` below | **config validated, not host-tested** (GUI app): same `mcpServers` shape as Claude Code, checked against the documented format |
| MCP Inspector CLI 2.11.0 | `npx @modelcontextprotocol/inspector --cli perci mcp --method tools/list` | **tested**: `tools/list` (14 tools) and `tools/call` for `exact`, `check_math`, `policy_check`, `agent_gate_step`, `status`, `next_step`; replies carry `structuredContent` verdicts |

Model-driven tool calls inside Claude Code, Codex and Cursor need an account
and were not run; the hosts above started Perci and read its tool list.


Every host starts `perci mcp` as a child process. If the host cannot see your
shell PATH (desktop apps often can't), use the absolute path:
`~/.perci/bin/perci` on macOS/Linux, or
`%LOCALAPPDATA%\Perci\app\bin\perci.exe` on Windows.

**Claude Code**

```sh
claude mcp add --scope user perci -- perci mcp
```

or commit a project `.mcp.json`:

```json
{ "mcpServers": { "perci": { "command": "perci", "args": ["mcp"] } } }
```

**Cursor**: `~/.cursor/mcp.json` (all projects) or `.cursor/mcp.json` (one project):

```json
{ "mcpServers": { "perci": { "command": "perci", "args": ["mcp"] } } }
```

**Codex**: `~/.codex/config.toml`:

```toml
[mcp_servers.perci]
command = "perci"
args = ["mcp"]
```

**Claude Desktop**: `claude_desktop_config.json` (macOS
`~/Library/Application Support/Claude/`, Windows `%APPDATA%\Claude\`), then
restart the app:

```json
{ "mcpServers": { "perci": {
    "command": "/Users/YOU/.perci/bin/perci",
    "args": ["mcp"] } } }
```

On Windows use `"command": "C:\\Users\\YOU\\AppData\\Local\\Perci\\app\\bin\\perci.exe"`.

**Any other harness.** MCP stdio is newline-delimited JSON-RPC 2.0:

```text
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"my-harness","version":"1"}}}
→ {"jsonrpc":"2.0","method":"notifications/initialized"}
→ {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"exact","arguments":{"expr":"17% of 240","claimed":"41"}}}
← {"jsonrpc":"2.0","id":2,"result":{"structuredContent":{"status":"fail","reasons":[{"code":"EXACT_MISMATCH",...}],...},...}}
```

Without MCP, call the CLI once per check:
`perci tool NAME --json '<args>'` (or the arguments piped on stdin; with no
arguments it uses `{}` and never waits on stdin). It prints the
verdict JSON and exits `0` pass, `1` fail, `4` abstain, `3` error, `2` usage.
`perci tool schema NAME` prints a tool's input schema.

Useful environment variables: `PERCI_RUN_ID` (name the run ledger),
`PERCI_TOOLS=all` (also enable the off-by-default tools), `PERCI_PROFILE` (your
own policy, sequence rules and packs, kept outside the install; see
[PROFILES.md](PROFILES.md)), `PERCI_LEDGER=off`.

A line for the system prompt (the server's `instructions` say the same):
*"Call `verify` on any answer with numbers or citations before presenting it,
and `agent_gate_step` (phase `pre`) before shell commands or writes. On `fail`
fix what the reasons say; on `abstain` say it is unverified."*

## 3. The 60-second demo

Paste into a terminal (macOS/Linux; in PowerShell set `$env:PERCI_RUN_ID` and
pipe the same JSON strings):

```sh
echo '{"expr":"17% of 240","claimed":"41"}' | perci tool exact
#  FAIL  EXACT_MISMATCH: claimed 41, exact 204/5 (40.8)

echo '{"text":"The subtotal is 120 + 45 = 165, and 3 x 42 = 128."}' | perci tool check_math
#  FAIL  MATH_MISMATCH: 3 x 42: claimed 128, exact 126

echo '{"action":{"kind":"shell","command":"git push --force origin main"}}' | perci tool policy_check
#  FAIL  POLICY_REFUSE: git.force_push: rewrites or deletes remote history

export PERCI_RUN_ID=demo-$$             # one run ledger for the next four calls
echo '{"phase":"pre","action":{"kind":"shell","command":"git push origin main"}}' | perci tool agent_gate_step
#  FAIL  SEQUENCE_REQUIRES: seq.deploy_requires_tests: deploying requires tests that passed
#        after the last code change in this run          (ledger:demo-…#1)
echo '{"phase":"post","action":{"kind":"shell","command":"cargo test"},"exit_code":0}' | perci tool agent_gate_step
#  PASS                                                   (ledger:demo-…#2)
echo '{"phase":"pre","action":{"kind":"shell","command":"git push origin main"}}' | perci tool agent_gate_step
#  ABSTAIN  NEEDS_APPROVAL: git.history: publishes (the sequence rule is now satisfied, citing #2)
echo '{}' | perci tool ledger_verify
#  PASS  3 entries, chain intact
```

A full agent loop (a scripted model is corrected by Perci's reasons, a push is
held until tests pass, `next_step` suggests the next move) is in
`evals/demo/` in the repository: `python3 evals/demo/harness.py`.

### The Temporal Router

Every gated step and verdict lands in the run's hash-chained ledger.
`perci ledger learn` turns the ledgers whose chains verify into integer
n-gram counts, and `next_step` (the Temporal Router) answers "what usually
comes next here?" with the counts behind each suggestion, plus warnings when a
planned step would break a sequence rule:

```sh
perci ledger learn                                   # after a few runs
echo '{"planned":{"action":{"kind":"shell","command":"git push origin main"}}}' | perci tool next_step
#  FAIL  SEQUENCE_REQUIRES: planned shell:git push: … no tests step with status pass …
#        next: [{step, count, of, permille, outcomes}…]
```

With no history for the context it abstains (`NO_HISTORY`) instead of
guessing. It is advisory: `agent_gate_step` is what enforces.

## 4. What each tool does

| tool | decides | typical work |
|---|---|---|
| `exact` | exact rational arithmetic, %, powers, roots, primality, linear equations; checks a `claimed` value | ~5–10 µs |
| `check_math` | finds every `expr = value` claim in free text and recomputes it | ~20 µs |
| `relation` | what follows from stated premises under a change; checks claimed impacts | ~110 µs |
| `verify` | one call: math + evidence hashes + relation + policy for an answer | ~0.2 ms |
| `agent_check_claims` | splits an answer into claims and says which Perci can check | ~30 µs |
| `policy_check` | allow / refuse / needs-approval for shell, write, delete, network, commit, push | ~10 µs |
| `agent_gate_step` | `pre`: policy + sequence rules against the run ledger; `post`: exit code and output math | 15 µs–0.5 ms |
| `ledger_append` / `ledger_show` / `ledger_verify` | record a step; show the run; re-hash the chain (tamper check) | 1 µs / ~5 µs per entry |
| `next_step` (**Temporal Router**, alias `temporal_route`) | likely next steps learned from verified, hash-chained run ledgers, with counts; warnings for a planned call; abstains on a context it has not seen | 10–600 µs |
| `route` | deterministic route (refuse / exact / relation / recall / llm) with the weight pack's label and calibrated confidence; abstains when unsure | ~0.4 ms (needs weights) |
| `recall` | cited lookup in your Perci notes (AgentSearch when installed) | ~1.5 ms |
| `status` | health, enabled tools, policy hash | ~0.3 ms |
| `remember`, `agent_verify_patch`, `eval` | write a note; run a patch's tests in a scratch worktree; bug2eval cases. Off by default (`PERCI_TOOLS=all`) | |

Verdicts: `pass` (checked and right), `fail` (checked and wrong, or refused),
`abstain` (could not check, or needs a human; never a silent pass), `error`
(bad input). Full reference: [TOOLS.md](TOOLS.md). Benchmarks:
[BENCHMARKS.md](BENCHMARKS.md).

## 5. Check and update

```sh
perci doctor            # home, weights, optional integrations (exit 2 on a problem)
perci doctor --tools    # the tool layer only (weights optional)
perci fetch-weights     # re-download any weight file that is missing or does not match its sha256
```

Re-run the installer to upgrade. Weights that still match the manifest are kept.
