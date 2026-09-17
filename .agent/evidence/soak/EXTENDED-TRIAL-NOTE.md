# Extended soak trial (3604 s) — provenance and limits

The gate runs the soak at 600 s and the harness writes ONE report per run, so this
snapshot preserves the extended trial's own output (`report-extended-3600s.json`,
`SUMMARY-extended-3600s.md`) before the next unit-lane run overwrites it. Nothing here
is re-measured or edited: the numbers are read from the report that run wrote.

- Duration: **3604s** of a requested 3600s, label
  `ABBREVIATED`. The clause's specified scale is 24/48/72+ hours and is NOT met.
- Cycles: **45350** writes through the production command; errors: **0**.
- Vault: 25444352 bytes for 45350 events
  (561 bytes/event, bound 65 536).
- Working set: 11026432 -> 22929408 bytes
  (net 11902976), sampled at 58 heartbeats ~60s apart,
  ranging 7753728 to 22929408 bytes with no monotonic climb.

WHAT THIS IS NOT, stated because the clause's own terms invite over-reading: this is an
ABBREVIATED trial. It is not the pack's flatline invariant and it measures no
P99-creep-per-day slope; both require the 24/48/72+ hour scale on dedicated
infrastructure, which is why DOD-038 is DEFERRED_LONG_RUNNING. The vault curve rises
because it is storing events, so vault growth here is expected; the leak signal is the
working set, which oscillates rather than climbing.

ONE FIELD IS STALE BY CONSTRUCTION: this run executed before the specification_gap
correction, so its `specification_gap` and `not_claimed` fields still carry the old
wording that no soak scale is specified anywhere and that DOD-038 "remains PARTIAL".
That claim was measured to be false -- `.agent/verification/E2E_SUITE_LIBRARY.md`
specifies 24/48/72+ hours for E2E-018 -- and is corrected in the harness, in the gate's
600 s report.json, and in the DOD-038 disposition. The measurements above are unaffected.
