# Mutation-fuzz campaign — GEN-027, DOD-038's fuzz term

`GEN-027` was recorded in the applicability matrix as *"no mutation-based fuzzing
engine is configured"*. That absence is now replaced by an executed campaign, and
the campaign found a real defect in shipping code.

## What runs

`apps/desktop/src-tauri/tests/fuzz_campaign.rs` — deterministic, seeded, and
dependency-free (the repository has no fuzzing framework, and adding one would
require a nightly toolchain to fuzz functions that are pure over `&str`):

1. **Deterministic adversarial phase.** Every awkward character inserted at every
   character position of every seed string — 4 634 inputs per target, run against
   all ten targets before any randomness. The character set includes the
   case-folding characters that break byte-indexed code (`İ` U+0130, `K` U+212A,
   `ß`), combining marks, a NUL, a BOM, separators and an emoji.
2. **Random mutation phase.** Seeded xorshift64\*, with `byte-flip`,
   `awkward-char-insert(-charwise)`, `delete`, `duplicate`, `splice-corpus`,
   `awkward-char-append/prepend(-charwise)` and `lowercase`. A panic is a
   **finding**: the input is recorded and then shrunk deterministically until it
   stops reproducing, so the report names a small reproducer.

Targets (all reachable from IPC with user-supplied text): `check_claim_text`,
`RedactionPolicy::apply`, `InputHardener::sanitize_path`,
`InputHardener::sanitize_archive_entry`, `LogRedactor::redact_with_report`,
`PatentLinter::lint_claims`, `ReceiptImport::import`, `PackageBuilder::build_docx`,
`RuleSetAuthority::new`, and the chain-of-title date validator. Evidence:
`.agent/evidence/fuzz/report.json`. **Label: ABBREVIATED** — no fuzzing duration,
corpus size or iteration count is specified in the repository, so the run does not
claim DOD-038 at full scale.

## The defect it found

```
RedactionPolicy::apply("İapi_key=secretvalue")
  -> panicked: start byte index 2 is not a char boundary; it is inside '\u{307}'
```

The credential search ran over `input.to_lowercase()` and then indexed the
**original** string with the offsets it found. Lowercasing is not
length-preserving: `İ` is 2 bytes and lowercases to `i̇` (3 bytes), `K` is 3 bytes
and lowercases to `k` (1 byte). Every offset after such a character shifted, and
the slice that followed panicked — **in the credential-redaction path**, on a log
line a Turkish keyboard produces routinely, so the incident reporter could abort
exactly when it was needed. The same pattern existed in `domain::scope::
check_claim_text` (the truth-boundary guard), where the shift was a silent
mis-detection risk rather than a panic.

**Fixed:** both now search the original text with ASCII case folding and
char-boundary checks (`find_ascii_case_insensitive`, and the rewritten
`contains_phrase`), so every offset is a property of the string being indexed.
Regression tests cover `İapi_key=…`, `İ token=…`, `ß password=…`, a combining-mark
form, and the negative case that a key inside a longer identifier
(`Kapi_key=…`, ASCII `K`) must NOT be treated as a credential assignment.

## How the campaign itself was wrong, twice — and how that was caught

A campaign that reports zero findings is not evidence of absence, so the campaign
was itself mutation-tested: `MUT-FUZZ-001` reintroduces the lowercasing-offset bug
and requires the campaign to fail. It came back **SURVIVED** — twice:

1. **Corpus dilution.** The pool reached its 512-item cap within a few hundred
   iterations, leaving ~2 % of picks on the twelve seed strings that actually
   contain the key names. Fixed by seeding half the iterations from `SEED_CORPUS`
   and adding char-wise insertion operators.
2. **The fixture scrubbed the trigger.** The redaction target registered `"İ"` as
   a *registered secret*, and `apply` replaces registered secrets **before** the
   key-value pass runs — so the very character whose case folding shifts offsets
   was deleted from every input. 4 634 adversarial inputs per target reported
   clean while a direct probe of the same code panicked. Fixed by registering only
   an ASCII secret.

After both fixes `MUT-FUZZ-001` is **CAUGHT**, so the campaign demonstrably
detects the class of defect it exists for. The clean run reports **0 findings**
over 20 000 random iterations plus 4 634 adversarial inputs per target, across ten
targets.

## Limits — stated, not implied

* **Pure functions only.** No network, filesystem, IPC transport, Tauri command
  dispatch, or concurrent interleaving is fuzzed.
* **Not coverage-guided** (GEN-025 remains an accurate absence) and **no grammar
  model** of the input languages (GEN-026 remains accurate).
* **Abbreviated**: 24 634 inputs per target is a bounded trial, not a long
  campaign; the seed is fixed by default so runs are reproducible rather than
  varied.
* A clean run says nothing about inputs the corpus cannot reach; the mutation
  proof above is what makes its silence meaningful for the class it covers.
