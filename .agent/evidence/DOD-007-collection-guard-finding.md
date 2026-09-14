# DOD-007 Collection Guard Finding — green suite with zero tests

Measured 2026-09-10 at candidate `4f54de5`. Fixed in the same session.

## Finding (DOD-007 violation, DOD-024 masking)

DOD-007 RULE: *"The harness fails when zero tests or fewer than the expected
manifest are collected. BECAUSE: Many runners exit zero for empty,
misconfigured, or partially discovered suites."*
OR ELSE: *"The run is ERROR; no pass result from that suite is valid."*

All four `package.json` files invoked vitest as:

```
"test:unit": "vitest run --passWithNoTests"
```

Consequence, reproduced:

```
$ pnpm -r test:unit
packages/ui test:unit:        No test files found, exiting with code 0
packages/contracts test:unit: No test files found, exiting with code 0
apps/desktop test:unit:       No test files found, exiting with code 0
$ echo $?
0
```

The JavaScript lane reported **success while collecting zero tests**. Measured
test-file count under `apps/desktop/src`, `packages/ui/src` and
`packages/contracts/src` for `*.test.ts(x)` / `*.spec.ts(x)`:

```
test file count: 0
```

So the "pass" was produced entirely by the `--passWithNoTests` flag. Per DOD-007's
OR ELSE clause, no pass result from that suite was valid, and per DOD-024 this is
exactly failure-masking (`--passWithNoTests` is a stripped-down equivalent of
continue-on-error for an empty suite).

Separately, `scripts/test-collection-guard.sh` existed but was **not invoked by
any runner**, so the Rust suite had no expected-count guard either.

## Repair

1. Set `--passWithNoTests=false` in all four `test:unit` scripts
   (`package.json`, `apps/desktop`, `packages/ui`, `packages/contracts`).

   Verified before/after:

   | | before | after |
   | --- | --- | --- |
   | `pnpm -r test:unit` on empty JS suite | `exiting with code 0` | `exiting with code 1`, `ERR_PNPM_RECURSIVE_RUN_FIRST_FAIL`, exit 1 |

2. Added `scripts/test-collection-guard.py`, which runs the Rust suite, parses
   the runner's own per-binary counts, and fails when:
   - no parsable results are produced,
   - zero tests were collected,
   - the collected total is below the committed manifest,
   - any test failed, or
   - any test was ignored (DOD-006 requires a waiver for skipped tests).

3. Added the expected-count manifest
   `.agent/verification/state/TEST_COLLECTION_MANIFEST.json`:
   `expected_min_passed = 49`, `expected_test_binaries = 13`.

4. Wired the guard into `scripts/test-unit.sh` so it runs after `cargo test`.
   Note the guard is deliberately independent of `cargo test`'s exit code: the
   point of DOD-007 is to catch runners that exit **zero** on a bad collection.

## Verification

```
$ python3 scripts/test-collection-guard.py
collection guard: 25 result lines, 13 test binaries
  passed=49 failed=0 ignored=0 (manifest min=49)
collection guard: ok
exit 0
```

Mutation proof (DOD-018) — raising the manifest threshold to 500:

```
collection guard: FAIL -- collected 49 below manifest 500
exit 1
```

Restored to 49; guard green again. The guard therefore discriminates.

## Honest status

- **Rust lane**: guard wired and passing. 49 passed, 0 failed, 0 ignored,
  13 binaries, matching the manifest exactly.
- **JavaScript lane**: the masking flag is removed, so an empty suite now fails
  loudly. But the underlying fact is unchanged and must be stated plainly:
  **the JS packages contain zero test files**. DOD-006 cannot be PASS while
  required JS-side behavior is untested; this is recorded as FAIL, not waived.
  The desktop UI (`apps/desktop/src/App.tsx`, `main.tsx`) and the
  `packages/ui` / `packages/contracts` surfaces have no automated tests.
- **Python lane**: no `pyproject.toml` and no pytest suite exist, so
  `scripts/test-unit.sh` skips that lane entirely. Not a pass — an absent lane.

DOD-007 is recorded **PARTIAL** (guard now real and proven for the Rust lane;
JS lane empty and Python lane absent). DOD-006 is recorded **FAIL**.
