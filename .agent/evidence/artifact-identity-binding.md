# Artifact identity binding — a defect that could have bound the release to a test package

Clause: DOD-029 ("the exact candidate commit, base revision, test-overlay revision,
build inputs, and release artifact digest are pinned and recorded"). Found while
settling the EP-010 closure; recorded here because the finding is about the harness,
not about the product.

## What was wrong

`scripts/ship-gate.py` chose the release installer like this:

```python
msis = sorted(MSI_DIR.glob("*.msi"))
msia = msis[-1] if msis else None
```

Two things made that unsafe:

1. **Lexicographic order is not version order.** `LINCHPIN_0.9.0…` sorts after
   `LINCHPIN_0.10.0…`, so "the last one" is not "the newest one" in general.
2. **Foreign packages can be present, and were.** `scripts/version-matrix.py`
   provisions a second release by bumping the version manifests, building through the
   production path, staging the package under `target/version-matrix/<version>/` and
   restoring the manifests. It left the bumped-version package in
   `target/release/bundle/msi`, where the glob found it.

Measured consequence: after the EP-010 settle rerun, `RELEASE_GATE.json` named
`target/release/bundle/msi/LINCHPIN_0.2.0_x64_en-US.msi` (`1a7c171c…`) as the release
artifact while `RUN_MANIFEST.json` pinned
`target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi` (`86118287…`) — a **test
package built from temporarily bumped manifests**, of a version this repository does
not declare, recorded as the release identity. That is precisely the drift DOD-029
exists to prevent.

## How it surfaced

The gate's own coherence check refused the settled state with
`INCONCLUSIVE: candidate identity disagrees: RUN_MANIFEST.json says 04fd659, EPOCH.json
says 7bf5951`, which sent me to the gate file, where the artifact block disagreed with
the manifest. The first reason was a real bookkeeping error of mine (a commit landed
after the epoch was recorded); the artifact disagreement was the more serious find.

## What changed

- **`scripts/ship-gate.py` binds instead of selecting.** It reads the installer path
  and digest that `RUN_MANIFEST.json` pins, re-hashes the file, fails when the digest
  moved ("the artifact moved under the identity"), fails when the file is missing, and
  fails when the file name does not carry the version the repository declares
  (`apps/desktop/src-tauri/tauri.conf.json`, falling back to the Tauri `Cargo.toml`).
  The executable digest is verified against the manifest the same way.
- **`scripts/version-matrix.py` cleans up after itself.** After staging a package it
  removes build outputs of that version from `target/release/bundle/{msi,nsis}` — but
  only when that version is **not** the repository's own, because when the lane builds
  the repository's version that package *is* the release artifact.
- **`ship-gate.py --check` compares more than the verdict.** Measured: substituting the
  pinned installer added the reason "the artifact moved under the identity" while the
  verdict stayed `INCONCLUSIVE`, so `--check` passed and the recorded gate kept
  describing bytes that no longer existed. The check now also compares `artifact`,
  `blocking_clauses`, `external_clauses`, `inconclusive_reasons`, `dod_status_tally`
  and `registry_accounting`.

## Proofs (run, not assumed)

| Proof | Result |
| --- | --- |
| Foreign 0.2.0 package present in the release bundle directory | the gate binds the pinned 0.1.0 installer and does not mention 0.2.0 |
| Pinned installer replaced by different bytes | the gate reports `the pinned installer … hashes 1a7c171c9779124d but RUN_MANIFEST.json recorded 86118287faa9dcaa; the artifact moved under the identity` |
| Gate file left describing substituted bytes, artifact restored, `--check` | exits 1: `the recorded artifact no longer matches the recomputed one` |
| Restored state | `--check` exits 0; the gate file was restored byte-for-byte after each experiment |

## Residual, stated rather than hidden

`scripts/run-manifest.py` pins the installer with a literal path
(`target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi`). It is correct for the
declared version today and the gate's new declared-version check would catch a drift,
but a version bump would need that literal updated. Deriving it from the declared
version is the right fix and is recorded here as outstanding rather than quietly
tolerated.
