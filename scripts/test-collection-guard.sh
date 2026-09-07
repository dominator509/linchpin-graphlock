#!/usr/bin/env bash
set -eu
expected="${1:?expected count}"; actual="${2:?actual count}"; [ "$actual" -ge "$expected" ] || { echo "test collection below expected: $actual < $expected" >&2; exit 1; }
