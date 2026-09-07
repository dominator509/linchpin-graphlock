#!/usr/bin/env bash
set -eu
python3 scripts/anti-gaming-scan.py .
# Final reality proof additionally requires live-fire once EP-007 exists.
[ -f scripts/run-live-fire-real.sh ] && sh scripts/live-fire.sh
