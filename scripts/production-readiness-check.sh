#!/usr/bin/env bash
set -eu
[ -f .agent/verification/reports/RELEASE_GATE.json ] || exit 2
python3 - <<'PY2'
import json
x=json.load(open('.agent/verification/reports/RELEASE_GATE.json'))
if x.get('verdict') not in {'GO','CONDITIONAL_EXTERNAL_GATES'}: raise SystemExit('production-readiness: release verdict is not GO/CONDITIONAL_EXTERNAL_GATES')
print('production-readiness: ok')
PY2
