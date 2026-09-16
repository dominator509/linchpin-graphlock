# REQ-SCOPE-001 — declared scope and the five truth boundaries

Status: **IMPLEMENTED AND EXECUTED.**

SPEC-000 declares two things in one line: "REQ-SCOPE-001 through 012 are UO-01
through UO-12 from PROJECT_BRIEF. Product claims must preserve five truth
boundaries." Before this round both halves existed only as prose — the mapping
in `PROJECT_BRIEF.md`, the boundaries in `ARCHITECTURE.md` section 5 — so no test
could assert either, and a product claim could drift from the promised scope with
nothing to notice it.

## What exists now

| Layer | Artifact | What it does |
| --- | --- | --- |
| Domain | `crates/domain/src/scope.rs` — `SCOPE_MAP`, `TRUTH_BOUNDARIES`, `check_claim_text`, `entry_for_requirement`, `boundary_by_id` | The twelve promised outcomes with their honest state and stated limitation; the five boundaries transcribed from ARCHITECTURE.md with their enforcement point; a word-anchored guard that refuses promise forms |
| Command | `commands.rs` — `get_scope_declaration`, `build_commercialization_package` | The declaration is served over IPC; the guard runs on the ACTUAL exported payload, not on its inputs |
| IPC | `lib.rs` — `get_scope_declaration` | Registered Tauri command |
| UI | `apps/desktop/src/App.tsx` — Settings → *Declared scope and truth boundaries* | Renders the boundaries and a capability table **including each limitation**, from the backend declaration rather than a copy in the UI, so the promise the user reads and the promise the code enforces cannot drift |
| Tests | `crates/domain/src/scope.rs` (4), `commands.rs` (1), `apps/desktop/e2e/shell.spec.ts` (1), `apps/desktop/e2e-artifact.mjs` (3 assertions) | Unit, product-boundary, browser-lane and exact-artifact evidence |
| Harness | `scripts/mutation-proof.py` — `MUT-SCOPE-001-a`, `MUT-SCOPE-001-b` | Controlled defects, both CAUGHT |

## The map does not over-claim — that is the point

Four outcomes are `IMPLEMENTED` (UO-04 claim/support architecture, UO-05 filing
package + handoff, UO-06 docket/receipts/deadlines, UO-12 durable persistence with
executed recovery). The other eight are `PARTIAL` **with the missing part named**,
for example UO-09 "only the loopback local transport is live; the OpenAI/Anthropic/
xAI lanes report Unimplemented instead of returning fabricated text", and UO-01
"the vault itself is a device-local SQLite database, not an OS-encrypted
container, and the only KeyringStore is an in-process map".

A test enforces that discipline: an entry may claim `Implemented` only when it
names its production entry point and states no limitation, and may claim `Partial`
only when it names both the entry point and what is missing. A scope surface with
twelve ticks would be exactly the over-claim the clause exists to prevent.

## The boundaries are enforced, not just stated

`check_claim_text` is applied inside `build_commercialization_package`, which
produces text an inventor sends to third parties — the place where a promise does
real damage. The refusal names the boundary that was crossed:

```
commercialization package crosses a truth boundary: TB-4 would be crossed by the phrase "guaranteed revenue"
```

The guard is a **lexical backstop, not a semantics engine**, and saying so is part
of the evidence: it catches the assertive forms that appear in generated copy
("guaranteed patent", "% patentable", "risk-free return"), and a rephrased promise
can still slip past it. That is why the boundaries are also rendered in the UI and
asserted by acceptance tests rather than trusted to one function.

Matching is word-anchored on both sides. This repository has already been burned
twice by substring probes — a hardware probe that matched `encrypt` inside
`encrypted` and wrongly activated a domain pack, and a class probe that matched
the English word "observation" — so the honest-text cases are part of the test:
*"The vault stores invention content encrypted at rest in a local database"* and
*"Observation: the cited reference discloses a valve seat"* must NOT fire.

## Executed evidence

* `cargo test -p domain` → 22 passed, including `test_scope_map_is_complete_and_one_to_one`,
  `test_scope_map_does_not_over_claim`,
  `test_five_truth_boundaries_are_declared_and_enforced` and
  `test_boundary_guard_refuses_promises_and_permits_honest_text` (8 refusal
  cases, each asserting the boundary it crossed, plus 7 honest cases).
* `cargo test -p linchpin-desktop --lib` → 48 passed, including
  `test_scope_map_and_five_truth_boundaries_hold_at_the_product_boundary`, which
  exercises all five boundaries through the commands: TB-1 screening copy carries
  axes and states uncertainty, TB-2 lint findings assert no clearance, TB-3 a
  draft package is never submission-ready without a paid fee, TB-4 the exported
  payload passes the guard **and** a package whose target name promises an outcome
  is refused with `TB-4` named, TB-5 AI origin is stored separately from human
  conception.
* `apps/desktop/e2e/shell.spec.ts` → 20 passed, including the scope surface test.
* `scripts/artifact-e2e.sh` → 21 assertions PASS against the packaged executable:
  `get_scope_declaration` round-trips 5 boundaries and 12 capabilities, all five
  boundary ids are declared, no capability is over-claimed, and **the boundaries
  render in the Settings surface** — the only lane where the IPC bridge is real,
  so the only lane that can prove the declaration reaches a user.
* Mutation proofs, both CAUGHT and restored: removing the guard call from the
  export path (MUT-SCOPE-001-a) and making `check_claim_text` accept everything
  (MUT-SCOPE-001-b).

## Limits — stated, not implied

* The guard is lexical. A promise phrased outside the fourteen recorded forms is
  not caught by it; the acceptance tests and the rendered boundary statements are
  the compensating controls, not a proof of semantic safety.
* `SCOPE_MAP` is a static declaration. Nothing mechanically cross-checks a
  `Partial` entry's limitation against the code, so the limitation text is a
  reviewed claim, not a derived one.
* Eight of the twelve promised outcomes remain `PARTIAL`. This requirement is
  about declaring scope honestly, not about the eight being finished.
