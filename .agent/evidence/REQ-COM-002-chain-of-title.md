# REQ-COM-002 — chain-of-title timeline and asset readiness

Status: **IMPLEMENTED AND EXECUTED** at the domain and product-command boundary.
**No UI surface yet** — stated here rather than implied (see Limits).

SPEC-009 REQ-COM-002: "Asset readiness builds a chain-of-title timeline from
inventor/owner records, assignments, public Assignment Search evidence,
liens/security-interest warnings when available, filing/grant/legal-status
events, maintenance/deadline state, remaining-life assumptions, related
families/continuations, know-how dependencies and unresolved
ownership/inventorship questions. USPTO recordation is represented as
recordation evidence, never as legal validation."

Before this round the behaviour did not exist at all, so no acceptance test
could pass. The unbound-requirement audit recorded it as the last implementable
gap in DOD-001.

## What exists now

| Layer | Artifact | What it does |
| --- | --- | --- |
| Domain | `crates/commercialization/src/asset_readiness.rs` | `TitleRecordKind` (9 kinds), `TitleRecord`, `TitleFinding` (5 kinds), `RemainingLife`, `AssetReadiness`, `assess_asset_readiness` |
| Command | `commands.rs` — `build_asset_readiness` | The product boundary, reachable over IPC, with typed input validation |
| IPC | `lib.rs` — `build_asset_readiness` | Registered Tauri command |
| Tests | 5 domain tests + 1 product-boundary test + `MUT-COM-002-a` | Unit, boundary and mutation evidence |

## Three decisions that follow from "never as legal validation"

Each is the opposite of what an "asset readiness score" normally does, and each
is asserted:

1. **A gap in the chain is reported, never bridged.** If an owner record follows
   an inventor record with no assignment or recordation covering the transition,
   the transition is a `GAP` finding. Assuming continuity because two records are
   adjacent is exactly the laundering the clause forbids. Two adjacent ownership
   changes with no assignment produce **two** gaps, and the asset is not ready.
2. **Recordation never sets readiness.** A recordation record is emitted as
   `RECORDATION_IS_EVIDENCE_ONLY` with its reel/frame, and
   `recordation_is_evidence_not_validation` is always `true` in the output — so a
   consumer that ignores `findings` still cannot read a recordation as a legal
   conclusion.
3. **A lien is never cleared by this code.** An encumbrance is surfaced as
   `LIEN_WARNING` and blocks readiness, because only the lienholder can release
   it.

Two further honesty rules: a caller-supplied unresolved ownership/inventorship
question blocks readiness rather than being answered, and absent remaining life is
carried as an open question instead of being invented ("remaining life was not
supplied; no term assumption is asserted here").

Inputs are refused with a named cause rather than defaulted: an unknown record
kind, a blank `source` ("a chain without evidence is not a chain"), a
non-`YYYY-MM-DD` effective date, two same-kind records sharing a date (the order
of events would be ambiguous), a blank asset label, and a negative remaining life.

## Executed evidence

* `cargo test -p commercialization` → 15 passed (5 new), covering chain
  continuity and ordering, gap-not-assumption, recordation-is-evidence, lien
  blocking, and refusal of invalid records.
* `cargo test -p linchpin-desktop --lib` → 49 passed, including
  `test_asset_readiness_reports_gaps_and_never_treats_recordation_as_validation`,
  which exercises the same rules through the production command and asserts the
  refusal messages name the requirement ("ISO").
* `MUT-COM-002-a` in `scripts/mutation-proof.py` → **CAUGHT**: making the
  inventor-to-first-owner transition bridged without evidence fails the
  acceptance test, then the mutant is restored and the suite reruns green.

## Limits — stated, not implied

* **No UI surface.** The command is reachable over IPC and is covered by the
  product-boundary test, but no Settings/Commercialize panel renders the timeline
  yet, so a user cannot see it without an IPC client. That is a real gap in the
  user-facing half of the outcome (UO-08 remains `PARTIAL` in `SCOPE_MAP`).
* **No Assignment Search ingestion.** Records are supplied by the caller; the
  product does not query USPTO Assignment Search, so "public Assignment Search
  evidence" is represented as a record with a `source`, not as a live lookup.
* **The date check is shape-and-range, not a calendar.** `2025-02-31` is accepted
  by the validator; no date crate was added for it, and pretending otherwise
  would be a bigger claim than the code earns.
* **Remaining life is an input, not a computation.** The product records the
  assumption supplied and does not compute a term, because term depends on filing
  dates, priority claims, extensions and maintenance state that this module does
  not interpret.
* **Nothing here is legal advice.** The verdict is conservative: it says whether
  the *records supplied* form a continuous chain, never whether title is valid.
