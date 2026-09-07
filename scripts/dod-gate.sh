#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
import csv
rows=list(csv.DictReader(open('.agent/verification/DOD_REGISTRY.csv',newline='')))
if len(rows)!=42: raise SystemExit('DoD registry count mismatch')
print('dod-registry: 42 clauses present; execution status must be proven by candidate evidence')
PY2
