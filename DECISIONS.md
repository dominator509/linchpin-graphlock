ADR-001: Toolchain version mismatch reconcilliation between current environment and TOOLCHAIN_PINS.md.

ADR-002: MPL-2.0 dependencies. Status: **ACCEPTED** (human decision, 2026-09-14).
The project owner permitted MPL-2.0 in distributable core. Five crates required the
permission because MPL-2.0 is their only license -- cssparser 0.36.0,
cssparser-macros 0.6.1, dtoa-short 0.3.5, selectors 0.36.1 and option-ext 0.2.0
(four via the Tauri foundation, option-ext via tauri-build). A sixth crate,
htmlescape 0.3.1 (via tantivy -> this project's own storage crate), merely offers
MPL-2.0 among permissive alternatives and never needed the permission; the
earlier finding's claim that the MPL surface was Tauri-only was corrected during
this review. Change control applied: MPL-2.0 added to deny.toml [licenses] allow
with the reviewed crate list, LICENSE_ALLOWLIST.md updated to record the decision,
and scripts/generate-sbom.py moved MPL-2.0 out of REVIEW_REQUIRED so the SBOM
agrees with the gate, as LICENSE_ALLOWLIST.md requires. Effect: cargo deny check
licenses, scripts/security-check.sh and scripts/dependency-audit.sh all return
exit 0 (previously exit 4/1/4 on this single check). Residual obligation: none of
the crates is modified by this project and the notice obligation ships in the
third-party notices; any future vendoring or patching of them, or any NEW
MPL-2.0 crate, requires a fresh ADR. Full analysis:
.agent/evidence/ADR-002-mpl-dependencies.md.

ADR-003: **Unsigned release accepted as a documented limitation** (human decision,
2026-09-14). PF-015 (Windows code-signing identity for GA) is unavailable -- probed
directly: no code-signing certificate exists in the CurrentUser or LocalMachine
store. Rather than leave DOD-003 permanently blocked, the owner accepts an
unsigned release provided the limitation is disclosed wherever the artifact is
distributed. Consequences recorded: DOD-003's "signatures/attestations when
applicable" becomes not-applicable by explicit decision rather than by omission;
no build-provenance attestation is produced; and SmartScreen/AV warnings must be
stated in the operator-facing documentation as an expected condition, not a
defect. This is an accepted risk with an owner, not a waived gate. See
.agent/evidence/ADR-003-unsigned-release.md.

ADR-004: **Clean-room support scope is Windows 10 or higher** (human decision,
2026-09-14). PF-016 asks for clean Win10 *and* Win11 reference targets; no
hypervisor CLI is present on this host and Windows 11 is genuinely unavailable, so
the owner scoped the supported clean-room matrix to Windows 10 or higher rather
than claiming an untested platform. Effect: DOD-034 and the DOD-035 compatibility
matrix cover Windows 10+ only; Windows 11 is not claimed as verified and no
artifact is labelled Win11-compatible until a real target exists. See
.agent/evidence/ADR-004-clean-room-scope.md.

ADR-005: **Human sign-off gates remain EXTERNAL_REQUIRED** (human decision,
2026-09-14). PF-017 (human UAT and manual assistive-technology validation) and
PF-018 (patent-workflow independent reviewer) are human-external and cannot be
satisfied by any agent or automated tooling. The owner confirms they stay
EXTERNAL_REQUIRED rather than being waived, so DOD-039 remains EXTERNAL_REQUIRED
and a GO verdict must be CONDITIONAL_EXTERNAL_GATES at best, never unconditional
GO. No agent may simulate, pre-fill or draft these sign-offs. See
.agent/evidence/ADR-005-external-signoff-gates.md.
