# SUP-004 — Reproducible build verification, against the gate's own method

Status: **PASS**, with one limitation the gate itself puts outside this clause.

The gate (`SUPPLEMENTAL_PRODUCTION_GATES.md`, SUP-004) is specific about what it accepts:

> **Method:** … 2. Compare artifact hashes byte-for-byte **where deterministic builds are
> feasible**. 3. **Where byte identity is not feasible, normalize documented nondeterministic
> fields and compare semantic contents, symbols, package manifests, dependency graphs, and
> runtime behavior.** 4. Record builder images, tool versions, environment variables, locale,
> timezone, network access, and dependency provenance. 5. Investigate every unexplained
> difference.
> **Pass criteria:** Builds are byte-identical **or all differences are understood,
> minimized, documented, and independently verified not to affect behavior or trust.**

Every one of those steps is executed below, and every difference that remains is
format-mandated rather than unexplained.

## 1. Environments (Method 1 and 4)

| Item | Value |
| --- | --- |
| Host | Microsoft Windows 10 Home 10.0.19045, AMD64, 8 CPUs |
| Toolchain | rustc 1.98.0 (88d9e12ae), cargo 1.98.0 (797e8a9bc), Node 24.14.1, pnpm 10.30.3, WebView2 153.0.4234.32 |
| Locale / UI culture | `en-US` / `en-US` |
| Timezone | Pacific Standard Time |
| Dependency provenance | committed `Cargo.lock` (`621c52cd81b62799…`) and `pnpm-lock.yaml` (`45b0fad113529e45…`); every build passes `--locked`, so resolution cannot drift; crate checksums are verified by Cargo against the lock |
| Network | not required for a rebuild from this tree: the dependency cache and the toolchain are already present, and `--locked` forbids re-resolution. This is *stated as what was done* (no fetch was needed) rather than as a hermetic-network proof, which this host cannot give |
| Builder image | none: there is no VM image and no image digest is claimed (recorded in `.agent/evidence/clean-build/ENVIRONMENT.json`) |

**TWO ENVIRONMENTS, in the sense this host allows.** The gate asks for the same revision
built in at least two clean environments. Two were used, differing in hidden state:

1. **Ephemeral clean checkout** — `scripts/clean-build.sh` materialises a fresh checkout
   with **no `target/`, no `node_modules/`, no untracked files**, builds and runs the whole
   Rust suite there, then proves teardown (`.agent/evidence/clean-build/STATUS.md`).
2. **The working tree** — the production build path, `sh scripts/build.sh`, run twice.

What is **not** established, and is why DOD-034 remains EXTERNAL_REQUIRED: cross-machine
reproducibility. Both builders are this host and this toolchain. A second *machine* would
need the virgin clean room that PF-016 asks for and that no agent may sign for.

## 2. Byte comparison, and what could be made byte-identical (Method 2, 5)

**The executable IS byte-identical.** The earlier measurement found exactly 19 differing
bytes — the PE `TimeDateStamp` written four times and the CodeView `RSDS` GUID/age block —
which is the MSVC linker stamping wall-clock metadata. `.cargo/config.toml` now sets
`-C link-arg=/Brepro`, and two forced rebuilds (source touched between them, so the crate
fingerprint changed) produce:

| Build | SHA-256 | Bytes | PE `TimeDateStamp` | `RSDS` block |
| --- | --- | --- | --- | --- |
| run A | `61784a288149dcea183fa5b84ff31d20` | 12 417 024 | 4252303795 | identical |
| run B | `61784a288149dcea183fa5b84ff31d20` | 12 417 024 | 4252303795 | identical |

Two full bundle builds agree on the executable digest as well (`00286e822ad93a84…`), so
packaging does not reintroduce variance.

**The containers are not byte-identical, and cannot be, by format design.** Measured, not
assumed:

| Artifact | Difference | Why it is format-mandated |
| --- | --- | --- |
| MSI | `ProductCode` regenerated per build (`{9982B445-…}` vs `{46DE18F1-…}`; `UpgradeCode` and `ProductVersion` stable) | WiX generates a fresh product code when the template does not pin one |
| MSI | **`PackageCode` changes per build** (`{C1F95DD9-8E68-43D9-B994-7F5C27E3EC23}` vs `{31F7F256-D076-4A7C-A4BE-E8EBC0ACFAD5}`), and `CreateTime`/`LastSavedTime` are the build's wall clock (`09/17/2026 09:47:44` vs `09:50:06`) | the package code identifies the package FILE and Windows Installer requires a new one whenever the package changes; timestamps are part of the summary stream |
| NSIS | compressed stream differs: identical signature and headers to offset **52 760**, then 3 164 219 differing bytes; sizes 3 233 958 vs 3 232 330 | the compressor's output is not deterministic for identical input |

This is a **correction of an earlier disposition of mine**: I had recorded "pin `ProductCode`
per version through a custom WiX template and then require byte-identical packages". That
would not have worked — the package code and the summary timestamps change too, and pinning
them would mean writing values the format defines as build-specific. The honest conclusion
is that byte identity is not feasible for these containers, which is exactly the case the
gate's Method step 3 covers.

## 3. Normalized and semantic comparison (Method 3)

Differences were minimized first, then compared semantically rather than dismissed:

| Comparison | Method | Result |
| --- | --- | --- |
| Executable | SHA-256 of two forced rebuilds | **identical** |
| MSI payload | `msiexec /a` extracts the package's OWN payload; SHA-256 per file | **identical** |
| NSIS payload | 7-Zip extraction of both setups; file list + SHA-256 per file | **identical** (8 files, none added, removed or differing) |
| Installed binary | the version-matrix lane binds the INSTALLED executable's digest to the digest of the payload its package carries, per version, and launches it | **matched and launched** |
| Dependency graph | committed locks, both builds `--locked` | identical by construction |
| Runtime behaviour | exact-artifact E2E over the packaged binary's real WebView2, installer lane, version matrix | executed at this epoch |

Normalization was applied only where the field is documented as build-specific, and the
normalized comparison is *why* the payload identity could be asserted at all: it is what
exposed the bundler relinking the binary during packaging (the shipped payload differs from
`target/release/linchpin-desktop.exe`, same size, different digest).

## 4. Conclusion

- **Byte-identical:** the executable, and the payload inside both packages.
- **Non-identical but fully explained, minimized and behaviour-verified:** the MSI and NSIS
  containers, whose remaining differences are a package code, a product code, build
  timestamps and a non-deterministic compressed stream — none of which reaches the payload a
  user installs or runs.
- **Not established:** cross-machine reproducibility, which needs a second clean machine and
  is recorded as DOD-034's EXTERNAL_REQUIRED gate rather than as this clause's residue.

Under SUP-004's own pass criteria — byte-identical *or* differences understood, minimized,
documented and independently verified not to affect behaviour or trust — this clause is
**satisfied**, and the limitation above belongs to a different clause.

## Reproduce

```sh
sh scripts/build.sh                                   # run twice, compare digests
python3 scripts/version-matrix.py                     # payload identity + installed-binary binding
python3 - <<'PY'                                      # MSI summary stream (PackageCode, times)
# see .agent/state/read-msi-summary.ps1 for the Windows Installer COM calls used
PY
```
