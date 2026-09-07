#!/usr/bin/env bash
set -eu
[ -f scripts/run-live-fire-real.sh ] || { echo "live-fire: real production-path runner absent until EP-007" >&2; exit 2; }
sh scripts/run-live-fire-real.sh
