# Anti-Gaming Scan — Production Path Findings (EP-009 / release blocker class)

Scan command: `python3 scripts/anti-gaming-scan.py .`
Scan date: 2026-09-10
Candidate: `41e0b20` (base `962e365`)

The tree-wide scan returns 371 matches, most of which are false positives in
prose/spec documents (`AGENTS.md` DoD text legitimately uses words like
"simulate", "placeholder", "bypass" to *forbid* them). This file classifies the
matches that occur in **production code paths**. Per AGENTS.md §12 and DOD-019,
unexplained production hits are release blockers or the affected claim must be
marked incomplete.

## Finding AG-001 — `provider_transport` returns fabricated model output

`crates/provider_transport/src/lib.rs`

| Line | Code | Problem |
| --- | --- | --- |
| 47-50 | `text: format!("Local generation for: {}", request.prompt)`, `metadata: "simulated-local-inference"` | `LocalModelAdapter::generate` performs **no inference**. It echoes the prompt and self-labels the result simulated. |
| 102-109 | `// Without making actual network calls, simulate terms-gated response` … `text: format!("OpenAI mock generation for: {}", …)` | `OpenAiAdapter::generate` makes **no network call** and returns a literal `"OpenAI mock generation"` string. |
| 132, 74 | assertions `contains("Analyze patent")` / `metadata == "simulated-local-inference"` | The tests **assert the mock text**, so they lock in the fabricated behavior and would pass forever. |

**Why this matters (DOD-019, DOD-020, DOD-010, §9 Reality law).** The crate's
own identifiers claim production capability (`openai-official`, `local-llama3-proof`,
`LocalModelAdapter`, `OpenAiAdapter`), but neither adapter reaches a real
dependency boundary. `OpenAiAdapter::new` stores an `api_key_ref` and only
checks `is_empty()` — no keyring read, no HTTP, no auth. This is precisely the
"appears to work is a failure state" pattern. Any claim that LINCHPIN performs
provider inference is currently **UNVERIFIED / SIMULATED**.

**Reachability (measured).**
`grep provider_transport apps/desktop/src-tauri/src/*.rs` → **no matches**. The
desktop entry point (`apps/desktop/src-tauri/src/lib.rs`) registers exactly one
command, `get_system_health`. The crate is a workspace member and compiles, but
**is not wired into any user-facing path**.

This is a mitigating fact that must be stated precisely: because the adapters are
unreachable from the desktop boundary, they cannot yet mislead a user at
runtime. It does **not** clear the finding — the crate exists as a production
workspace member and the capability claim is unproven. Disposition:
**INCOMPLETE / SIMULATED**, not PASS.

## Finding AG-002 — `crash_reporter` redaction is a no-op assignment

`crates/crash_reporter/src/lib.rs:113-116`

```rust
if !incident.redacted {
    // Force redaction simulation
    incident.redacted = true;
}
```

Setting a boolean does **not** redact anything. `is_safe_for_export()` returns
that same flag, so the safety gate is self-certifying. A capsule holding
unredacted invention content would report `is_safe_for_export() == true`.
This is a security-relevant gap under SECURITY.md (invention content is
Confidential/device-only by default). Disposition: **INCOMPLETE** — the flag is
asserted, not earned. Marked for EP-006 remediation.

## Finding AG-003 — `evidence` "fuzz target" exercises nothing

`crates/evidence/src/lib.rs:137-144`

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

Discards every result and asserts only "did not panic" over four hand-written
strings. It is named a fuzz target but performs no mutation, no corpus, no
generation. It provides **no coverage credit** for any fuzz claim and must not
be cited as evidence of fuzzing (DOD-038). Disposition: **INCOMPLETE** — rename
or replace with a real fuzzing harness; the claim must not be counted.

## Not findings (classified, cleared)

| Match | Location | Disposition |
| --- | --- | --- |
| `vitest` | `apps/desktop/package.json:11` | Legitimate test-runner script (`TEST_ONLY_BRANCH` pattern is over-broad). Not a production branch. |
| `simulate`/`placeholder`/`bypass`/`stub` | `AGENTS.md`, `.agent/**`, `*.md` specs | Control-plane prose that *prohibits* these patterns. Cleared. |
| `SKIP_DIRS` bypass in the scanner itself | `scripts/anti-gaming-scan.py` | Scanner implementation detail. Cleared. |

## Required action before any GO verdict

1. Either implement real provider transports against a declared boundary
   (DOD-010 real/sandbox execution), or mark `REQ-*` provider claims
   INCOMPLETE and remove them from any production-ready statement.
2. Make `crash_reporter` redaction actually transform content, with a negative
   test proving unredacted input cannot report `is_safe_for_export() == true`.
3. Replace or rename the `evidence` fuzz simulation.
4. Re-run this scan and confirm zero *unclassified* production hits
   (DOD-019 REQUIRED EVIDENCE: lexical scan, reachability trace, allowlist
   decisions, findings).
