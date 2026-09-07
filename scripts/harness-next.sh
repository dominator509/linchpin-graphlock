#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
import json
p='.agent/verification/state/RUN_STATE.json'; x=json.load(open(p)); print(x.get('active_stage') or 'V-000')
PY2
