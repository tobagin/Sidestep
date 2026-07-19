#!/usr/bin/env bash
# Build and sign the device-DB bundle that the app auto-syncs.
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Produces `devices.tar.gz` (a tarball of data/devices) and a detached minisign
# signature `devices.tar.gz.minisig`. Upload BOTH as assets on the GitHub release
# whose tag matches the app version (e.g. v0.3.0). The app fetches them from
# https://github.com/tobagin/Sidestep/releases/download/v<version>/ and refuses
# to apply the bundle unless the signature verifies against the public key
# embedded in src/models/sync.rs (DEVICE_DB_PUBLIC_KEY).
#
# One-time key setup (keep the .key file OFFLINE and secret — it is the whole
# point of signing; never commit it):
#     minisign -G -p minisign.pub -s minisign.key
# Then paste the SECOND line of minisign.pub into DEVICE_DB_PUBLIC_KEY.
#
# Usage:
#     packaging/sign-devices.sh path/to/minisign.key
set -euo pipefail

KEY="${1:?usage: sign-devices.sh <minisign.key>}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO_ROOT/devices.tar.gz"

command -v minisign >/dev/null || { echo "minisign not found (install it first)"; exit 1; }

# Deterministic tarball: sorted entries, fixed mtime/owner so re-runs are stable.
# Entries are rooted at `data/devices/...`, which is what sync.rs expects.
tar --sort=name --mtime='UTC 2020-01-01' --owner=0 --group=0 --numeric-owner \
    -C "$REPO_ROOT" -czf "$OUT" data/devices

minisign -S -s "$KEY" -m "$OUT"

echo "Wrote:"
echo "  $OUT"
echo "  $OUT.minisig"
echo "Upload both to the GitHub release matching this version's tag."
