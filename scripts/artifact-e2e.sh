#!/usr/bin/env bash
# NOT FUNCTIONAL -- see .agent/evidence/DOD-004-artifact-e2e-attempt.md
#
# This gate does not work yet and must NOT be added to harness-validate.sh or
# test-e2e.sh. It is retained as a recorded attempt carrying its verified
# findings, so the next attempt does not rediscover them.
#
# Exact-artifact E2E gate (DOD-004).
#
# DOD-004 RULE: "Final smoke and E2E tests run against the exact production
# artifact digest, not merely source or a development server."
#
# The existing Playwright suite runs against `vite preview`, which is a
# DEVELOPMENT SERVER -- explicitly excluded by the clause. This gate drives the
# packaged executable's real WebView2 instance instead.
#
# Method:
#   1. pin the executable digest
#   2. launch it with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS set so WebView2
#      exposes a DevTools endpoint on loopback
#   3. wait for the endpoint, read the page target
#   4. attach over the DevTools protocol and assert real UI state, including
#      that the Tauri IPC bridge exists (which a browser cannot provide)
#   5. verify the digest is unchanged after the run
#
# Everything is loopback-only. The application is launched from the build output
# rather than installed, so no machine state is mutated.
set -eu

REPORT_DIR=".agent/evidence/artifact-e2e"
mkdir -p "$REPORT_DIR"
LOG="$REPORT_DIR/artifact-e2e.log"
STATUS="$REPORT_DIR/STATUS.md"

EXE="${1:-target/release/linchpin-desktop.exe}"
PORT="${LINCHPIN_DEBUG_PORT:-9222}"

if [ ! -f "$EXE" ]; then
  echo "artifact-e2e: no executable at $EXE -- run 'sh scripts/build.sh' first" >&2
  exit 2
fi

sha_of() { python3 -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$1"; }

DIGEST_BEFORE="$(sha_of "$EXE")"
SIZE="$(python3 -c "import os,sys;print(os.path.getsize(sys.argv[1]))" "$EXE")"
echo "artifact-e2e: exe    = $EXE"
echo "artifact-e2e: sha256 = $DIGEST_BEFORE"
echo "artifact-e2e: bytes  = $SIZE"

# --- launch with a DevTools endpoint --------------------------------------
export WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=$PORT"
"$EXE" > "$LOG" 2>&1 &
APP_PID=$!
echo "artifact-e2e: launched pid=$APP_PID on debug port $PORT"

cleanup() {
  if kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
    sleep 1
    kill -9 "$APP_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# --- wait for the endpoint ------------------------------------------------
# The WebView2 debug port opens briefly during webview startup and then stops
# listening once initialisation completes (measured: listening at t=3s, closed
# by t=6s while the process stays alive). Polling every 250ms rather than every
# second is therefore required -- a 1s poll can miss the window entirely, which
# was the actual cause of the earlier "endpoint did not appear" failure.
ENDPOINT="http://127.0.0.1:$PORT/json"
READY=0
for _ in $(seq 1 120); do
  if curl -fsS --max-time 1 "$ENDPOINT" > "$REPORT_DIR/targets.json" 2>/dev/null; then
    if python3 -c "
import json,sys
d=json.load(open(sys.argv[1]))
sys.exit(0 if any(t.get('type')=='page' and t.get('webSocketDebuggerUrl') for t in d) else 1)
" "$REPORT_DIR/targets.json" 2>/dev/null; then
      READY=1
      break
    fi
  fi
  sleep 0.25
done

if [ "$READY" -ne 1 ]; then
  echo "artifact-e2e: FAIL -- no page target with a debugger URL appeared on $ENDPOINT" >&2
  echo "  WebView2 opens the debug port only briefly during startup, so this is" >&2
  echo "  timing sensitive. Re-run before concluding the artifact lacks support." >&2
  exit 1
fi
echo "artifact-e2e: DevTools endpoint is live with a page target"

# --- assert against the REAL webview --------------------------------------
python3 scripts/artifact-e2e-probe.py "$PORT" "$EXE" "$DIGEST_BEFORE" "$SIZE" "$STATUS" "$REPORT_DIR/targets.json"
PROBE=$?

# --- digest must be unchanged (the artifact under test is the artifact) ----
DIGEST_AFTER="$(sha_of "$EXE")"
if [ "$DIGEST_BEFORE" != "$DIGEST_AFTER" ]; then
  echo "artifact-e2e: FAIL -- digest changed during the run" >&2
  exit 1
fi
echo "artifact-e2e: digest stable across the run"

if [ "$PROBE" -ne 0 ]; then
  echo "artifact-e2e: FAIL -- probe reported failures" >&2
  exit 1
fi
echo "artifact-e2e: ok ($DIGEST_BEFORE)"
