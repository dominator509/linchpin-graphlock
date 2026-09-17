# EP-009 / M1 — Produce frozen Windows installer/app artifacts

> **CURRENT STATE (added later; the body below is the historical record at
> candidate `41e0b20` and is preserved, not rewritten).**
>
> M1 is **DONE**. The build-blocking icon defect documented below was repaired,
> the full Windows pipeline builds both formats from the current candidate, and
> the "Remaining" list has since been discharged item by item:
>
> | Item the historical body lists as open | Current state |
> | --- | --- |
> | No installer install/launch executed | executed — `.agent/evidence/EP-009/M3/STATUS.md` (real `msiexec`, ARP read-back, launch with window title) |
> | Artifacts unsigned | ADR-003 records an unsigned release as an accepted limitation |
> | No SBOM/notices/provenance | done — `.agent/evidence/EP-009/M2/STATUS.md` |
> | No rollback/uninstall preservation proof | done, cross-version — `.agent/evidence/EP-009/M4/STATUS.md` |
> | Digests bound to a dirty tree | superseded: `RUN_MANIFEST.json` records the dirty state explicitly (`working_tree.dirty: true`) instead of implying a frozen revision |
>
> Artifact identity at the current candidate (`.agent/verification/state/RUN_MANIFEST.json`):
> executable `00286e822ad93a84…` (12,500,992 B), MSI `a29cdb38c463594a…` (5,709,824 B),
> NSIS setup produced by the same build. Rebuilds are byte-identical (SUP-004,
> `-C link-arg=/Brepro`).

Status at the time of writing: **IN_PROGRESS** (partial). M1 is not closed; see "Remaining" below.

## Candidate identity

| Field | Value |
| --- | --- |
| Base revision | `962e36575af5ae4590ec49143a401ec285eedaf3` (origin/main) |
| Toolchain | `rustc 1.98.0 (88d9e12ae 2026-08-18)`, `cargo 1.98.0 (797e8a9bc 2026-08-05)` |
| Target triple | `x86_64-pc-windows-msvc` |
| Host | Windows 10 Home 10.0.19045, x64 |
| Node / pnpm | see `node --version` / `pnpm --version` (EP-000 pins) |

## Defect found and repaired (real, reproduced)

`cargo build --workspace` **failed** on the desktop crate at base revision. Exact first-failure log:

```
error: failed to run custom build command for `linchpin-desktop v0.1.0`
  --- stdout
  Microsoft (R) Windows (R) Resource Compiler Version 10.0.10011.16384
  ...\out\resource.rc(26) : error RC2175 : resource file
  ...\apps\desktop\src-tauri\icons\icon.ico is not in 3.00 format
  thread 'main' panicked at tauri-winres-0.3.6\src\lib.rs:543:14
  ... Failed("RC.EXE failed to compile specified resource file")
[exit=101]
```

### Root cause (measured, not inferred)

All five declared icon assets were **the same file** — a PNG, byte-identical,
with `.ico`/`.icns` extensions:

| File | Bytes | Magic | IHDR dims | SHA-256 (first 16) |
| --- | --- | --- | --- | --- |
| `32x32.png` | 224566 | `137,80,78,71` | 800x600 | `C4E870D358D64CE6` |
| `128x128.png` | 224566 | `137,80,78,71` | 800x600 | `C4E870D358D64CE6` |
| `128x128@2x.png` | 224566 | `137,80,78,71` | 800x600 | `C4E870D358D64CE6` |
| `icon.icns` | 224566 | `137,80,78,71` | 800x600 | `C4E870D358D64CE6` |
| `icon.ico` | 224566 | `137,80,78,71` | 800x600 | `C4E870D358D64CE6` |

`137,80,78,71` is the PNG signature. `icon.ico` was therefore not an ICO
container at all, so `tauri.conf.json`'s `bundle.icon` list was unsatisfiable
and no Windows artifact could ever be produced.

### Repair

Added `scripts/generate-icons.py` (Python stdlib only — `struct`/`zlib`; **no new
dependency**, per AGENTS.md §10). It performs a genuine format conversion:

- Decodes the source PNG (real IHDR/IDAT/zlib inflate + all 5 PNG filter types).
- Box-filter downscales with premultiplied alpha to 16/24/32/48/64/128/256.
- Writes a real `ICONDIR`/`ICONDIRENTRY` container with PNG-compressed entries.
- Writes a real `icns` container with `icp4`/`icp5`/`icp6`/`ic07`/`ic08` elements.
- **Re-parses the emitted ICO** and asserts structure before reporting success.

Run: `python3 scripts/generate-icons.py apps/desktop/src-tauri/icons/source-icon.png apps/desktop/src-tauri/icons`

```
source: apps\desktop\src-tauri\icons\source-icon.png 800x600 channels=4
wrote icon.ico (100542 bytes, 7 sizes [16, 24, 32, 48, 64, 128, 256])
wrote icon.icns (95828 bytes)
ICO verification: ok
[exit=0]
```

### Proof of repair (post-change rerun)

```
cargo build --workspace
   Compiling tauri v2.11.5
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.01s
[exit=0]
```

`cargo test --workspace --locked`: **25 suites, 40 passed, 0 failed, 0 ignored**, 13 test binaries, `[exit=0]`.

## Toolchain evidence (command source: COMMANDS.md / package.json scripts)

`@tauri-apps/cli` was verified as a **declared** devDependency in
`apps/desktop/package.json` (`"@tauri-apps/cli": "^2.6.0"`) with a `tauri`
script — so the bundle command is transcribed from repository evidence, not
invented (AGENTS.md §7).

```
pnpm install --frozen-lockfile     -> Done in 6.9s, exit 0
pnpm --filter @linchpin/desktop build
  tsc && vite build
  dist/index.html                  0.33 kB
  dist/assets/index-BnGKEOql.js  194.85 kB | gzip: 61.01 kB
  ✓ built in 1.25s, exit 0
pnpm --filter @linchpin/desktop exec tauri build
   Finished `release` profile [optimized] target(s) in 7m 30s
   Built application at: target\release\linchpin-desktop.exe
   Running candle ... Running light ... makensis ...
   Finished 2 bundles at:
     target\release\bundle\msi\LINCHPIN_0.1.0_x64_en-US.msi
     target\release\bundle\nsis\LINCHPIN_0.1.0_x64-setup.exe
   exit 0
```

## Frozen artifact identity (DOD-003, DOD-029)

| Artifact | Size (bytes) | SHA-256 |
| --- | --- | --- |
| `target/release/linchpin-desktop.exe` | 8,689,664 | `6E09A3DD7308248BA73F5CAC86AB2443D50344280AAAAF41678CB605EAE2D2A1` |
| `target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi` | 3,002,368 | `D28C0B9400A83E39B8EBAC1687FE9219A11845F5F9444A974DB5CE479E11784E` |
| `target/release/bundle/nsis/LINCHPIN_0.1.0_x64-setup.exe` | 1,964,747 | `6D9B45A40135F0173A70AFECA3BA85A84440E6F3ED240BEA0ABBE6D1F3575081` |

### Independent read-back (DOD-012 method)

Rather than trusting the presence of files, the MSI was opened through the
**Windows Installer COM API** — an independent reader, not the build tool — and
its property table queried:

```
magic bytes: MSI = D0CF11E0 (OLE compound file), NSIS/exe = MZ (PE)
ProductName    = LINCHPIN
ProductVersion = 0.1.0
Manufacturer   = linchpin
ProductLanguage= 1033
UpgradeCode    = {F521FA25-961F-5B0D-BF96-2961D39CC086}
ProductCode    = {A65DA36B-1000-4B8F-A75A-E0AF8D757B57}
ALLUSERS       = 1
```

The MSI is a structurally valid Windows Installer database carrying the correct
product identity, not a renamed or empty file.

### Negative case (DOD-014)

At base revision the same command **failed** (`RC2175`, exit 101) — recorded
above. The gate therefore discriminates: it fails on the broken input and
passes on the repaired input.

## Remaining (M1 NOT closed)

1. **No installer install/launch has been executed.** The artifacts are built and
   independently parsed, but nothing has been installed on a clean Win10/11
   target and no golden path has run through the installed binary (DOD-034/DOD-004).
   Running the MSI here would mutate this developer machine; that is M3's scope
   on a reference VM (PF-016), which does not exist.
2. **Artifacts are unsigned.** PF-015: no Windows signing identity is present,
   so `signtool verify` cannot pass and M2/M4 cannot close.
3. **No SBOM/notices/provenance manifest** (M2).
4. **No rollback/uninstall vault-preservation proof** (M4).
5. The digests above are bound to a **dirty working tree** (`apps/desktop/src-tauri/gen/` untracked); the candidate commit for these digests is `41e0b20` plus untracked generated schema files.

## Honest status

M1 = **PARTIAL, substantially advanced.** A genuine build-blocking defect was
found, root-caused by measurement, and repaired; the full Windows installer
pipeline now runs green end-to-end from `pnpm install` to two signed-format
bundles whose integrity was independently verified. M1 is **not** closed because
its distribution acceptance (clean-machine install/boot/golden path) and the
signing gates are unmet.
