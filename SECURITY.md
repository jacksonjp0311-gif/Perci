# Security policy

Perci is a gate that agents rely on to refuse dangerous steps, so a way to
get a dangerous action **allowed** (a policy bypass), a way to forge or
replay a receipt or ledger entry undetected, or any way to make Perci itself
execute code or reach the network unexpectedly is a security issue.

## Reporting

Please report privately through GitHub's
[private vulnerability reporting](https://github.com/jacksonjp0311-gif/Perci/security/advisories/new)
("Report a vulnerability" on the Security tab). Include the Perci version,
the tool and the exact arguments (or the command line), what Perci answered
and what it should have answered. Do not include real secrets.

You should get an acknowledgement within a few days. Fixes ship as a new
release with a CHANGELOG entry; reporters are credited unless they prefer
not to be.

## Supported versions

Perci is in preview (0.x). Only the newest release gets security fixes.

## Scope and limits

- The policy is structural and conservative, not a sandbox. It classifies
  commands and paths before they run; it does not contain them. Run untrusted
  agents in a sandbox as well.
- Some checks are point in time (for example, "this delete target does not
  exist"); a race between the check and the action is out of scope.
- Release binaries are not code-signed yet. Verify downloads against the
  release's `SHA256SUMS` (see docs/QUICKSTART.md).
- Perci makes no network requests of its own except `perci fetch-weights`
  (GitHub release assets, sha256-checked against the manifest).
