ADR-001: Toolchain version mismatch reconcilliation between current environment and TOOLCHAIN_PINS.md.

ADR-002: MPL-2.0 transitive dependencies from the Tauri foundation (cssparser,
cssparser-macros, dtoa-short, selectors, option-ext). LICENSE_ALLOWLIST.md
requires "explicit file-level review" and a legal/license ADR before MPL-2.0 may
enter distributable core. Status: PROPOSED, awaiting human legal decision under
AGENTS.md section 5(c). Full analysis, options and gate impact:
.agent/evidence/ADR-002-mpl-dependencies.md. Not silently reconciled: the
licenses gate remains intentionally red for exactly these five crates.

