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

echo "=== e2e lane 1/2: Playwright against the built bundle (system Edge) ==="
pnpm --filter @linchpin/desktop exec playwright test

echo "=== e2e lane 2/2: exact-artifact E2E (DOD-004) ==="
sh scripts/artifact-e2e.sh

echo "e2e: ok (both lanes)"
