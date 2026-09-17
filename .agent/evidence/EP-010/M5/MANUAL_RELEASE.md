# EP-010 / M5 — Manual release instructions

Derived by `scripts/residual-risk-report.py`, so the identity block below cannot drift from
`RUN_MANIFEST.json`. Auto-deploy authorization is `no` (AGENTS.md section 5e): the artifact is
ship-ready, the deployment is MANUAL.

## 1. What is being released, exactly

| Field | Value |
| --- | --- |
| Candidate commit | `6ef5968` |
| Epoch | `75f078ac0961b54a8be5e5363e78369e519e993dd939f0c08bbb65851603e200` over 156 tracked inputs |
| Executable | `target\release\linchpin-desktop.exe` — `2c94241d1e10f1f23163c673b15e8c9d377a73f8a77f16d0eb674a8fed64bcb3` (12500992 B) |
| Installer (MSI) | `target\release\bundle\msi\LINCHPIN_0.1.0_x64_en-US.msi` — `f9ac3296af27494f31ec75cbe1647e95d78705e204a77ee5863d26c065f66a05` (5709824 B) |
| Ship-gate verdict | **CONDITIONAL_EXTERNAL_GATES** (blocking clauses: none) |

## 2. Preconditions

- Target: Windows 10 or higher, x64 (ADR-004; Windows 11 is **not** claimed as verified).
- The artifact is **unsigned** (ADR-003): SmartScreen and some AV products will warn. That is the
  documented, accepted condition, not a defect to work around.
- A verified backup of any existing vault exists before upgrading, and `ROLLBACK.md` has been read.

## 3. Install / verify / roll back, in published commands only

The commands are the published operator documentation, and `scripts/doc-exec.py` executes exactly
these documents as a gate, so they are not aspirational:

1. **Install** — `DEPLOYMENT.md`: `msiexec /i <msi> /qn /norestart /l*v install.log`, or run the NSIS
   `LINCHPIN_0.1.0_x64-setup.exe`.
2. **Read back independently** — `DEPLOYMENT.md`: query Add/Remove Programs for `DisplayName`,
   `DisplayVersion`, `InstallLocation`; hash the installed executable and compare it with the package's
   own payload (extract with `msiexec /a`), never with `target/release`, because the bundler relinks
   the binary during packaging.
3. **Smoke** — launch the installed `linchpin-desktop.exe`, confirm the window title and that the
   process is responding.
4. **Upgrade** — install the newer MSI over the existing installation; the user's vault at
   `%LOCALAPPDATA%\LINCHPIN\linchpin-vault.db` must survive (measured in the cross-version matrix).
5. **Roll back** — `ROLLBACK.md`: reinstall the previous artifact; the vault is never deleted by a
   binary rollback.
6. **Uninstall** — `msiexec /x <msi> /qn /norestart`; confirm the program directory and the ARP entry
   are gone **and** the vault directory is intact.

## 4. What must not be done

- Do not publish, tag, or push a release artifact by automation: publication is manual by policy.
- Do not claim `GO`. The machine gate's verdict is the only lawful release verdict, and it is
  `CONDITIONAL_EXTERNAL_GATES` until the external gates in `RESIDUAL_RISK_AND_EXTERNAL_GATES.md` close.
- Do not treat the abbreviated soak (3600 s, labelled `ABBREVIATED`) as the pack's 24/48/72+ hour scale.

## 5. After release

Record the install on the target machine against REQ-REL-001 (the next lawful manual action in
`NEXT_ACTION.md`); that single record closes DOD-034, E2E-011 and the last unbound requirement.
