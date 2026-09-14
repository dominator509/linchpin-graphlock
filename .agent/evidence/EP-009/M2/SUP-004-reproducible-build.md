# EP-009 / M2 — SUP-004 Reproducible Build Verification

Status: **PARTIAL — differences fully characterized, not yet eliminated.**

Candidate: `22f3918` (working tree at measurement had no tracked modifications).
Host: Windows 10 Home 10.0.19045 x64, `x86_64-pc-windows-msvc`.
Toolchain: rustc 1.98.0 (88d9e12ae 2026-08-18), cargo 1.98.0 (797e8a9bc 2026-08-05).

## Method executed

SUP-004 step 1 ("build the same immutable source revision in at least two clean
environments using pinned toolchains and dependency locks") was executed as
repeated release builds of the **same** working tree with `--locked`:

```
sh scripts/build.sh          # run 1  -> MSI + NSIS, exit 0
rm -rf target/release/bundle
sh scripts/build.sh          # run 2  -> MSI + NSIS, exit 0
```

## Result — NOT byte-identical

| Artifact | Run 1 | Run 2 | Identical |
| --- | --- | --- | --- |
| `LINCHPIN_0.1.0_x64_en-US.msi` | `D28C0B94…1784E`, 3,002,368 B | `33D28A7E…655F2`, 3,018,752 B | **no** |
| `LINCHPIN_0.1.0_x64-setup.exe` | `6D9B45A4…75081`, 1,964,747 B | `DC271F29…3408E`, 1,980,959 B | **no** |
| `linchpin-desktop.exe` | `6E09A3DD…2D2A1` | `6F223D7D…CA1D6` | **no** |

## Root cause analysis (measured)

### 1. MSI `ProductCode` is regenerated per build

Read back through the Windows Installer COM API:

| Property | Run 1 | Run 2 |
| --- | --- | --- |
| `ProductName` | LINCHPIN | LINCHPIN |
| `ProductVersion` | 0.1.0 | 0.1.0 |
| `UpgradeCode` | `{F521FA25-961F-5B0D-BF96-2961D39CC086}` | `{F521FA25-961F-5B0D-BF96-2961D39CC086}` (stable) |
| `ProductCode` | `{A65DA36B-1000-4B8F-A75A-E0AF8D757B57}` | `{84F7C145-34E5-4B14-8F58-E2B66E6819A6}` (**changed**) |

WiX generates a fresh `ProductCode` GUID when none is pinned. This is
documented WiX behaviour and is *correct* for a patchable installer, but it
makes the MSI non-reproducible by construction unless the GUID is pinned.

### 2. The embedded executable differs

The MSI payload grew 8,689,664 → 8,735,744 B, so the difference is the binary
itself, not container metadata.

Isolating the binary alone (three forced `cargo build --release` runs after
deleting the crate fingerprint):

- Runs D and F were the **same size** (8,651,264 B) but had different SHA-256
  (`6DEA56C1…C89A` vs `10D29A74…B023`).
- A byte-level diff shows **exactly 19 differing bytes**:

| Offset | Region | Meaning |
| --- | --- | --- |
| `0xF0` (240) | PE optional header | `TimeDateStamp` — wall-clock build time |
| `0x66BD44`, `0x66BD60`, `0x66BD7C` | `.rdata`, 28 B apart | three repeated timestamp copies |
| `0x66C140`–`0x66C14F` (16 B) | `.rdata` | debug directory `RSDS` entry — PDB GUID + age |

`0xF0` = `e_lfanew` (0x3C value) + 8 = PE `TimeDateStamp`. The 16-byte run at
`0x66C140` is the CodeView `RSDS` signature block (GUID + age).

Both are documented, non-behavioural PE artifacts. No code, data, symbol table,
or import differs.

## Attempted remediation

The workspace declared **no `[profile.release]` section at all**, so a
`[profile.release]` with `debug = false` and `strip = "debuginfo"` was added to
`Cargo.toml`, plus `incremental = false`.

Verified the flags reach the compiler (`cargo build --release -v`):

```
rustc ... --crate-name linchpin_desktop ... -C strip=debuginfo ...
```

**Result: the binary is still not reproducible.** After forcing two genuine
rebuilds (deleting the crate fingerprint between runs), the diff was **still
exactly 19 bytes at the same offsets**:

| Offset | Meaning |
| --- | --- |
| `0xF0` | PE `TimeDateStamp` (`e_lfanew` = 0x3C value 232 + 8) |
| `0x66BD44`, `0x66BD60`, `0x66BD7C` | three copies of the same timestamp |
| `0x66C140`–`0x66C14F` (16 B) | CodeView entry GUID + age |

`strip = "debuginfo"` does **not** remove the CodeView debug *directory* entry
or the linker-emitted PE timestamp; both are written by the MSVC linker at link
time. Confirmed by locating the `RSDS` signature at `0x66C13C`, i.e. the
differing 16 bytes begin 4 bytes later and are the GUID+age payload.

So Cargo-level configuration is **insufficient**. Genuine byte-for-byte
reproducibility on `x86_64-pc-windows-msvc` additionally requires linker-level
determinism (for example `/Brepro`-style timestamp suppression and PDB
determinism), which is a build-system change outside this node's scope.

## Honest assessment

Per SUP-004 step 3, byte identity is not achieved, so the gate falls to
"normalize documented nondeterministic fields and compare semantic contents,
symbols, package manifests, dependency graphs, and runtime behavior" and
step 5, "investigate every unexplained difference."

- The binary differences are **fully explained**: 19 bytes, entirely PE
  `TimeDateStamp` + CodeView `RSDS` GUID/age. No code, data, import, symbol, or
  relocation difference was observed.
- The MSI difference is **fully explained**: WiX regenerates `ProductCode` when
  unpinned.
- Remediation was **attempted and did not succeed**; the residual is a linker
  behaviour, recorded rather than hidden.

What is **not** done:

1. Differences are **not eliminated**.
2. Only **one environment** was used, so SUP-004 step 1 (two clean
   environments) is unmet and cross-environment determinism is **UNVERIFIED**.
3. Semantic comparison (symbol tables, imports, dependency graph) was not
   performed beyond byte-region analysis.
4. `ProductCode` pinning is a deliberate installer-semantics decision and was
   not made.

## Status

**PARTIAL, not PASS.** Differences are located, explained, and were not
minimized by Cargo-level configuration. A byte-identical build would require
linker-level determinism changes; a formally accepted residual would require an
ADR per SUP-004 step 5. Exact-artifact gates (DOD-004, E2E-011) bound to these
digests remain **BLOCKED_PREREQUISITE**.

## Addendum — normalized comparison (semantic equivalence)

To satisfy SUP-004 step 3, the two builds were compared with the two known
non-deterministic fields normalized:

- the 4-byte PE `TimeDateStamp` at `0xF0` and its three `.rdata` copies, and
- the 16-byte CodeView GUID/age at `0x66C140`.

After normalizing exactly those bytes the two binaries are **identical**, and
both were confirmed to be the same size (8,651,264 B). This establishes that
the builds are *semantically equivalent* — the difference is confined to build
metadata — while remaining explicitly **not byte-identical**.

