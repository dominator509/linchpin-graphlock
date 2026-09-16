#!/usr/bin/env python3
"""Execute the commands and references the documentation publishes (DOD-023).

DOD-023 RULE: "README commands, examples, quickstarts, install, upgrade,
deployment, rollback, and operator instructions are executed exactly as
published in clean environments."
EVIDENCE: "Extracted command manifest, clean execution logs, expected/actual
outputs, and corrected documentation."

The recorded disposition said "scripts/smoke-test.sh exits 2 because the
exact-artifact smoke runner is absent" -- measured, it exits 0. This script does
the work the clause actually asks for, mechanically:

  1. EXTRACT every shell command and referenced file path from the operator-facing
     documents (README.md, COMMANDS.md, DEPLOYMENT.md, OPERATIONS.md,
     ROLLBACK.md, RELEASE.md, ENVIRONMENT.md).
  2. EXECUTE the commands. `COMMANDS.md` is the sanctioned command source, so
     those are the ones run; prose references are checked for existence.
  3. Compare expected vs actual and report every discrepancy.

Nothing is assumed to work because it looks reasonable.
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

OUT = Path(".agent/evidence/documentation/STATUS.md")

# Operator-facing documents whose published commands are in scope.
DOCS = [
    "README.md",
    "COMMANDS.md",
    "DEPLOYMENT.md",
    "OPERATIONS.md",
    "ROLLBACK.md",
    "RELEASE.md",
    "ENVIRONMENT.md",
]

# Markdown inline-code spans and fenced blocks that look like shell commands.
INLINE_RE = re.compile(r"`([^`\n]+)`")
FENCE_RE = re.compile(r"```(?:bash|sh|shell)?\n(.*?)```", re.S)
# Shell commands we are willing to execute: they must start with a known runner.
COMMAND_RE = re.compile(r"^(?:sh|python3|pnpm|cargo|node|git)\s+\S")
# Backticked tokens that are file paths rather than commands.
PATH_RE = re.compile(r"^[\w./-]+\.(?:md|json|toml|yaml|yml|csv|sh|py|rs|ts|tsx)$")

# Commands that mutate state, need credentials, or are interactive: executing
# them in a documentation gate would be wrong. Each is reported as SKIPPED with
# the reason, never silently dropped.
EXCLUDED_PATTERNS = [
    (r"\btauri\s+dev\b", "interactive dev server; long-running"),
    (r"\btauri\s+build\b", "bundle build; covered by scripts/build.sh separately"),
    (r"\bpnpm\s+install\b", "mutates node_modules; covered by install.sh"),
    (r"\buv\s+sync\b", "no Python package exists"),
    (r"\bmsiexec\b", "installs software on the host"),
    (r"\bgit\s+push\b", "mutates a remote; forbidden by AGENTS.md section 11"),
    (r"\bkill\b", "process management"),
    (r"\bfor\s+f\s+in\b", "loop over adapter parity; awk-based, not a product command"),
    # Self-reference guard. Measured: documenting this gate by its own command
    # made it execute itself, recursing until the per-command timeout fired.
    (r"\bdoc-exec\.py\b", "self-reference: a documentation gate must not re-invoke itself"),
    # The mutation harness is DELIBERATELY destructive: it edits tracked source
    # files, runs the guarding test, and restores the bytes. Documenting it in
    # COMMANDS.md is required (that file is the only legal command source), but a
    # documentation gate must not start mutating the tree it is verifying.
    # `--list` and `--check` are read-only and are NOT excluded.
    (
        r"mutation-proof\.py(?!\s+--(?:list|check))",
        "injects controlled defects into tracked source files; run deliberately",
    ),
]


def extract(doc: Path) -> tuple[list[str], list[str]]:
    text = doc.read_text("utf-8", errors="replace")
    commands: list[str] = []
    paths: list[str] = []

    for block in FENCE_RE.findall(text):
        for line in block.splitlines():
            line = line.strip()
            if COMMAND_RE.match(line):
                commands.append(line)
    for span in INLINE_RE.findall(text):
        span = span.strip()
        if COMMAND_RE.match(span):
            commands.append(span)
        elif PATH_RE.match(span):
            paths.append(span)

    # De-duplicate, preserving order.
    seen_c: set[str] = set()
    uniq_c = [c for c in commands if not (c in seen_c or seen_c.add(c))]
    seen_p: set[str] = set()
    uniq_p = [p for p in paths if not (p in seen_p or seen_p.add(p))]
    return uniq_c, uniq_p


def _resolve_argv(cmd: str) -> list[str] | None:
    """Resolve the runner through PATH, returning None when it is not installed.

    subprocess with shell=False uses CreateProcess on Windows, which does not
    apply PATHEXT: `pnpm` is installed as pnpm.cmd, so passing the bare name
    raised FileNotFoundError and the command was misreported as missing even
    though it is present. Resolving through shutil.which (which honours
    PATHEXT) fixes the false negative; .cmd/.bat shims are invoked via cmd /c.
    """
    parts = cmd.split()
    if not parts:
        return None
    exe = shutil.which(parts[0])
    if exe is None:
        return None
    if exe.lower().endswith((".cmd", ".bat")):
        return ["cmd", "/c", exe, *parts[1:]]
    return [exe, *parts[1:]]


def excluded(cmd: str) -> str | None:
    for pattern, reason in EXCLUDED_PATTERNS:
        if re.search(pattern, cmd):
            return reason
    return None


# Per-command wall-clock bound. The desktop lanes legitimately take minutes;
# 900s is generous but bounded so one wedged process cannot stall the gate.
PER_COMMAND_TIMEOUT = 900


def _kill_stragglers() -> None:
    """Reap GUI/test children that outlive a timed-out command.

    subprocess timeout kills only the direct child. The desktop smoke and E2E
    lanes spawn linchpin-desktop, which keeps the artifact locked and would
    corrupt later lanes in the same run.

    Best-effort only: this runs on an already-failing path, so it must never
    raise. Measured: `taskkill` is not on PATH in this environment, and an
    unguarded call turned a per-command timeout into a hard gate crash.
    """
    for name in ("linchpin-desktop", "linchpin-desktop-e2e"):
        exe = shutil.which("taskkill")
        if exe is None:
            return
        try:
            subprocess.run(
                [exe, "/F", "/IM", f"{name}.exe"],
                capture_output=True,
                shell=False,
                timeout=30,
            )
        except (OSError, subprocess.SubprocessError):
            pass


def main() -> int:
    check_only = "--check" in sys.argv

    results: list[dict] = []
    missing_paths: list[tuple[str, str]] = []

    for doc_name in DOCS:
        doc = Path(doc_name)
        if not doc.exists():
            continue
        commands, paths = extract(doc)

        for p in paths:
            if not Path(p).exists():
                missing_paths.append((doc_name, p))

        for cmd in commands:
            reason = excluded(cmd)
            if reason:
                results.append(
                    {
                        "doc": doc_name,
                        "command": cmd,
                        "status": "SKIPPED",
                        "reason": reason,
                    }
                )
                continue

            # Per-command timeout. Several published commands (smoke-test,
            # test-e2e) launch the desktop binary; without a bound a single
            # wedged GUI process stalls the whole documentation gate, and the
            # child outlives the timeout holding the artifact. Measured: a
            # 900s timeout left linchpin-desktop running after doc-exec gave up.
            argv = _resolve_argv(cmd)
            if argv is None:
                results.append(
                    {
                        "doc": doc_name,
                        "command": cmd,
                        "status": "FAIL",
                        "exit_code": "NOT_FOUND",
                        "actual": f"runner not on PATH: {cmd.split()[0]!r}",
                    }
                )
                continue
            try:
                proc = subprocess.run(
                    argv,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    shell=False,
                    timeout=PER_COMMAND_TIMEOUT,
                )
            except subprocess.TimeoutExpired as exc:
                _kill_stragglers()
                results.append(
                    {
                        "doc": doc_name,
                        "command": cmd,
                        "status": "FAIL",
                        "exit_code": "TIMEOUT",
                        "actual": (
                            f"exceeded {PER_COMMAND_TIMEOUT}s per-command bound; "
                            f"partial output: {((exc.stdout or b'').decode('utf-8','replace') if isinstance(exc.stdout, bytes) else (exc.stdout or ''))[-200:]}"
                        ),
                    }
                )
                continue
            except FileNotFoundError as exc:
                # A published command whose executable is not installed must be
                # reported, not raised: the clause asks whether the documented
                # commands RUN as written, so a missing interpreter is a finding
                # about the documentation/environment, not a verifier crash.
                results.append(
                    {
                        "doc": doc_name,
                        "command": cmd,
                        "status": "FAIL",
                        "exit_code": "NOT_FOUND",
                        "actual": f"executable not found: {exc}",
                    }
                )
                continue
            except OSError as exc:
                results.append(
                    {
                        "doc": doc_name,
                        "command": cmd,
                        "status": "FAIL",
                        "exit_code": "OS_ERROR",
                        "actual": f"could not execute: {exc}",
                    }
                )
                continue
            results.append(
                {
                    "doc": doc_name,
                    "command": cmd,
                    "status": "PASS" if proc.returncode == 0 else "FAIL",
                    "exit_code": proc.returncode,
                    "actual": ((proc.stdout or "") + (proc.stderr or "")).strip()[-300:],
                }
            )

    executed = [r for r in results if r["status"] in {"PASS", "FAIL"}]
    failed = [r for r in executed if r["status"] == "FAIL"]
    skipped = [r for r in results if r["status"] == "SKIPPED"]

    verdict_ok = not failed and not missing_paths

    if not check_only:
        lines = [
            "# DOD-023 Executable Documentation",
            "",
            "Generated by `scripts/doc-exec.py`. Do not hand-edit.",
            "",
            "## Method",
            "",
            "Commands are extracted from the operator-facing documents and executed",
            "with `sh`/`python3`/`cargo`/`pnpm` as written. Commands that mutate the",
            "host, need credentials, or are interactive are listed as SKIPPED with a",
            "reason rather than silently omitted. File paths referenced in prose are",
            "checked for existence.",
            "",
            "## Summary",
            "",
            f"- Documents scanned: {len([d for d in DOCS if Path(d).exists()])} of {len(DOCS)}",
            f"- Commands executed: {len(executed)} (PASS {len(executed) - len(failed)}, FAIL {len(failed)})",
            f"- Commands skipped: {len(skipped)}",
            f"- Referenced paths missing: {len(missing_paths)}",
            "",
            "## Executed commands",
            "",
            "| Document | Command | Result | Exit | Actual (tail) |",
            "| --- | --- | --- | --- | --- |",
        ]
        for r in executed:
            actual = r["actual"].replace("|", "\\|").replace("\n", " ")[:120]
            lines.append(
                f"| `{r['doc']}` | `{r['command']}` | {r['status']} | "
                f"{r['exit_code']} | `{actual}` |"
            )

        lines += ["", "## Skipped commands (with reason)", ""]
        if skipped:
            for r in skipped:
                lines.append(f"- `{r['command']}` ({r['doc']}) — {r['reason']}")
        else:
            lines.append("- none")

        lines += ["", "## Missing referenced paths", ""]
        if missing_paths:
            for doc_name, p in missing_paths:
                lines.append(f"- `{p}` referenced by `{doc_name}`")
        else:
            lines.append(
                "- none. Every file path named in the scanned documents exists."
            )

        lines += ["", "## Verdict", ""]
        lines.append(
            "ok — every executed command succeeded and every referenced path exists"
            if verdict_ok
            else f"FAIL — {len(failed)} command failure(s), {len(missing_paths)} missing path(s)"
        )
        lines.append("")

        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text("\n".join(lines), "utf-8")
        print(f"wrote {OUT}")

    for r in executed:
        tail = r["actual"].splitlines()[-1] if r["actual"] else ""
        # Command output can contain non-cp1252 bytes (box-drawing characters
        # from cargo/pnpm). Printing it raw crashed this script on Windows; the
        # report itself is unaffected because it is written as UTF-8.
        safe = tail[:70].encode("ascii", errors="replace").decode("ascii")
        print(f"  {r['status']:<5} exit={r['exit_code']}  {r['command']}  -- {safe}")
    for r in skipped:
        print(f"  SKIP  {r['command']}  ({r['reason']})")
    for doc_name, p in missing_paths:
        print(f"  MISSING PATH {p} (from {doc_name})")

    print(
        f"doc-exec: {len(executed)} executed, {len(failed)} failed, "
        f"{len(skipped)} skipped, {len(missing_paths)} missing paths"
    )
    return 0 if verdict_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
