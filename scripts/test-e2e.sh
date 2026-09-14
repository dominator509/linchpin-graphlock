#!/usr/bin/env bash
# End-to-end gate.
#
# Before ADR-004 this script invoked `pnpm -r test:e2e` while NO package
# declared a `test:e2e` script, so it failed with
# ERR_PNPM_RECURSIVE_RUN_NO_SCRIPT (exit 1). The desktop UI — the only
# user-facing surface — had zero automated tests.
#
# This runs the real Playwright suite against the production bundle served over
# loopback in a real browser engine (system Edge, per ADR-004). It FAILS LOUDLY
# when a browser is unavailable rather than silently skipping: a skipped E2E
# suite is a DOD-006 blind spot.
set -eu

[ -f apps/desktop/package.json ] || { echo "e2e: desktop product not bootstrapped" >&2; exit 2; }

# The preview server serves the built bundle, so it must exist first.
if [ ! -f apps/desktop/dist/index.html ]; then
  echo "e2e: building production bundle (dist/ absent)"
  pnpm --filter @linchpin/desktop build
fi

echo "e2e: running Playwright suite (system Edge, real browser engine)"
pnpm --filter @linchpin/desktop exec playwright test

echo "e2e: ok"
