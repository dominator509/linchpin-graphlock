#!/usr/bin/env bash
set -eu
stage="${1:?stage required}"; f=$(find .agent/verification/stage-plans -maxdepth 1 -name "${stage}-*.md" -print -quit); [ -n "$f" ] || { echo "unknown stage" >&2; exit 2; }; echo "Stage plan: $f"; echo "Execution requires candidate-specific case materialization; never synthesize PASS."; exit 3
