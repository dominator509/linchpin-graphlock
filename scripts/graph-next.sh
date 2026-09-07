#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
from pathlib import Path
import re
text=Path('.agent/GRAPH.md').read_text()
block=re.search(r'GRAPH-TABLE-BEGIN\n(.*?)\nGRAPH-TABLE-END',text,re.S).group(1)
nodes=[]
for line in block.splitlines():
 m=re.fullmatch(r'NODE\s+(\S+)\s+DEPS\s+(.+)',line.strip())
 nodes.append((m.group(1),[] if m.group(2)=='-' else m.group(2).split(',')))
ledger=Path('.agent/state/LEDGER.md').read_text() if Path('.agent/state/LEDGER.md').exists() else ''
done=set(re.findall(r'\|\s*(EP-\d{3})\s*\|\s*DONE_VERIFIED\s*\|',ledger))
blocked=set(re.findall(r'\|\s*(EP-\d{3})\s*\|\s*CLOSED_BLOCKED\s*\|',ledger))
for n,deps in nodes:
 if n in done or n in blocked: continue
 if all(d in done for d in deps):
  plans=sorted(Path('.agent/execplans').glob(n+'-*.md'))
  print('NEXT',n,str(plans[0]) if plans else 'MISSING_EXECPLAN')
  raise SystemExit
unclosed=[n for n,_ in nodes if n not in done and n not in blocked]
if not unclosed: print('ALL_DONE')
else: print('RUN_BLOCKED dependencies prevent: '+','.join(unclosed))
PY2
