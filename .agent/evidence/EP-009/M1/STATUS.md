# EP-009 / M1 — Produce frozen Windows installer/app artifacts

Status: **IN_PROGRESS** (partial). M1 is not closed; see "Remaining" below.

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

## Remaining (M1 NOT closed)

1. No `cargo tauri build` has been run; **no installer (MSI/NSIS) artifact exists**.
   `@tauri-apps/cli` is not installed in `node_modules/.bin` and is absent from
   `apps/desktop/package.json`, yet `tauri.conf.json` declares
   `bundle.targets: "all"` and `COMMANDS.md`/EP-005 reference `tauri dev`.
   Transcribing a build command requires repository evidence; the CLI dependency
   is a change-control item (needs ADR + lockfile update).
2. No frontend build (`pnpm build`) has been run or verified; `frontendDist: "../dist"` does not exist yet.
3. No artifact digest/SHA-256 can be pinned until (1) and (2) produce a real artifact.

## Honest status

M1 = **PARTIAL**: a genuine build-blocking defect was found and fixed with
reproducible before/after evidence, but the milestone's own acceptance
condition (a frozen Windows installer artifact with a digest) is **not met**.
