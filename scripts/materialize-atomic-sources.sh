#!/usr/bin/env bash
set -eu
python3 - <<'PY2'
from pathlib import Path
import base64,tarfile,io
p=Path('.agent/verification/atomic-security-sources.tar.gz.b64')
raw=base64.b64decode(''.join(p.read_text().split()))
with tarfile.open(fileobj=io.BytesIO(raw),mode='r:gz') as tf:
 for m in tf.getmembers():
  if not m.isfile(): continue
  dest=Path('.agent/verification/source-library')/m.name
  dest.parent.mkdir(parents=True,exist_ok=True); dest.write_bytes(tf.extractfile(m).read())
print('materialize-atomic-sources: ok')
PY2
