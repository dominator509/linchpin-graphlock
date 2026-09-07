#!/usr/bin/env bash
set -eu
python3 scripts/validate-generated-pack.py .
echo "harness-init: blueprint valid; verification state remains NOT_STARTED until a candidate exists"
