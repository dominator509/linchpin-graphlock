# Anti-Gaming Scan — Production Path Findings (EP-009 / release blocker class)

Scan command: `python3 scripts/anti-gaming-scan.py .`
Scan date: 2026-09-10
Candidate: `264913f` (base `962e365`)

**STATUS: AG-001 through AG-006 have all been REPAIRED and verified.**
Each carries a mutation/negative proof showing the guarding test genuinely
fails when the real behavior is removed (DOD-018). Re-run
`python3 scripts/anti-gaming-scan.py .` to confirm the production hits are gone.

Six defects were found, not three. AG-004, AG-005 and AG-006 were located by
writing probe tests against existing APIs rather than by reading code — the
AG-004 probe failed on its first run, which is what exposed it. That method is
worth reusing: executable probes surface fabrication faster than review does.

Severity note: AG-006 is the most serious, because it manufactured the
**release-gating** live-fire evidence that REQ-SHIP-001 depends on. AG-004 is
next, because it returned a wrong application number on a filing-evidence path
(REQ-PAT-005). AG-001/AG-002 fabricated provider output and defeated an export
safety guard.

The tree-wide scan returned 371 matches at base revision, most of which are
false positives in prose/spec documents (`AGENTS.md` DoD text legitimately uses
words like "simulate", "placeholder", "bypass" to *forbid* them). This file
classifies the matches that occur in **production code paths**. Per AGENTS.md
§12 and DOD-019, unexplained production hits are release blockers or the
affected claim must be marked incomplete.

## Finding AG-001 — `provider_transport` returned fabricated model output — REPAIRED

`crates/provider_transport/src/lib.rs` (base revision)

| Line | Code | Problem |
| --- | --- | --- |
| 47-50 | `text: format!("Local generation for: {}", request.prompt)`, `metadata: "simulated-local-inference"` | `LocalModelAdapter::generate` performed **no inference**. It echoed the prompt and self-labelled the result simulated. |
| 102-109 | `// Without making actual network calls, simulate terms-gated response` … `text: format!("OpenAI mock generation for: {}", …)` | `OpenAiAdapter::generate` made **no network call** and returned a literal `"OpenAI mock generation"` string. |
| 132, 74 | assertions `contains("Analyze patent")` / `metadata == "simulated-local-inference"` | The tests **asserted the mock text**, locking in the fabricated behavior so it would pass forever. |

**Resolution (commit `bcc54a0`).** The fake success paths were removed.
`PROVIDER_TRANSPORT_MATRIX.md` requires "Local | llama.cpp / Ollama | local
process/HTTP on loopback", so:

- `LocalModelAdapter::new` validates a loopback-only endpoint and **refuses**
  non-loopback hosts, so prompts cannot leave the device (SECURITY.md).
- `generate` performs a real `reqwest` POST to `/completion` (llama.cpp) or
  `/api/generate` (Ollama) and extracts the real generated text.
- `TransportError` distinguishes `InvalidRequest` / `Unreachable` /
  `ProviderFailure` / `InvalidResponse` / `Unimplemented` (DOD-014).
- The fabricated OpenAI path was replaced by `UnimplementedTransport`, which
  **always** returns `Unimplemented` and reports `is_live() == false`.
- `ProviderTransport` gained `is_live()` so callers cannot assume success.

**Negative proof.** `test_unreachable_endpoint_fails_closed_not_fabricated`
hits the real network boundary at `127.0.0.1:1` and asserts a genuine
`Unreachable` error. `test_unimplemented_transport_never_succeeds` proves the
unwired lane cannot return success. `provider_transport` went 2 → 6 tests.

Dependencies: `reqwest 0.13.5` / `serde_json 1.0.151` were already in
`Cargo.lock`; the diff adds them to this crate only and introduces **zero new
packages**. Remaining honesty caveat: the crate is still unreachable from the
desktop boundary, so no end-to-end provider claim is made.

## Finding AG-002 — `crash_reporter` redaction was a no-op assignment — REPAIRED

`crates/crash_reporter/src/lib.rs:113-116` (base revision)

```rust
if !incident.redacted {
    // Force redaction simulation
    incident.redacted = true;
}
```

Setting a boolean redacts nothing, and `is_safe_for_export()` returned that same
self-certifying flag, so a capsule holding unredacted invention content would
report `is_safe_for_export() == true`.

**Resolution (commit `1c4341b`).** Flag-setting was replaced with real content
transformation: `RedactionPolicy::apply` rewrites registered secrets to
`[REDACTED]` and also scrubs token-shaped substrings (`sk-`, `ghp_`, `gho_`,
32+ hex). `RepairCapsule::new(incident, brief, &policy)` redacts the detail,
clears the raw field on the retained incident, and `is_safe_for_export()` now
requires both `redacted` and an empty raw detail. `redacted_detail()` is the
only detail egress.

**Mutation proof.** Replacing `policy.apply(..)` with a pass-through clone makes
`test_redaction_actually_removes_invention_content` fail with
`raw invention content leaked into redacted detail` (exit 101). Restored,
green. `crash_reporter` went 2 → 7 tests.

## Finding AG-003 — `evidence` "fuzz target" exercised nothing — REPAIRED

`crates/evidence/src/lib.rs:137-144` (base revision)

```rust
// A mock fuzz target (would normally be in `fuzz/` dir with `cargo fuzz`)
#[test]
fn test_fuzz_target_simulation() {
    let fuzz_inputs = vec!["..", "/", "a/b/c", "a/../b"];
    for input in fuzz_inputs {
        let _ = InputHardener::sanitize_path(input); // Should not panic
    }
}
```

It discarded every return value and asserted only "did not panic" over four
hand-written strings — no generation, no corpus, no oracle, so it could never
fail and provided no coverage credit for any fuzzing claim (DOD-038).

**Resolution (commit `9255b8d`).** Replaced with
`test_sanitize_path_property_over_generated_corpus`: a deterministic xorshift64
PRNG (no new dependency, AGENTS.md §10) generates 20,000 paths from a
traversal-heavy alphabet and asserts the real invariant — every path containing
`..` or rooted at `/` is rejected, every other path accepted. It also asserts
both branches were exercised >1000 times so a degenerate generator cannot make
the test vacuous. `test_archive_entry_rejects_zip_slip` was added.

**Mutation proof.** Removing the `path.contains("..")` guard fails the corpus
test with `permissive acceptance of escaping path: "..%2efile.txt"`
(exit 101) — the generated corpus found a real bypass class. Restored, green.

## Finding AG-004 — `patent` receipt import returned hardcoded values — REPAIRED

`crates/patent/src/lib.rs::ReceiptImport::import` (base revision)

```rust
pub fn import(ack_file_content: &str) -> Result<Self, &'static str> {
    // Mock parsing logic
    if ack_file_content.contains("AppNumber:") && ack_file_content.contains("ConfNumber:") {
        Ok(ReceiptImport {
            application_number: "12/345,678".to_string(),
            confirmation_number: "9876".to_string(),
        })
    } else {
        Err("Invalid receipt format")
    }
}
```

It checked only that the input *contained* two labels, then returned hardcoded
literals. Its own test asserted those literals, so the fabrication was locked
in and **every** receipt imported successfully with the wrong application number.

**Proven by probe before the fix** (DOD-019 worked example):

```
ReceiptImport::import("AppNumber: 99/888,777\nConfNumber: 1234")
  -> application_number == "12/345,678"
  assertion `left == right` failed: import ignored the supplied application number
  left: "12/345,678"  right: "99/888,777"
  [exit 101]
```

**Resolution (commit `587cc0c`).** Real line-anchored field extraction with
fail-closed validation. Missing or empty labels, non-numeric confirmation
numbers and malformed application numbers all return `Err`. A realistic
multi-line USPTO acknowledgement receipt round-trips correctly. Tests: 4 added
including 6 negative cases.

In a filing-evidence path a wrong application number is a record-integrity
failure (REQ-PAT-005), so this is a correctness defect as well as an
anti-gaming one.

## Finding AG-005 — `patent` package builder labelled strings as DOCX/PDF — REPAIRED

`crates/patent/src/lib.rs::PackageBuilder` (base revision)

```rust
pub fn build_docx(claims: &str, spec: &str) -> Result<PatentPackage, &'static str> {
    // Mock deterministic DOCX package
    Ok(PatentPackage {
        content: format!("DOCX: Claims: {} | Spec: {}", claims, spec),
        format: "DOCX".to_string(),
    })
}
```

Neither builder produced a DOCX or a PDF. Both produced a Rust `String` with the
format name prefixed and a `format` field claiming the document type, so a
caller could not distinguish a real package from that string. REQ-PAT-002
requires real package artifacts plus a manifest SHA-256.

**Resolution (commit `587cc0c`).** The builders now emit an explicit,
self-describing `MANIFEST` whose `format` is `"MANIFEST"` — never `"DOCX"` or
`"PDF"` — and whose content carries a real dependency-free FNV-1a digest of both
inputs. The digest function is named `input_digest` and documented as a content
fingerprint, **not** a cryptographic signature, so it cannot be mistaken for
one. Tests assert the format is not DOCX/PDF, that the digest is deterministic,
and that it changes when either input changes.

Real OOXML/PDF rendering remains **INCOMPLETE** against REQ-PAT-002. That is
recorded rather than implied: the honest move was to stop claiming a format the
crate cannot produce, not to build a half-renderer.

## Finding AG-006 — `commercialization` fabricated live-fire evidence — REPAIRED

`crates/commercialization/src/lib.rs` (base revision)

```rust
pub fn run_uo_live_fire(&mut self) -> Result<(), &'static str> {
    // Simulating UO-01..12 live fire
    self.completed_runs = 12;
    Ok(())
}
```

plus its test:

```rust
assert!(orchestrator.run_uo_live_fire().is_ok());
assert_eq!(orchestrator.completed_runs, 12);
```

`completed_runs` was set to 12 without executing anything, and the test asserted
that value, so the fabrication was self-certifying. This is the **most serious
defect found this session**: REQ-SHIP-001 states *"Release requires UO-01..12
live-fire"*, so any release gate reading this counter would have been told
live-fire passed when no run occurred. That is manufactured release evidence.

**Resolution (commit `264913f`).** The counter is now computed from recorded
runs. `record_run(outcome_id, passed, detail)` appends a `UoRun`;
`completed_runs()` counts **distinct passing** outcomes; `missing_outcomes()`
lists what is absent; `verify_domain_regression()` fails while anything is
missing. There is no code path that reaches 12 without 12 real passing records.
Failing runs are retained and do not count; duplicates count once.

Also repaired in the same crate (AG-006b): `DataRoom::export_pitch_deck`
returned the constant `"Pitch deck generated"`, discarding the room's targets,
so every export was byte-identical and carried no information. It now renders
the non-confidential target list, labelled as draft planning material with no
valuation claim (REQ-COM-004, REQ-UI-003).

Tests: `commercialization` 2 → 5, including
`test_completed_runs_cannot_be_fabricated`,
`test_failing_runs_do_not_count_as_completed` and
`test_duplicate_runs_count_once`.

## Not findings (classified, cleared)

| Match | Location | Disposition |
| --- | --- | --- |
| `vitest` | `apps/desktop/package.json:11` | Legitimate test-runner script (`TEST_ONLY_BRANCH` pattern is over-broad). Not a production branch. |
| `simulate`/`placeholder`/`bypass`/`stub` | `AGENTS.md`, `.agent/**`, `*.md` specs | Control-plane prose that *prohibits* these patterns. Cleared. |
| `SKIP_DIRS` bypass in the scanner itself | `scripts/anti-gaming-scan.py` | Scanner implementation detail. Cleared. |

### Residual production-path hits after repair (all cleared as prose)

Post-repair the scan reports 7 hits under `crates/`. Each was read and
classified; none is executable fabricated behavior:

| Location | Line content | Class |
| --- | --- | --- |
| `provider_transport/src/lib.rs:17` | `//! * [UnimplementedTransport] -- an explicit, non-succeeding placeholder` | doc comment |
| `provider_transport/src/lib.rs:45` | `/// accurately, never degrade into simulated success.` | doc comment |
| `provider_transport/src/lib.rs:70` | `TransportError::Unimplemented(m) => write!(f, "not implemented: {m}")` | error message emitted **because** the lane is unimplemented |
| `provider_transport/src/lib.rs:221` | `/// Explicit placeholder for provider lanes that are not yet wired...` | doc comment |
| `evidence/src/lib.rs:158` | `/// ...replaces the previous test_fuzz_target_simulation, which...` | doc comment |
| `crash_reporter/src/lib.rs:146` | `/// Replace every registered secret with a fixed placeholder and drop` | doc comment |
| `crash_reporter/src/lib.rs:292` | `// Simulate the old behavior: flip the flag, leave content intact.` | comment inside the DOD-018 negative proof |

The `Unimplemented` error string is the mechanism by which an unwired lane fails
honestly; removing it would restore the fabricated-success defect. The remaining
six are comments. Zero unexplained *executable* production hits remain.

## Still required before any GO verdict

1. `provider_transport` must be **wired** to a user-facing path and proven
   against a live loopback llama.cpp/Ollama server (DOD-010 real dependency
   execution, PF-011). Until then the provider capability stays INCOMPLETE.
2. Provider claims must not appear as production-ready in UI/docs until (1).
3. Re-run this scan and confirm zero *unclassified* production hits
   (DOD-019 REQUIRED EVIDENCE: lexical scan, reachability trace, allowlist
   decisions, findings).
