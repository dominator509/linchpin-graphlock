# DOD-037 — operator diagnostics: health, readiness, logs, traces, metrics, alerts

DOD-037 RULE: "Health, readiness, logs, metrics, traces, alerts, dashboards, and
correlation IDs truthfully describe critical workflows and induced failures
without leaking secrets."
REQUIRED EVIDENCE: "Known-failure injection mapped to signals, alert lifecycle,
trace/log correlation, and redaction evidence."

## What exists now

| Signal | Where | Truthfulness mechanism |
| --- | --- | --- |
| Health / readiness | `commands::get_diagnostics` | `storage_ok` is MEASURED by probing the vault directory (absent, read-only, unreadable, or writable); readiness is `DEGRADED` when the probe fails, never a constant |
| Logs | `commands::record_diagnostic_outcome` window | every recorded outcome carries command, outcome, error class, measured duration, timestamp and correlation id |
| Traces | `crash_reporter::LocalTelemetryPipeline` | a span is recorded per command execution; the view renders them as `<correlation>:<command>`, so the field is populated by real executions rather than being a placeholder |
| Metrics | `DiagnosticsMetrics` | recorded/succeeded/failed counts, failure rate, p50 and p95 duration over the window |
| Alerts | `DiagnosticsView::alerts` | derived from the same measurements: storage unavailable, ≥3 of the last 5 commands failed, or the most recent failure named with its command and correlation id |
| Correlation IDs | every `CommandResult` | already present; the diagnostics view is keyed by them |
| Redaction | `crash_reporter::RedactionPolicy` | applied to every recorded detail before it is stored |

Recording happens at the **IPC boundary** (`lib.rs`), which is where an operator's
latency and failures actually occur, for `record_conception`, `backup_vault`,
`restore_vault` and `run_local_inference`.

## Induced-failure mapping, executed

`test_diagnostics_report_induced_failure_with_correlation_and_redaction` injects
real conditions and asserts the signals:

| Injected condition | Observed signal |
| --- | --- |
| storage directory absent | `readiness = DEGRADED`, `storage_ok = false`, alert `storage unavailable: …` |
| `restore_vault` from a missing backup | a `FAILED` log entry with `error_class = POLICY`, the measured duration, `epoch:` timestamp, and the correlation id; the alert names the command and the correlation id; the same id appears in `traces` |
| credential in a message (`Authorization: Bearer sk-live-…`, `api_key=abcdef`) | not present in any recorded detail |

## Two defects found in the redaction path while proving this

1. **Registering key NAMES as secrets made redaction worse.** `with_secret("api_key")`
   replaced the *name* and left the value: `api_key=abcdef` became
   `[REDACTED]=abcdef`. Measured by this surface's own test, which is why the
   diagnostics redactor now registers nothing and the policy does the work.
2. **Short or opaque credentials were never redacted at all.** The policy covered
   registered literals and token shapes (`sk-`, `ghp_`, `32+` hex), so
   `password=hunter2`, `{"token":"abc123"}` and `Authorization: Basic-…` passed
   through in the clear — in the Repair Capsule path too, not only here. The
   policy now redacts the VALUE following a credential-shaped key, handling
   `key=value`, `key: value`, quoted JSON `"key":"value"` and `key=value;`, while
   leaving ordinary prose ("the token was refreshed") untouched. A third defect
   was found inside that fix: the quoted-key form was missed because the
   character after the key is `"`, not `:`.

`MUT-OPS-037-a` is the controlled defect for this path: removing the key-value
redaction from `apply` fails
`test_redaction_covers_key_values_not_only_token_shapes`, and the mutant is
restored with a green rerun.

## Why this is PARTIAL and not PASS

The clause also names **dashboards**. There is no dashboard surface: the signals
are reachable over IPC (`get_diagnostics`) and rendered nowhere, and no alert is
delivered outside the process. Claiming PASS would overstate what a user can
actually see, so the status is PARTIAL with the missing half named.

Two smaller limits, also stated rather than implied:

* The diagnostics window is **in-process and bounded to 200 outcomes**: it is a
  session view, not durable telemetry, and it is empty after a restart.
* `CommandError::safe_message` was documented as "never contains internal paths"
  while several messages name a LOCAL path. That comment was false and is
  corrected: the path is on the user's own device, it never leaves the device, and
  an operator told only "recovery failed" cannot recover. Secrets remain the thing
  that must never appear, and the redaction tests assert exactly that.
