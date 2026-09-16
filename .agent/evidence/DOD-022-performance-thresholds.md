# DOD-022 — encoded performance thresholds against a defined workload

DOD-022 RULE: "Performance, resource, cost, and SLO requirements are encoded as
automated pass/fail thresholds against a defined workload and environment."
REQUIRED EVIDENCE: "Workload model, environment, samples, percentiles, resource
metrics, thresholds, and verdict."
OR ELSE: "The nonfunctional claim is UNVERIFIED and a mandatory SLO failure is
NO_GO."

## Why this exists

The recorded disposition read **FAIL** because no threshold existed anywhere in
the repository. The only timing numbers that existed were *observations* inside
the recovery drill's report, and an observation is not a gate: nothing failed if
they got ten times worse. Two encoded bounds now exist, both enforced.

## 1. Vault write/read workload — `apps/desktop/src-tauri/tests/performance_gate.rs`

| Required evidence | Measurement |
| --- | --- |
| Workload model | 200 conception events written **one at a time** through the production command `record_conception`, each carrying a runtime canary, into a fresh file-backed vault; then every event read back through a **second connection** |
| Environment | recorded in the report: `os=windows`, `arch=x86_64`, profile `debug` (test profile), single machine, single process, local disk |
| Samples and percentiles | see below |
| Resource metrics | vault size after the workload |
| Thresholds | p50 ≤ 50 ms, p95 ≤ 150 ms, max ≤ 500 ms, total writes ≤ 30 000 ms, total read ≤ 2 000 ms, vault ≤ 32 MiB — each one FAILS the gate |
| Verdict | `PASS`, written to `.agent/evidence/performance/report.json` |

Measured on the current candidate:

```
performance: 200 writes p50 12.03ms p95 16.88ms max 60.52ms total 2509.56ms,
             read 5.52ms, vault 245760 bytes, profile debug
```

The gate also asserts the read observed **every** write (`200`), so a fast but
lossy path cannot pass by dropping work.

## 2. Exact-artifact startup bound — `apps/desktop/e2e-artifact.mjs`

Measured from process spawn to the point where the packaged executable's page
reports itself complete, against an encoded bound of **45 000 ms**. Measured on
the current candidate: **1 012 ms**, recorded as its own assertion in
`.agent/evidence/artifact-e2e/STATUS.md`. The bound is deliberately generous
because this lane runs on a loaded developer host; it exists to fail a change
that makes the app take minutes to become usable.

## What these are NOT, stated rather than implied

* The measured profile is **debug** (the `test` cargo profile). These are
  **regression bounds on this host**, not published service levels. They fail a
  change that makes the vault an order of magnitude slower; they do not promise a
  latency to a customer.
* Single machine, single process, local disk, **no concurrency**: there is no
  capacity or multi-user claim here.
* No **cost** metric is encoded, because the product has no metered resource —
  it runs entirely on the user's device. That absence is a property of the
  architecture, not an unmeasured requirement.
* No soak or endurance duration is claimed under this clause; that belongs to
  DOD-038, which remains PARTIAL.
