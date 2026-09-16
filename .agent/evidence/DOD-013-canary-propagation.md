# DOD-013 — runtime-generated canary data in the black-box proofs

DOD-013 RULE: "Runtime-generated unpredictable canary data is used for critical
black-box proofs."
BECAUSE: "Static examples can be hard-coded, cached, or accidentally satisfied by
canned responses."
REQUIRED EVIDENCE: "Seed/source, generated canary, propagation trace, and final
independent observation."

## Why this document exists

The recorded disposition for DOD-013 read **FAIL** with the reason "No
runtime-generated canary data is used in any black-box proof. Test data is static
literal strings." That was accurate when written and has since become a **stale
claim**: later rounds introduced runtime canaries in three separate black-box
lanes. A FAIL disposition that measurement disproves is a correctness defect of
its own kind, so this round re-measured the clause lane by lane instead of
repeating the earlier sentence.

One real gap was found while re-measuring, and fixed: the exact-artifact lane
wrote the static literal `"e2e artifact probe"` and only read the product's own
success response back. It now generates a canary at runtime and looks for it in
the durable vault file from OUTSIDE the product.

## Lane-by-lane measurement

### 1. Exact-artifact E2E — `apps/desktop/e2e-artifact.mjs` (was the gap; now closed)

| Requirement | Measurement |
| --- | --- |
| Seed/source | `ARTIFACT-CANARY-<base36 time>-<random>` generated in the harness at run time |
| Propagation | carried as the `content` of `record_conception`, invoked through the packaged executable's real IPC bridge (`window.__TAURI_INTERNALS__.invoke`) |
| Independent observation | the harness asks `get_configuration` for the vault path and searches `<vault_file>` and `<vault_file>-wal` directly with `readFileSync`, outside the app |
| Measured result | `PASS: canary found in the durable vault by an independent channel (DOD-012, DOD-013) -- ARTIFACT-CANARY-… present in C:\Users\domin\AppData\Local\LINCHPIN\linchpin-vault.db` |

A canned response or a fabricated `persisted: true` cannot satisfy this: the value
did not exist before the run, and the check does not go through the product.

### 2. Local-provider live-fire — `apps/desktop/e2e-local-provider.mjs`

| Requirement | Measurement |
| --- | --- |
| Seed/source | `CANARY-<base36 time>-<random>` per call |
| Propagation | embedded in the inference prompt sent to the loopback provider through the product's own `run_local_inference` |
| Independent observation | (a) the returned completion is searched across the whole tracked tree with `git grep -F` and must NOT already exist there, so a hard-coded or cached response fails; (b) the provider is asked directly through its own `/api/tags` that it really serves the named model |
| Measured result | `PASS: completion text is not a string that already exists in the repository -- not found in tracked files`, plus the provider's independent model check |

### 3. Recovery drill — `apps/desktop/src-tauri/tests/recovery_drill.rs`

| Requirement | Measurement |
| --- | --- |
| Seed/source | `canary-<iteration>-<nanos>-<pid>` per iteration, in every one of the 25 seeded events |
| Propagation | written through the production command `record_conception`, then a fault is injected (total loss or in-place corruption) and recovery runs through `restore_vault` |
| Independent observation | the recovered vault is reopened through a SECOND connection and the canary-bearing events are counted |
| Measured result | 25 of 25 canary events recovered in every cycle, exact digest reconciliation, recorded in `.agent/evidence/recovery-drill/report.json` |

### 4. Installer preservation — `scripts/vault-preservation-e2e.sh`

| Requirement | Measurement |
| --- | --- |
| Seed/source | `LINCHPIN-CANARY-<epoch>-<pid>` per run |
| Propagation | written into a real SQLite database at the product's real vault path plus a content-addressed blob, before install/uninstall/reinstall |
| Independent observation | the canary row and blob are read back after each step, byte-intact |
| Measured result | recorded in the lane's report; the gate fails with "vault database is GONE" if the state is destroyed |

## What is NOT covered, stated rather than implied

* The **browser lane** (`apps/desktop/e2e/shell.spec.ts`) uses no canary. It runs
  against the bundle without IPC and asserts rendering and honesty of the
  disconnected state; it is not a state-changing proof, so there is no effect for
  a canary to trace.
* The canary in the artifact lane proves the value reached **durable storage**;
  it does not prove a specific SQL row shape, which is covered separately by
  `storage`'s own tests.
* Canaries are unpredictable but not cryptographically random. They defeat
  hard-coding and caching, which is what the clause asks for; they are not a
  security primitive.

## Consequence for the clause

DOD-013 moves from **FAIL** to **PASS**: three of the four critical black-box
lanes already carried runtime canaries with independent observation, and the
fourth — the exact-artifact lane — was the actual gap and is now closed with a
canary that is read back outside the product.
