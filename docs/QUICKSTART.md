# Perci quickstart

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
| pin a version | `--version v0.11.7` | `$env:PERCI_VERSION="v0.11.7"` |
| install folder | `--dir DIR` | `$env:PERCI_INSTALL_DIR="DIR"` |
| leave shell startup files / user PATH alone | `--no-modify-path` | `$env:PERCI_NO_MODIFY_PATH="1"` |

Prebuilt targets: Linux x64 (static musl), macOS arm64 and x64, Windows x64.
Elsewhere, build from a clone: `git clone https://github.com/jacksonjp0311-gif/Perci`,
`git lfs pull`, `cargo build --release` (Rust 1.75+), and use `target/release/perci`. To install by hand, download
`perci-<target>.tar.gz|zip` and `SHA256SUMS` from the
[release page](https://github.com/jacksonjp0311-gif/Perci/releases), verify
the archive, unpack it anywhere, and run `perci fetch-weights`. Perci finds its
home two levels above the executable, so no `PERCI_HOME` is needed.

## 2. Connect your agent (MCP over stdio)

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
`perci tool NAME --json '<args>'` (or the arguments on stdin). It prints the
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
| `next_step` | likely next steps from verified past runs, with counts; warnings for a planned call | 10–600 µs |
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
