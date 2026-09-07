#!/usr/bin/env bash
set -eu
[ -f scripts/smoke-installed-artifact.sh ] || { echo "smoke: exact-artifact smoke runner absent until EP-009" >&2; exit 2; }
sh scripts/smoke-installed-artifact.sh
