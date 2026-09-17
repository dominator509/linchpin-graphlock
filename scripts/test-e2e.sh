#!/usr/bin/env bash
# End-to-end gate: two lanes, both required.
#
# LANE 1 -- Playwright against the built bundle over loopback. Before ADR-004
# this script invoked `pnpm -r test:e2e` while NO package declared a
# `test:e2e` script, so it failed with ERR_PNPM_RECURSIVE_RUN_NO_SCRIPT. The
# desktop UI had zero automated tests.
#
# LANE 2 -- exact-artifact E2E (DOD-004). The clause requires E2E "against the
# exact production artifact digest, not merely source or a development server",
# and lane 1 uses `vite preview`, which is a development server. Lane 2 drives
# the packaged executable's real WebView2 over CDP instead, and additionally
# proves the Tauri IPC bridge exists and that commands round-trip -- none of
# which a browser run can demonstrate.
#
# Both lanes must pass; a failure in either fails the gate.
set -eu

[ -f apps/desktop/package.json ] || { echo "e2e: desktop product not bootstrapped" >&2; exit 2; }

# The preview server serves the built bundle, so it must exist first.
if [ ! -f apps/desktop/dist/index.html ]; then
  echo "e2e: building production bundle (dist/ absent)"
  pnpm --filter @linchpin/desktop build
fi

# --- port hygiene on 4173 -------------------------------------------------
# Measured failure: an interrupted run of this lane (a killed doc-exec pass) left
# its `vite preview` listening on 4173, and every later run failed within three
# seconds with Playwright's "http://localhost:4173 is already used". That message
# names no process, so the cause was invisible for several runs. This lane now
# reports the OWNING process before failing, and clears its own leftovers on the
# way out -- a leaked server otherwise breaks the NEXT run, which is how this was
# found.
PREVIEW_PORT=4173
port_pids() {
  python3 - "$PREVIEW_PORT" <<'PY'
import subprocess, sys
port = sys.argv[1]
try:
    out = subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         f"Get-NetTCPConnection -LocalPort {port} -State Listen -ErrorAction SilentlyContinue "
         f"| Select-Object -ExpandProperty OwningProcess -Unique"],
        capture_output=True, text=True, timeout=30,
    ).stdout
except Exception:
    out = ""
print(" ".join(line.strip() for line in out.split() if line.strip().isdigit()))
PY
}

OCCUPIED="$(port_pids)"
if [ -n "$OCCUPIED" ]; then
  echo "e2e: FAIL -- port $PREVIEW_PORT is already in use by pid(s): $OCCUPIED" >&2
  for pid in $OCCUPIED; do
    python3 - "$pid" <<'PY' >&2
import subprocess, sys
pid = sys.argv[1]
out = subprocess.run(
    ["powershell", "-NoProfile", "-Command",
     f"(Get-CimInstance Win32_Process -Filter \"ProcessId = {pid}\").CommandLine"],
    capture_output=True, text=True, timeout=30,
).stdout.strip()
print(f"  pid {pid}: {out or 'command line unavailable'}")
PY
  done
  echo "  This lane refuses to reuse a server it did not start (reuseExistingServer=false)." >&2
  echo "  Stop that process and rerun; an interrupted earlier run of this lane is the usual cause." >&2
  exit 2
fi

cleanup_preview() {
  LEFTOVER="$(port_pids)"
  if [ -n "$LEFTOVER" ]; then
    echo "e2e: clearing leftover preview server on $PREVIEW_PORT (pid $LEFTOVER)"
    for pid in $LEFTOVER; do
      python3 - "$pid" <<'PY'
import subprocess, sys
subprocess.run(["taskkill", "/PID", sys.argv[1], "/T", "/F"], capture_output=True)
PY
    done
  fi
}
trap cleanup_preview EXIT

echo "=== e2e lane 1/4: Playwright against the built bundle (system Edge) ==="
pnpm --filter @linchpin/desktop exec playwright test

echo "=== e2e lane 2/4: exact-artifact E2E (DOD-004) ==="
sh scripts/artifact-e2e.sh

# LANE 3 -- installer data preservation (EP-009 M4). DOD-035 requires upgrade,
# rollback and compatibility paths be executed "with realistic persistent state",
# and DOD-036 requires recovery claims be "executed against reconciled state".
# This lane installs the real MSI, seeds a real vault-shaped database at the
# product's real app-data path, then proves the data survives an update
# (install-over-install), an uninstall and a rollback (reinstall after removal).
# Previously NOT executed; M4's "signed" half is covered by ADR-003.
echo "=== e2e lane 3/4: installer vault preservation (EP-009 M4) ==="
if [ ! -d target/release/bundle/msi ]; then
  echo "e2e: building the MSI first (bundle/msi absent)"
  sh scripts/build.sh
fi
sh scripts/vault-preservation-e2e.sh

# LANE 4 -- CROSS-VERSION matrix (DOD-035). Lane 3 installs the same v0.1.0 MSI at
# every step, so it cannot show an upgrade ACROSS versions, which is what DOD-035
# names. This lane builds a second release (v0.2.0) when the staged artifacts are
# absent, then executes install -> upgrade -> downgrade attempt -> uninstall ->
# rollback against realistic persistent state, binding the installed binary to each
# package's OWN payload digest (measured: the bundler relinks the binary, so
# target/release is not the shipped identity) and observing the upgraded binary
# actually launch.
echo "=== e2e lane 4/4: cross-version upgrade/downgrade/rollback matrix (DOD-035) ==="
if [ ! -d target/version-matrix/0.2.0 ]; then
  python3 scripts/version-matrix.py --provision
else
  python3 scripts/version-matrix.py
fi

echo "e2e: ok (all lanes)"
