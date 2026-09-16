#!/usr/bin/env bash
# Exact-artifact E2E gate (DOD-004).
#
# DOD-004 RULE: "Final smoke and E2E tests run against the exact production
# artifact digest, not merely source or a development server."
#
# The Playwright suite (scripts/test-e2e.sh) runs against `vite preview`, which
# the clause excludes. This gate builds and drives the PACKAGED EXECUTABLE.
#
# Three things were required to make this work, all established empirically over
# three attempts and recorded in .agent/evidence/DOD-004-artifact-e2e-attempt.md:
#
#   1. The artifact must come from `tauri build`, NOT `cargo build --release`.
#      A cargo build embeds `devUrl` and loads http://localhost:5173; only the
#      tauri build pipeline embeds `frontendDist` and serves
#      http://tauri.localhost/. This was the blocker.
#   2. The `devtools-e2e` cargo feature must be enabled -- without it no debug
#      endpoint exists at all.
#   3. The config must carry additionalBrowserArgs with the debug port.
#
# The artifact produced here is a DEDICATED TEST ARTIFACT: the feature is not a
# default, and a shipped release opens no debug port. A debug port in a released
# binary would let any local process attach to the webview and invoke backend
# commands, breaking the SPEC-005 confidentiality boundary. The production
# artifact is separately verified by scripts/smoke-installed-artifact.sh.
set -eu

REPORT_DIR=".agent/evidence/artifact-e2e"
mkdir -p "$REPORT_DIR"

PORT="${LINCHPIN_DEBUG_PORT:-9222}"
E2E_EXE="target/release/linchpin-desktop-e2e.exe"
PROD_EXE="target/release/linchpin-desktop.exe"

[ -f apps/desktop/src-tauri/tauri-e2e.json ] || {
  echo "artifact-e2e: FAIL -- apps/desktop/src-tauri/tauri-e2e.json is missing" >&2
  exit 2
}

echo "artifact-e2e: building the dedicated E2E artifact (tauri build, devtools-e2e)"
pnpm --filter @linchpin/desktop build >/dev/null

# The tauri build pipeline writes to target/release/linchpin-desktop.exe, the
# same path the production artifact uses. The two MUST NOT be confused: the E2E
# build opens a WebView2 debug port, and shipping that would let any local
# process attach to the webview and invoke backend commands, breaking the
# SPEC-005 confidentiality boundary.
#
# The sequence is therefore: move the production artifact aside, build the E2E
# one, copy it to a clearly-named path, then RESTORE the production artifact and
# verify it. An earlier version of this script left the devtools build in place
# as "the production artifact" -- a real regression, caught by checking whether
# the restored binary still kept port 9222 closed.
PROD_EXE="target/release/linchpin-desktop.exe"
PROD_STASH="target/release/linchpin-desktop.production-stash.exe"
HAD_PROD="no"
if [ -f "$PROD_EXE" ]; then
  cp "$PROD_EXE" "$PROD_STASH"
  HAD_PROD="yes"
  echo "artifact-e2e: stashed the production artifact ($(python3 -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest()[:16])" "$PROD_EXE"))"
fi

TAURI_CONFIG="$(cat apps/desktop/src-tauri/tauri-e2e.json)"
export TAURI_CONFIG

# `-- --locked` forwards to cargo. REQ-LIC-003 requires dependency versions to be
# pinned exactly and never floated; without this, `tauri build` could silently
# re-resolve and rewrite Cargo.lock, which every other cargo invocation in this
# repository forbids via --locked.
( cd apps/desktop && npx tauri build --no-bundle -- --locked ) > "$REPORT_DIR/build.log" 2>&1 || {
  echo "artifact-e2e: FAIL -- tauri build failed; see $REPORT_DIR/build.log" >&2
  tail -20 "$REPORT_DIR/build.log" >&2
  [ "$HAD_PROD" = "yes" ] && cp "$PROD_STASH" "$PROD_EXE"
  exit 1
}

[ -f "$PROD_EXE" ] || { echo "artifact-e2e: FAIL -- no exe after build" >&2; exit 1; }
cp "$PROD_EXE" "$E2E_EXE"

# --- restore and verify the production artifact ---------------------------
if [ "$HAD_PROD" = "yes" ]; then
  cp "$PROD_STASH" "$PROD_EXE"
  echo "artifact-e2e: restored the production artifact"
fi

# Prove the restored production binary does NOT open a debug port. Without this
# check the script could silently leave a debug-enabled binary at the production
# path, which is a confidentiality regression rather than a test failure.
#
# The check lives in its own script because the inline version had no baseline and
# no attribution: a leftover debug build still holding 9222 made this gate accuse
# the honest production binary. See scripts/assert-no-debug-port.py.
python3 scripts/assert-no-debug-port.py "$PROD_EXE" "$PORT"

echo "artifact-e2e: driving $E2E_EXE over CDP"
node apps/desktop/e2e-artifact.mjs "$E2E_EXE" "$REPORT_DIR/STATUS.md"
echo "artifact-e2e: ok"
