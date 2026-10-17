#!/bin/sh
# Perci installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/jacksonjp0311-gif/Perci/main/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --weights tools      # tool layer + route pack only (~20 MB)
#
# Downloads the release archive for this OS/CPU, verifies it against the
# release's SHA256SUMS, unpacks it into ~/.perci (a complete Perci home), puts
# ~/.perci/bin on PATH, downloads + verifies the weights with
# `perci fetch-weights`, and runs `perci doctor`.
#
# Options (or environment variables):
#   --version vX.Y.Z   PERCI_VERSION       release tag (default: latest)
#   --dir DIR          PERCI_INSTALL_DIR   install folder (default: ~/.perci)
#   --weights SET      PERCI_INSTALL_WEIGHTS       default (~240 MB) | tools (~20 MB) | all | none
#   --no-modify-path   PERCI_NO_MODIFY_PATH=1   do not edit shell startup files
#   PERCI_RELEASE_URL  download base for the release assets (mirrors, testing)
#
# Re-running upgrades in place; already-downloaded weights that still match
# the manifest are kept.
set -eu

REPO="jacksonjp0311-gif/Perci"
VERSION="${PERCI_VERSION:-latest}"
DIR="${PERCI_INSTALL_DIR:-$HOME/.perci}"
WEIGHTS="${PERCI_INSTALL_WEIGHTS:-default}"
NO_PATH="${PERCI_NO_MODIFY_PATH:-0}"

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --dir) DIR="$2"; shift 2 ;;
    --weights) WEIGHTS="$2"; shift 2 ;;
    --tools-only) WEIGHTS="tools"; shift ;;
    --no-weights) WEIGHTS="none"; shift ;;
    --no-modify-path) NO_PATH=1; shift ;;
    -h|--help) sed -n '2,24p' "$0" 2>/dev/null || true; exit 0 ;;
    *) echo "perci install: unknown option $1" >&2; exit 2 ;;
  esac
done

say() { printf '%s\n' "perci install: $*"; }
die() { printf '%s\n' "perci install: error: $*" >&2; exit 1; }

case "$WEIGHTS" in default|tools|all|none) ;; *) die "--weights must be default, tools, all or none" ;; esac

os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64|Linux/amd64) target="x86_64-unknown-linux-musl" ;;
  Darwin/arm64|Darwin/aarch64) target="aarch64-apple-darwin" ;;
  Darwin/x86_64) target="x86_64-apple-darwin" ;;
  *) die "no prebuilt binary for $os/$arch yet; build from a clone of https://github.com/$REPO (git lfs pull && cargo build --release)" ;;
esac

if [ -n "${PERCI_RELEASE_URL:-}" ]; then
  base="${PERCI_RELEASE_URL%/}"
elif [ "$VERSION" = "latest" ]; then
  base="https://github.com/$REPO/releases/latest/download"
else
  case "$VERSION" in v*) ;; *) VERSION="v$VERSION" ;; esac
  base="https://github.com/$REPO/releases/download/$VERSION"
fi

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q -O "$2" "$1"; }
else
  die "need curl or wget"
fi
if command -v sha256sum >/dev/null 2>&1; then
  sha() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
  sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
  die "need sha256sum or shasum to verify the download"
fi

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t perci)"
trap 'rm -rf "$tmp"' EXIT INT TERM

asset="perci-$target.tar.gz"
say "downloading $asset from $base"
fetch "$base/$asset" "$tmp/$asset" || die "download failed: $base/$asset"
fetch "$base/SHA256SUMS" "$tmp/SHA256SUMS" || die "download failed: $base/SHA256SUMS"
want="$(awk -v f="$asset" '$2 == f || $2 == "*"f {print $1}' "$tmp/SHA256SUMS")"
[ -n "$want" ] || die "$asset is not listed in SHA256SUMS"
got="$(sha "$tmp/$asset")"
[ "$want" = "$got" ] || die "checksum mismatch for $asset (expected $want, got $got)"
say "sha256 ok ($got)"

mkdir -p "$tmp/x" "$DIR"
tar -xzf "$tmp/$asset" -C "$tmp/x"
[ -x "$tmp/x/perci/bin/perci" ] || die "archive has no perci/bin/perci"
# Copy over the previous install; weights already in models/ are kept.
(cd "$tmp/x/perci" && tar -cf - .) | (cd "$DIR" && tar -xf -)
PERCI="$DIR/bin/perci"
say "installed $("$PERCI" --version) to $DIR"

if [ "$NO_PATH" != "1" ]; then
  line="export PATH=\"$DIR/bin:\$PATH\"  # perci"
  for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
    case "$rc" in
      */.profile) ;;
      */.zshrc) [ -f "$rc" ] || [ "$os" = "Darwin" ] || continue ;;
      *) [ -f "$rc" ] || continue ;;
    esac
    if ! grep -qs "$DIR/bin" "$rc"; then
      printf '\n%s\n' "$line" >> "$rc"
      say "added $DIR/bin to PATH in $rc"
    fi
  done
fi

if [ "$WEIGHTS" != "none" ]; then
  if [ -n "${PERCI_RELEASE_URL:-}" ] && [ -z "${PERCI_WEIGHTS_URL:-}" ]; then
    PERCI_WEIGHTS_URL="$base"; export PERCI_WEIGHTS_URL
  fi
  say "fetching weights (set: $WEIGHTS)"
  PERCI_HOME="$DIR" "$PERCI" fetch-weights --set "$WEIGHTS" || die "weights download failed; re-run: perci fetch-weights --set $WEIGHTS"
fi

doctor_args=""
[ "$WEIGHTS" = "default" ] || [ "$WEIGHTS" = "all" ] || doctor_args="--tools"
# shellcheck disable=SC2086
PERCI_HOME="$DIR" "$PERCI" doctor $doctor_args || die "perci doctor reported a problem (see above)"

cat <<MSG

Perci is installed. Open a new shell (or run: export PATH="$DIR/bin:\$PATH"), then:
  perci tool exact --json '{"expr":"17% of 240","claimed":"41"}'
  claude mcp add perci -- perci mcp          # Claude Code; other harnesses: docs/QUICKSTART.md
Quickstart: $DIR/docs/QUICKSTART.md
MSG
