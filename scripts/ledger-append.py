#!/usr/bin/env python3
"""Append a hash-chained event to the GraphLock execution ledger.

`.agent/LOOPS.md` requires a ledger event after each milestone, and
`scripts/validate-hash-ledger.py` (run by `scripts/harness-validate.sh`) rejects
the whole ledger if any event's `event_hash` does not match
`sha256(json(event, event_hash=""))` with sorted keys and no spaces, or if
`previous_event_hash` does not chain to the prior event.

An earlier revision of this repository appended the readable `.agent/state/
LEDGER.md` row by hand and left `.agent/state/LEDGER.jsonl` to a separate manual
step. Hand-computing a chain hash is exactly the kind of step that silently
produces an invalid ledger, so this script derives the chain from the file
itself and refuses to write if the existing ledger is already invalid.

Usage:
  python3 scripts/ledger-append.py --event DONE_VERIFIED --node EP-009 \
      --mode REMEDIATOR --command "cargo test -p storage" --exit-code 0 \
      --evidence .agent/evidence/REQ-REL-005-recovery.md \
      --reason "one-line justification"

The readable LEDGER.md row is appended at the same time, so the view cannot
drift from the chain.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import sys
from pathlib import Path

LEDGER = Path(".agent/state/LEDGER.jsonl")
VIEW = Path(".agent/state/LEDGER.md")

REQUIRED = [
    "event_id",
    "timestamp_utc",
    "agent_id",
    "mode",
    "node_id",
    "event",
    "previous_event_hash",
    "event_hash",
]


def event_hash(obj: dict) -> str:
    tmp = dict(obj)
    tmp["event_hash"] = ""
    canonical = json.dumps(tmp, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def read_events() -> list[dict]:
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text("utf-8").splitlines() if line.strip()]


def verify_chain(events: list[dict]) -> str:
    """Return the last event hash, refusing to extend an invalid chain."""
    previous = ""
    for number, obj in enumerate(events, start=1):
        missing = [k for k in REQUIRED if k not in obj]
        if missing:
            raise SystemExit(f"ledger line {number}: missing {', '.join(missing)}")
        if obj["previous_event_hash"] != previous:
            raise SystemExit(f"ledger line {number}: previous_event_hash mismatch")
        if event_hash(obj) != obj["event_hash"]:
            raise SystemExit(f"ledger line {number}: event_hash mismatch")
        previous = obj["event_hash"]
    return previous


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", required=True)
    parser.add_argument("--node", required=True)
    parser.add_argument("--mode", default="REMEDIATOR")
    parser.add_argument("--agent-id", default="linchpin-agent")
    parser.add_argument("--command", default="")
    parser.add_argument("--exit-code", type=int, default=0)
    parser.add_argument("--artifact-digest", default="NONE")
    parser.add_argument("--candidate-epoch", default="EP-009-remediation-001")
    parser.add_argument("--evidence", action="append", default=[])
    parser.add_argument("--reason", default="")
    parser.add_argument("--event-id", default="")
    args = parser.parse_args()

    events = read_events()
    previous = verify_chain(events)

    stamp = datetime.datetime.now(datetime.timezone.utc)
    event_id = args.event_id or f"{args.node.lower()}-{args.event.lower().replace('_', '-')}-{stamp:%Y%m%d%H%M%S}"
    record = {
        "event_id": event_id,
        "timestamp_utc": stamp.strftime("%Y-%m-%dT%H:%M:%S.%fZ"),
        "agent_id": args.agent_id,
        "mode": args.mode,
        "node_id": args.node,
        "event": args.event,
        "candidate_epoch": args.candidate_epoch,
        "commit_sha": "HEAD",
        "artifact_digest": args.artifact_digest,
        "command": args.command,
        "exit_code": args.exit_code,
        "evidence_paths": args.evidence,
        "evidence_sha256": [],
        "previous_event_hash": previous,
        "event_hash": "",
    }
    record["event_hash"] = event_hash(record)

    with LEDGER.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(record) + "\n")

    # Verify what was actually written, not what was intended.
    verify_chain(read_events())

    row = (
        f"| {stamp:%Y-%m-%d} | {args.node} | {args.event} | {event_id} | "
        f"{', '.join(args.evidence) or 'none'} | {args.reason} |\n"
    )
    with VIEW.open("a", encoding="utf-8") as fh:
        fh.write(row)

    print(f"ledger: appended {event_id} ({args.event}) hash={record['event_hash'][:16]}...")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
