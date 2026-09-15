# ADR-005: Human sign-off gates remain EXTERNAL_REQUIRED

Status: **ACCEPTED** (human decision, 2026-09-14)
Related: PF-017, PF-018, DOD-039, DOD-042

## Context

`PREFLIGHT.md` classifies two prerequisites as `HUMAN_EXTERNAL`:

| ID | Requirement | Stated failure effect |
| --- | --- | --- |
| PF-017 | Human UAT and manual accessibility validators | blocks broad GA if absent |
| PF-018 | Patent-workflow independent reviewer | blocks broad GA if absent |

DOD-039 states the rule directly: "Human UAT, manual assistive-technology
validation, legal/compliance review, physical hardware/HSM work, and accredited
assessment are signed only by the required real participants", and its OR ELSE is
"Status remains EXTERNAL_REQUIRED and GO is prohibited when the gate is
mandatory."

Automated UAT cannot substitute for these. DOD-039's own BECAUSE clause gives the
reason: "AI or automated tooling cannot impersonate business acceptance, lived
accessibility use, hardware evidence, or professional certification."

## Decision

The owner **confirms both gates stay `EXTERNAL_REQUIRED`**. They are not waived,
not deferred to a later release, and not replaced with automated approximations.

## Binding consequences

1. **DOD-039 remains `EXTERNAL_REQUIRED`.** It cannot become PASS by any agent or
   automated action.
2. **The best lawful release verdict is `CONDITIONAL_EXTERNAL_GATES`, not `GO`.**
   `scripts/ship-gate.py` restricts the verdict to GO / NO_GO /
   CONDITIONAL_EXTERNAL_GATES / INCONCLUSIVE; while DOD-039 is EXTERNAL_REQUIRED
   and mandatory, an unconditional `GO` is prohibited by DOD-039's OR ELSE.
3. **No agent may fabricate these sign-offs.** Not as a placeholder, not as a
   "draft for signature", not as a template pre-populated with a name. DOD-039's
   REQUIRED EVIDENCE is "Named authorized sign-off, scope, date,
   scenarios/evidence, and unresolved findings" — all of which must come from the
   real participant. AG-006 in `ANTI_GAMING_FINDINGS.md` is the precedent: this
   project already had one incident of manufactured live-fire evidence, and this
   ADR exists so a fabricated UAT record cannot be added later as a shortcut.
4. **Automated accessibility checks do not discharge PF-017.** The Playwright
   suite contains an accessibility *baseline* test and its own comment already
   states that "Manual AT validation remains EXTERNAL_REQUIRED and is not implied
   by this test". That boundary is preserved.

## What would close these gates

PF-017: a named validator using assistive technology on the real artifact,
recording scope, date, scenarios and unresolved findings.

PF-018: a named patent-workflow reviewer producing a signed review scope and
evidence.

Both are human actions. Until they exist, the release verdict must carry the
external-gate condition explicitly rather than being reported as an unqualified
GO.
