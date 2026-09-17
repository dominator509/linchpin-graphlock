# SUP-004 — Reproducible build after the `/Brepro` linker switch

Status: **PARTIAL, materially reduced.** The application binary is now **byte-identical**
across rebuilds. The installer packages are not, and the remaining cause is isolated to
installer-package identity rather than to the product.

Candidate: working tree at round 53 (epoch `f8f792114d0ed530`), Windows 10 Home
10.0.19045 x64, `x86_64-pc-windows-msvc`, rustc/cargo 1.98.0.

## What changed

`.cargo/config.toml` (new) sets

```toml
[build]
rustflags = ["-C", "link-arg=/Brepro"]
```

`/Brepro` is the MSVC linker's reproducible-build switch: the PE `TimeDateStamp`
becomes a hash of the image instead of the wall clock, and the debug directory stops
carrying a per-link CodeView `RSDS` GUID. It changes no code, data, import or symbol.

## Measured — the binary is now reproducible

Two forced rebuilds of the release crate (source file touched between them so the
fingerprint changed):

| Build | SHA-256 (first 32 hex) | Bytes | PE TimeDateStamp | RSDS block |
| --- | --- | --- | --- | --- |
| run A | `61784a288149dcea183fa5b84ff31d20` | 12 417 024 | 4252303795 | identical |
| run B | `61784a288149dcea183fa5b84ff31d20` | 12 417 024 | 4252303795 | identical |

Before this change the same experiment produced **19 differing bytes**: the PE
`TimeDateStamp` at `0xF0` plus three copies in `.rdata`, and the 16-byte `RSDS`
GUID/age block (measured and recorded in `EP-009/M2/SUP-004-reproducible-build.md`).

The full-bundle path agrees: two `sh scripts/build.sh` runs produced the same
`linchpin-desktop.exe` digest (`00286e822ad93a84…`) both times, so the repackaging step
does not reintroduce variance into the executable.

## Measured — the INSTALLER packages are still not byte-identical

Two consecutive `sh scripts/build.sh` runs, comparing the artifacts for the version the
tree declares (0.1.0):

| Artifact | Run A | Run B | Identical |
| --- | --- | --- | --- |
| `LINCHPIN_0.1.0_x64_en-US.msi` | `c7f4fce38202dfe3…` | `b282b5da46eea631…` | **no** |
| `LINCHPIN_0.1.0_x64-setup.exe` (NSIS) | `a302be41b49d18c0…` | `8d61c29a3607c485…` | **no** |
| `linchpin-desktop.exe` | `00286e822ad93a84…` | `00286e822ad93a84…` | **yes** |

Read back through the Windows Installer COM API, the MSI's `ProductCode` changes on
every build while `UpgradeCode` and `ProductVersion` are stable:

| Property | Run A | Run B |
| --- | --- | --- |
| `UpgradeCode` | `{F521FA25-961F-5B0D-BF96-2961D39CC086}` | same (stable) |
| `ProductVersion` | 0.1.0 | 0.1.0 |
| `ProductCode` | `{9982B445-762C-4CD6-B161-36EF4610C8B9}` | `{46DE18F1-143C-4C6A-99F3-55A3F679D7B1}` (**changed**) |

WiX generates a fresh `ProductCode` when the package template does not pin one. That is
correct WiX behaviour for a patchable installer and it is the whole of the MSI residual
now that the payload is identical.

### The NSIS residual, isolated (round 58)

The earlier revision of this document said the NSIS difference was "the same class but its
differing bytes have NOT been isolated". They are now measured, and the answer is
different from the MSI's:

| Measure | Run A | Run B |
| --- | --- | --- |
| Setup size | 3 233 958 B | 3 232 330 B (**delta 1 628**) |
| SHA-256 | `fa7dd0103406e28a…` | `8b639e25d55391f6…` |
| Bytes identical up to | offset **52 760** (signature and headers) | — |
| Differing bytes in the common length | 3 164 219 of 3 232 330 | — |
| Extracted payload | 8 files | 8 files, **no file added, removed or differing** |

So the NSIS installer's **compressed stream is not reproducible** — identical headers, then
a divergent compressed block — while the **payload it installs is byte-identical**. That is
a property of the compressor rather than of the build inputs, and it is a different cause
from the MSI's regenerated `ProductCode`.

Raw measurements: `.agent/evidence/sup004/nsis-report.json` and
`.agent/evidence/sup004/nsis-divergence.json`.

One measurement bug was found and corrected while producing this, and it is recorded
because it produced a confidently wrong line: the first isolation script only compared
bytes when the two files had the SAME size, so for two differently-sized setups it printed
"differing ranges: 0" — a statement that reads like "identical" and is not. The divergence
measurement above re-measures the common prefix explicitly.

## Why this is still PARTIAL, and what would close it

PARTIAL, not PASS: the clause is about the reproducible **artifact**, and the artifact a
user installs is the MSI/NSIS package, which still differs between builds. What is now
proven is that the difference is confined to installer containers: the executable is
byte-identical, the MSI payload is identical, and the NSIS payload is identical.

Remediation paths, in order of cost:

1. **Pin `ProductCode` per version** through a custom WiX template or fragment. Tauri's
   configuration schema exposes `upgradeCode` only (checked against
   `@tauri-apps/cli@2.11.4/config.schema.json`), so this requires shipping and
   maintaining a template — a real change with its own upgrade-semantics review, since a
   pinned ProductCode is what makes patching possible and must change between versions.
2. **Make the NSIS compression deterministic**, or stop shipping the NSIS format and ship
   the MSI alone (the MSI is the format the installer lanes and the clean-room procedure
   use). Either is a packaging decision, not a build fix.
3. Then re-run these experiments and require byte-identical packages.


## Method note — a measurement error caught and corrected in this round

The first version of this experiment selected artifacts by glob-sort, which picked
`LINCHPIN_0.2.0_…` left over from the previous round's cross-version provisioning
instead of the freshly built 0.1.0, and reported a false "identical" for both packages.
The selection now reads the version from `apps/desktop/src-tauri/tauri.conf.json` and
refuses an ambiguous or missing match. The stale 0.2.0 bundle artifacts were removed,
and the `exe` result — which was never ambiguous — was unaffected and is confirmed by
the separate binary-only experiment above.

## Reproduce

```sh
python3 scripts/... # (experiment script kept out of the harness; see this document)
sh scripts/build.sh && sha256sum target/release/linchpin-desktop.exe   # twice
```
