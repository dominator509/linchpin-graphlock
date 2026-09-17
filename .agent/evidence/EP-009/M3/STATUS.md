# EP-009 / M3 — Exact-artifact install / launch / uninstall (DOD-004, SUP-003)

> **CURRENT STATE (added later; the body below is the historical record at
> candidate `1604012` with MSI `2521FE61…` (3,018,752 B) and is preserved, not
> rewritten).**
>
> M3 is **DONE for the host it can run on**, and its five honest limits have
> since been resolved or reclassified:
>
> | Limit listed below | Current state |
> | --- | --- |
> | 1. Not a clean room (DOD-034, PF-016) | still open, and now recorded as `EXTERNAL_REQUIRED`: no virgin host exists. Scope narrowed to Windows 10+ by ADR-004; Windows 11 is **not claimed** |
> | 2. Not signed (PF-015) | ADR-003: unsigned release accepted as a documented limitation |
> | 3. No SBOM/notices/provenance shipped | done — `.agent/evidence/EP-009/M2/STATUS.md` |
> | 4. No golden-path interaction through the UI | done at the artifact boundary: `scripts/artifact-e2e.sh` drives the packaged executable's real WebView2 over CDP, proves the IPC bridge round-trips and runs 15 assertions against the exact artifact digest (`.agent/evidence/artifact-e2e/STATUS.md`) |
> | 5. Windows 10 only | unchanged and deliberate: ADR-004 declares Windows 10+ as the clean-room scope and makes no Windows 11 claim |
>
> The lifecycle was re-executed at the current candidate by the smoke lane
> (`sh scripts/smoke-test.sh`, silent install → ARP read-back → launch with a real
> window title → clean uninstall) and by the installer lanes of
> `sh scripts/test-e2e.sh` (vault preservation and the cross-version matrix).

Status at the time of writing: **PARTIAL — real artifact lifecycle proven; clean-room and signature gates still open.**

Candidate: `1604012`, host Windows 10 Home 10.0.19045 x64.

## Artifact under test

| Field | Value |
| --- | --- |
| Artifact | `target/release/bundle/msi/LINCHPIN_0.1.0_x64_en-US.msi` |
| SHA-256 | `2521FE6151671424E77A1D6F9988BBCD5A73DE97E0DD01727C3D08D09835B7A3` |
| Size | 3,018,752 B |
| Build | `sh scripts/build.sh` → exit 0 |

## Procedure and results

This is a real install of the real installer on the real host — not a source-tree
test and not a mock.

### 1. Silent install

```
msiexec /i LINCHPIN_0.1.0_x64_en-US.msi /qn /norestart /l*v msi-install.log
exit = 0
```

### 2. Independent read-back (second observer, DOD-012)

Filesystem — `C:\Program Files\LINCHPIN\`:

| File | Size |
| --- | --- |
| `linchpin-desktop.exe` | 8,735,744 |
| `linchpin_desktop_lib.dll` | 144,896 |
| `Uninstall LINCHPIN.lnk` | 949 |

Registry (Add/Remove Programs):

```
DisplayName     : LINCHPIN
DisplayVersion  : 0.1.0
Publisher       : linchpin
InstallLocation : C:\Program Files\LINCHPIN\
```

### 3. Launch the INSTALLED binary (the artifact, not the dev build)

```
Start-Process "C:\Program Files\LINCHPIN\linchpin-desktop.exe"
PROCESS ALIVE
WindowTitle = 'LINCHPIN Patent Intelligence OS'
Responding  = True
Path        = C:\Program Files\LINCHPIN\linchpin-desktop.exe
```

The installed application boots, creates a real window with the correct title,
and is responsive to the window manager. This crosses the actual desktop/UI
boundary required by AGENTS.md §9.

**Note a real discrepancy worth recording:** the installed executable hashes
`37EE7E0A24E3759457351C340C643FB750C0618E75ED9D3AEC34D168C0231F93`, which is
**not** the `target/release/linchpin-desktop.exe` digest. The Tauri bundler
patches the binary with bundle-type information (visible in the build log:
"Patching ... with bundle type information: msi"). The digest that matters for
distribution is therefore the **installed** one, not the pre-bundle one.

### 4. Silent uninstall

```
msiexec /x LINCHPIN_0.1.0_x64_en-US.msi /qn /norestart /l*v msi-uninstall.log
exit = 0
```

Post-conditions verified:

| Check | Result |
| --- | --- |
| `C:\Program Files\LINCHPIN` exists | **False** |
| Add/Remove Programs entry remains | **False** |

Clean removal with no residue.

## What this proves

- The MSI is genuinely installable by Windows Installer, not merely a valid
  container (DOD-003 → DOD-004 progression).
- The installed artifact independently demonstrates correct product identity and
  launches to a responsive window (DOD-011 public entry point, DOD-012
  independent observation).
- Uninstall is clean and reversible.

## What this does NOT prove (honest limits)

1. **Not a clean room.** This host has the full Rust/Node/WebView2 toolchain
   installed. No zero-state machine or VM was used, so hidden prerequisite
   dependencies remain possible (DOD-034, E2E-011, PF-016 unmet).
2. **Not signed.** No signing identity is present (PF-015), so
   `signtool verify` cannot pass and the artifact is not provenance-bound.
3. **No SBOM/notices/provenance** artifact is shipped (EP-009 M2).
4. **No golden-path interaction.** The app was launched and observed; no
   user-visible workflow was driven through the UI.
5. **Windows 10 only.** Windows 11 was not exercised (SUP-008 unmet).
6. The install mutated this developer machine; those changes were reverted by
   the uninstall, which is itself verified above.

## Status

**PARTIAL.** The exact-artifact install/launch/uninstall lifecycle is genuinely
proven for one artifact on one host. M3 cannot close without a clean-room
target (PF-016) and signature verification (PF-015).
