#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
import csv
rows=list(csv.DictReader(open('.agent/verification/MASTER_TEST_REGISTRY.csv',newline='')))
acct=list(csv.DictReader(open('.agent/verification/reports/COMPLETE_TEST_ACCOUNTING.csv',newline='')))
print('registry',len(rows),'accounted',len(acct))
if len(acct)!=len(rows): raise SystemExit(2)
PY2
