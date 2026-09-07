#!/usr/bin/env bash
set -eu
cmd="${1:-tail}"; n="${2:-30}"; f=".agent/state/LEDGER.md"
case "$cmd" in
 tail) tail -n "$n" "$f" ;;
 *) echo "ledger: unsupported command: $cmd" >&2; exit 2;;
esac
