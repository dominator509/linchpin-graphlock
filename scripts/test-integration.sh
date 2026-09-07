#!/usr/bin/env bash
set -eu
[ -f scripts/run-integration-real.sh ] || { echo "integration: real-dependency runner absent until EP-003/EP-004" >&2; exit 2; }
sh scripts/run-integration-real.sh
