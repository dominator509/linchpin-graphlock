#!/usr/bin/env python3
"""Measure unit-test coverage with the toolchain's own LLVM tools (DOD-008).

DOD-008 REQUIRED EVIDENCE includes "coverage, and mutation sensitivity". The
mutation half is covered by scripts/mutation-proof.py; this gate produces the
coverage half using the `llvm-tools-preview` component that ships WITH the
compiler, so no repository dependency is added.

WHAT IT MEASURES, AND WHAT IT DOES NOT
  * Instrumented line/function/region coverage for the LIBRARY crates, whose unit
    tests carry the semantic assertions.
  * The desktop application crate is EXCLUDED by name. It is mostly command glue
    and its behaviour is proven at the artifact boundary by
    apps/desktop/e2e-artifact.mjs, which measures the packaged executable rather
    than the source. The exclusion is stated in the report, not implied.
  * Coverage says a line RAN. It does not say the assertion was meaningful --
    which is why the mutation proofs exist, and why the thresholds below are
    regression floors rather than quality targets.

TWO MEASURED HARNESS DEFECTS THIS GATE EXISTS TO AVOID
  1. The host LLVM installation is a different patch version from the LLVM rustc
     links, and `llvm-cov` from it produced a table of 0.00% rows from valid
     profiles. The toolchain's own tools are used instead, and a 0% TOTAL is
     treated as a harness failure rather than recorded as a measurement of zero.
  2. Profiles written by `cargo test` did not line up with the test binaries when
     the report was built from a glob of the target directory. The binaries are
     therefore RUN EXPLICITLY here, one process at a time, so each profile and its
     object come from the same execution.

Usage:
  python3 scripts/coverage-gate.py            # build, run, measure, enforce floors
  python3 scripts/coverage-gate.py --check    # fail if the epoch has moved
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPORT_DIR = Path(".agent/evidence/coverage")
REPORT = REPORT_DIR / "report.json"
SUMMARY = REPORT_DIR / "SUMMARY.md"
EPOCH = Path(".agent/verification/state/EPOCH.json")
PROFILE_DIR = Path("target/coverage-profiles")
TARGET_DIR = Path("target/coverage")

EXCLUDED_CRATES = {
    "linchpin-desktop": "command glue proven at the artifact boundary (apps/desktop/e2e-artifact.mjs)",
}

# Regression floors, not quality targets: the measured value must not fall below
# these. Set from the first measurement, then raised only with evidence.
LINE_FLOOR_PERCENT = 55.0
FUNCTION_FLOOR_PERCENT = 55.0

CRATES = [
    "application",
    "commercialization",
    "crash_reporter",
    "domain",
    "evidence",
    "mcp_hub",
    "patent",
    "platform_windows",
    "provider_transport",
    "research",
    "storage",
]


def run(argv: list[str], env: dict | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        argv,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        shell=False,
    )


def epoch_digest() -> str | None:
    if not EPOCH.exists():
        return None
    try:
        return json.loads(EPOCH.read_text(encoding="utf-8")).get("epoch_digest")
    except json.JSONDecodeError:
        return None


def check() -> int:
    if not REPORT.exists():
        print(f"coverage: FAIL -- no report at {REPORT}", file=sys.stderr)
        return 1
    recorded = json.loads(REPORT.read_text(encoding="utf-8"))
    current = epoch_digest()
    if recorded.get("epoch_digest") != current:
        print(
            "coverage: FAIL -- the report was measured at epoch "
            f"{str(recorded.get('epoch_digest'))[:16]} but the current epoch is "
            f"{str(current)[:16]}; a source change invalidates the measurement. "
            "Re-measure with scripts/coverage-gate.py",
            file=sys.stderr,
        )
        return 1
    totals = recorded.get("totals", {})
    if (
        totals.get("line_percent", 0) < LINE_FLOOR_PERCENT
        or totals.get("function_percent", 0) < FUNCTION_FLOOR_PERCENT
    ):
        print(
            f"coverage: FAIL -- line {totals.get('line_percent')}% / function "
            f"{totals.get('function_percent')}% below the enforced floors "
            f"({LINE_FLOOR_PERCENT}% / {FUNCTION_FLOOR_PERCENT}%)",
            file=sys.stderr,
        )
        return 1
    print(
        f"coverage: ok (line {totals.get('line_percent')}%, function "
        f"{totals.get('function_percent')}%, epoch {str(current)[:16]})"
    )
    return 0


def toolchain_llvm_tools() -> Path | None:
    sysroot = run(["rustc", "--print", "sysroot"]).stdout.strip()
    if not sysroot:
        return None
    candidate = Path(sysroot) / "lib" / "rustlib" / "x86_64-pc-windows-msvc" / "bin"
    return candidate if (candidate / "llvm-cov.exe").exists() else None


def percent(token: str) -> float:
    return float(token.rstrip("%"))


def measure() -> int:
    tools_dir = toolchain_llvm_tools()
    if tools_dir is None:
        print(
            "coverage: ERROR -- the toolchain's llvm-tools component is absent, so no "
            "llvm-cov matching this compiler is available. Install it with "
            "`rustup component add llvm-tools-preview`; this is an environment "
            "prerequisite, not a product result.",
            file=sys.stderr,
        )
        return 2
    llvm_profdata = str(tools_dir / "llvm-profdata.exe")
    llvm_cov = str(tools_dir / "llvm-cov.exe")

    if PROFILE_DIR.exists():
        shutil.rmtree(PROFILE_DIR, ignore_errors=True)
    PROFILE_DIR.mkdir(parents=True, exist_ok=True)

    env = dict(os.environ)
    env["RUSTFLAGS"] = "-C instrument-coverage"
    env["LLVM_PROFILE_FILE"] = str((PROFILE_DIR / "%p-%m.profraw").resolve())
    env["CARGO_TARGET_DIR"] = str(TARGET_DIR)

    packages: list[str] = []
    for crate in CRATES:
        packages += ["-p", crate]

    print(f"coverage: building instrumented test binaries for {len(CRATES)} crates")
    build = run(["cargo", "test", "--no-run", "--locked", *packages], env=env)
    if build.returncode != 0:
        tail = "\n".join(((build.stdout or "") + (build.stderr or "")).splitlines()[-15:])
        print(f"coverage: FAIL -- instrumented build failed:\n{tail}", file=sys.stderr)
        return 1

    combined = (build.stdout or "") + (build.stderr or "")
    executables = {
        str(Path(match).resolve())
        for match in re.findall(r"Executable .*?\((.+?\.exe)\)", combined)
    }
    binaries = sorted(
        path
        for path in executables
        if any(Path(path).name.startswith(f"{crate}-") for crate in CRATES)
    )
    if not binaries:
        print(
            "coverage: FAIL -- cargo reported no test executables, so nothing was measured",
            file=sys.stderr,
        )
        return 1

    print(f"coverage: running {len(binaries)} test binaries directly to collect profiles")
    failures: list[str] = []
    for binary in binaries:
        result = run([binary, "--test-threads=1"], env=env)
        if result.returncode != 0:
            failures.append(f"{Path(binary).name} (exit {result.returncode})")
    if failures:
        print(
            "coverage: FAIL -- a test binary failed, so a coverage number from this run "
            f"would describe broken code: {', '.join(failures)}",
            file=sys.stderr,
        )
        return 1

    profiles = sorted(PROFILE_DIR.glob("*.profraw"))
    if not profiles:
        print(
            "coverage: FAIL -- no profile data was produced, so a coverage number would "
            "be fabricated",
            file=sys.stderr,
        )
        return 1

    merged = (PROFILE_DIR / "merged.profdata").resolve()
    merge = run([llvm_profdata, "merge", "-sparse", *map(str, profiles), "-o", str(merged)])
    if merge.returncode != 0:
        print(f"coverage: FAIL -- llvm-profdata failed: {merge.stderr[:300]}", file=sys.stderr)
        return 1

    objects: list[str] = []
    for binary in binaries:
        objects += ["-object", binary]

    report = run(
        [
            llvm_cov,
            "report",
            *objects,
            f"-instr-profile={merged}",
            # Both separators: measured, dependency sources appear as
            # `...\.cargo\registry\...` and a `/`-only pattern missed them all.
            r"-ignore-filename-regex=(tests[\\/]|target[\\/]|[\\/]\.cargo[\\/]|rustc[\\/])",
        ]
    )
    combined = (report.stdout or "") + (report.stderr or "")
    if report.returncode != 0:
        print(f"coverage: FAIL -- llvm-cov failed: {combined[:300]}", file=sys.stderr)
        return 1
    if "could not read profile data" in combined:
        print(
            "coverage: FAIL -- llvm-cov could not read the profile; refusing to report 0% "
            f"as a measurement: {combined.strip()[:200]}",
            file=sys.stderr,
        )
        return 1

    # Filename | Regions miss/cover/cover% | Functions ... | Lines ... | Branches ...
    per_crate: dict[str, dict[str, float]] = {}
    total_line = total_function = total_region = None
    for line in (report.stdout or "").splitlines():
        fields = line.split()
        if len(fields) < 13:
            continue
        if fields[0] == "TOTAL":
            total_region = percent(fields[3])
            total_function = percent(fields[6])
            total_line = percent(fields[9])
            continue
        # llvm-cov prints paths relative to the workspace, so a crate file appears
        # as `crates/<name>/src/...` OR as `<name>/src/...` depending on how the
        # unit was compiled. Measured: matching only the first form produced an
        # empty per-crate table beside a non-zero TOTAL, which is a report that
        # disagrees with itself.
        match = re.search(r"(?:crates[\\/])?([a-z_]+)[\\/]src[\\/]", fields[0])
        if not match:
            continue
        entry = per_crate.setdefault(
            match.group(1), {"lines": 0.0, "functions": 0.0, "files": 0}
        )
        entry["lines"] += percent(fields[9])
        entry["functions"] += percent(fields[6])
        entry["files"] += 1

    if total_line is None:
        print("coverage: FAIL -- llvm-cov produced no TOTAL row", file=sys.stderr)
        return 1
    if total_line == 0.0 and total_function == 0.0:
        print(
            "coverage: FAIL -- llvm-cov reported 0% for everything, which means the "
            "profiles and the objects did not line up; that is a harness failure, not a "
            "coverage result",
            file=sys.stderr,
        )
        return 1

    for entry in per_crate.values():
        if entry["files"]:
            entry["lines"] = round(entry["lines"] / entry["files"], 2)
            entry["functions"] = round(entry["functions"] / entry["files"], 2)

    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    payload = {
        "gate": "coverage",
        "covers": ["DOD-008"],
        "harness": "scripts/coverage-gate.py",
        "epoch_digest": epoch_digest(),
        "crates_measured": CRATES,
        "crates_excluded": EXCLUDED_CRATES,
        "instrumentation": {
            "rustflags": "-C instrument-coverage",
            "tools": {
                "llvm-profdata": llvm_profdata,
                "llvm-cov": llvm_cov,
                "note": "the tools shipped WITH the compiler; the host LLVM installation is a different patch version and produced 0% from valid profiles",
            },
            "test_binaries_run": len(binaries),
            "profiles_merged": len(profiles),
        },
        "totals": {
            "line_percent": total_line,
            "function_percent": total_function,
            "region_percent": total_region,
        },
        "per_crate": per_crate,
        "thresholds": {
            "line_floor_percent": LINE_FLOOR_PERCENT,
            "function_floor_percent": FUNCTION_FLOOR_PERCENT,
            "note": "a floor is a regression bound on this host, not a quality target",
        },
        "limits": [
            "line coverage shows that code RAN; the mutation proofs are what show an assertion discriminates",
            "the desktop application crate is excluded and is covered at the artifact boundary instead",
            "measured on one host with one toolchain; the number is not portable between toolchains",
        ],
    }
    REPORT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    rows = "\n".join(
        f"| {name} | {entry['files']} | {entry['lines']}% | {entry['functions']}% |"
        for name, entry in sorted(per_crate.items())
    )
    SUMMARY.write_text(
        "# DOD-008 coverage measurement\n\n"
        "Generated by `scripts/coverage-gate.py`. Do not hand-edit.\n\n"
        f"- Epoch: `{payload['epoch_digest']}`\n"
        f"- Test binaries run: {len(binaries)} (all passed); profiles merged: {len(profiles)}\n"
        f"- TOTAL: line **{total_line}%**, function **{total_function}%**, region {total_region}%\n"
        f"- Floors enforced: line >= {LINE_FLOOR_PERCENT}%, function >= {FUNCTION_FLOOR_PERCENT}%\n"
        f"- Excluded: {', '.join(f'{k} ({v})' for k, v in EXCLUDED_CRATES.items())}\n\n"
        "| Crate | Files | Line % | Function % |\n| --- | --- | --- | --- |\n"
        f"{rows}\n\n"
        "Line coverage shows that code RAN. It does not show that the assertions\n"
        "discriminate, which is what `scripts/mutation-proof.py` demonstrates; the two\n"
        "are reported together rather than one standing in for the other.\n",
        encoding="utf-8",
    )
    print(f"coverage: wrote {REPORT} and {SUMMARY}")
    print(f"  line {total_line}% function {total_function}% region {total_region}%")

    if total_line < LINE_FLOOR_PERCENT or total_function < FUNCTION_FLOOR_PERCENT:
        print(
            f"coverage: FAIL -- below the enforced floors (line {LINE_FLOOR_PERCENT}%, "
            f"function {FUNCTION_FLOOR_PERCENT}%)",
            file=sys.stderr,
        )
        return 1
    return 0


def main() -> int:
    return check() if "--check" in sys.argv else measure()


if __name__ == "__main__":
    raise SystemExit(main())
