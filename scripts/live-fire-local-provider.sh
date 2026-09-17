#!/usr/bin/env bash
# Live-fire proof of the local provider boundary (PF-011 / DOD-010 / DOD-019).
#
# PF-011 is "local llama.cpp/Ollama provider | REQUIRED_BEFORE_E2E", and DOD-010
# requires "at least one real/sandbox dependency execution at the final
# acceptance boundary". Before this gate, `provider_transport` was reachable only
# from its own unit tests -- the adapter was inert and no live provider proof
# existed (PF-011 unmet, DOD-019 PARTIAL).
#
# ABSENCE IS REPORTED, NEVER SKIPPED. If no loopback provider is served this
# script exits 2 naming PF-011. AGENTS.md section 12 forbids silently skipping a
# required test, and DOD-006 forbids a skip without an approved waiver, so this
# gate does not degrade to a pass when the prerequisite is missing.
#
# The artifact is the dedicated devtools-e2e build -- the shipped release opens
# no debug port, and must not. The build MUST go through `tauri build`, not
# `cargo build`: only the tauri pipeline embeds `frontendDist`, so a plain cargo
# build embeds `devUrl` and loads http://localhost:5173. That distinction cost
# two rounds on scripts/artifact-e2e.sh and is preserved here.
set -eu

ENDPOINT="${LINCHPIN_LOCAL_ENDPOINT:-http://127.0.0.1:11434}"
MODEL="${LINCHPIN_LOCAL_MODEL:-smollm2:135m}"
PORT="${LINCHPIN_LOCAL_DEBUG_PORT:-9222}"
REPORT_DIR=".agent/evidence/local-provider"
REPORT="$REPORT_DIR/STATUS.md"
E2E_EXE="target/release/linchpin-desktop-e2e.exe"
PROD_EXE="target/release/linchpin-desktop.exe"
PROD_STASH="target/release/linchpin-desktop.production-stash.exe"

mkdir -p "$REPORT_DIR"

echo "=== local provider live-fire (PF-011) ==="
echo "local-provider: endpoint = $ENDPOINT"
echo "local-provider: model    = $MODEL"

# --- 1. Require the real dependency --------------------------------------
if ! python3 - "$ENDPOINT" <<'PY'
import json, sys, urllib.error, urllib.request
endpoint = sys.argv[1].rstrip("/")
try:
    with urllib.request.urlopen(f"{endpoint}/api/tags", timeout=5) as r:
        body = json.loads(r.read().decode("utf-8", "replace"))
    names = [m.get("name") for m in body.get("models", []) or []]
    print("local-provider: provider reachable; served =", names)
except (urllib.error.URLError, OSError, ValueError, TimeoutError) as e:
    print(f"local-provider: probe failed: {e}")
    raise SystemExit(1)
PY
then
  echo "local-provider: PF-011 UNSATISFIED -- no loopback inference server answered at $ENDPOINT" >&2
  # The runtime is installed at a known path but is NOT a background service, so it
  # stops with the process and every provider-dependent stage then exits 2. MEASURED
  # while settling round 50: the lane failed this way mid-run and the message left the
  # operator to search COMMANDS.md for how to start it. The documented command is now
  # printed here, because a prerequisite failure that does not say how to satisfy it
  # costs a round every time it happens.
  echo "local-provider: start it with the documented procedure (COMMANDS.md, 'Local provider prerequisite'):" >&2
  echo "local-provider:   \$dir = \"\$env:LOCALAPPDATA\\linchpin-local-runtime\"" >&2
  echo "local-provider:   \$env:OLLAMA_HOST='127.0.0.1:11434'; \$env:OLLAMA_MODELS=\"\$dir\\models\"" >&2
  echo "local-provider:   Start-Process -FilePath \"\$dir\\ollama\\ollama.exe\" -ArgumentList 'serve' -WindowStyle Hidden" >&2
  echo "local-provider: missing prerequisite -- NOT a pass and NOT a product failure." >&2
  exit 2
fi

# --- 2. Require the named model to be served -----------------------------
if ! python3 - "$ENDPOINT" "$MODEL" <<'PY'
import json, sys, urllib.request
endpoint, model = sys.argv[1].rstrip("/"), sys.argv[2]
with urllib.request.urlopen(f"{endpoint}/api/tags", timeout=5) as r:
    body = json.loads(r.read().decode("utf-8", "replace"))
names = [m.get("name", "") for m in body.get("models", []) or []]
ok = any(n == model or n.split(":")[0] == model.split(":")[0] for n in names)
print(f"local-provider: served models = {names}")
raise SystemExit(0 if ok else 1)
PY
then
  echo "local-provider: model '$MODEL' is not served; pull it first" >&2
  exit 2
fi

[ -f apps/desktop/src-tauri/tauri-e2e.json ] || {
  echo "local-provider: FAIL -- apps/desktop/src-tauri/tauri-e2e.json is missing" >&2
  exit 2
}

# --- 3. Stash production, build the E2E artifact -------------------------
restore_production() {
  if [ -f "$PROD_STASH" ]; then
    mv -f "$PROD_STASH" "$PROD_EXE"
    echo "local-provider: restored the production artifact"
  fi
}
trap restore_production EXIT

if [ -f "$PROD_EXE" ]; then
  mv -f "$PROD_EXE" "$PROD_STASH"
fi

TAURI_CONFIG="$(cat apps/desktop/src-tauri/tauri-e2e.json)"
export TAURI_CONFIG

echo "local-provider: building the dedicated E2E artifact (tauri build, devtools-e2e)"
( cd apps/desktop && npx tauri build --no-bundle --features devtools-e2e -- --locked ) \
  > "$REPORT_DIR/build.log" 2>&1 || {
  echo "local-provider: FAIL -- tauri build failed; see $REPORT_DIR/build.log" >&2
  tail -20 "$REPORT_DIR/build.log" >&2
  exit 1
}
cp -f "$PROD_EXE" "$E2E_EXE"
echo "local-provider: E2E artifact built"

# --- 4. Drive the real command against the real provider -----------------
echo "local-provider: driving run_local_inference over CDP on port $PORT"
set +e
LINCHPIN_DEBUG_PORT="$PORT" node apps/desktop/e2e-local-provider.mjs \
  "$E2E_EXE" "$REPORT" "$ENDPOINT" "$MODEL"
STATUS=$?
set -e

# --- 5. Restore production and prove no debug port remains ---------------
rm -f "$E2E_EXE"
restore_production
trap - EXIT

echo "local-provider: rebuilding the production artifact from the committed config"
unset TAURI_CONFIG
sh scripts/build.sh > "$REPORT_DIR/prod-build.log" 2>&1 || {
  echo "local-provider: FAIL -- production rebuild failed" >&2
  tail -20 "$REPORT_DIR/prod-build.log" >&2
  exit 1
}

# The inline version of this check had no baseline and no attribution: anything
# else already holding 9222 (a leftover devtools build from an interrupted run)
# made the gate accuse the honest production artifact of opening a debug port.
python3 scripts/assert-no-debug-port.py "target/release/linchpin-desktop.exe" 9222

exit "$STATUS"
