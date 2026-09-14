#!/usr/bin/env bash
set -eu
python3 scripts/anti-gaming-scan.py .
# Final reality proof additionally requires live-fire once EP-007 exists.
if [ -f scripts/run-live-fire-real.sh ]; then
  sh scripts/live-fire.sh
fi
