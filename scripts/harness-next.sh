#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
import json
p='.agent/verification/state/RUN_STATE.json'
x=json.load(open(p))
stage=x.get('active_stage')
if stage:
    print(stage)
elif x.get('status')=='COMPLETE':
    print('ALL_STAGES_ACCOUNTED')
else:
    # No cursor and not complete: the only honest answer is that the run state is
    # unknown. Printing 'V-000' here (the previous behaviour) named a stage that
    # may already be accounted, which is how this script reported V-000 forever
    # while every stage had a record.
    print('RUN_STATE_UNKNOWN:' + str(x.get('status')))
PY2
