#!/usr/bin/env bash
# Startup wrapper for the personal-site server.
# Every start fetches the latest release binary from GitHub raw and execs it,
# so a systemd restart automatically picks up newly pushed binaries.
set -euo pipefail

BIN=personal-site
# Cache-busting query forces a fresh fetch of the committed binary on master.
URL="https://raw.githubusercontent.com/amirhosseineslami/khosrowan-website-rust/master/personal-site?t=$(date +%s)"

cd "$(dirname "$0")/.."
mkdir -p data

echo "[startup] Fetching latest binary..."
curl -fsSL "$URL" -o "$BIN.tmp"

# Sanity: the artifact must be a 64-bit ELF executable.
file "$BIN.tmp" | grep -q "ELF 64-bit" || {
  echo "[startup] Downloaded artifact is not a valid binary — aborting."
  rm -f "$BIN.tmp"
  exit 1
}

chmod +x "$BIN.tmp"
mv -f "$BIN.tmp" "$BIN"

echo "[startup] Launching $BIN"
exec ./"$BIN"
