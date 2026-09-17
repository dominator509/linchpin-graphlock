#!/usr/bin/env python3
"""DOD-035 -- cross-version upgrade, downgrade and rollback matrix.

WHY THIS EXISTS. DOD-035 RULE: "Every claimed upgrade, downgrade, rollback,
version-skew, mixed-fleet, and compatibility path is executed with realistic
persistent state." Its OR ELSE is "Unsupported or failed claimed paths block
release or must be explicitly removed from support claims."

Measured state before this script: `scripts/vault-preservation-e2e.sh` installed
the SAME v0.1.0 MSI three times (install, "update", "rollback"), so every step
exercised one version. The disposition therefore read FAIL with the honest note
that same-version reinstall is weaker evidence than a cross-version upgrade. This
script closes that gap by building a SECOND version and running a real matrix
against it.

WHAT IT DOES
  1. PROVISION (only when an artifact is missing, or --provision is passed):
     builds v0.1.0 and v0.2.0 MSIs by bumping the three version manifests, running
     the production build, staging the MSI + executable under
     target/version-matrix/<version>/, and RESTORING the manifests and lockfiles
     byte-for-byte. The restore is asserted with `git diff --exit-code`, so the
     repository can never be left at the bumped version.
  2. MATRIX, against realistic persistent state placed at the product's real
     app-data path (a vault-shaped SQLite database with a per-run canary token plus
     a content-addressed blob):
       install v0.1.0 -> assert version and installed-binary digest
       seed canary    -> assert readable
       UPGRADE to v0.2.0 over it   -> assert version, binary digest, canary digest
       DOWNGRADE to v0.1.0 over it -> record the measured outcome (Windows
           Installer may refuse an in-place downgrade); assert that the canary is
           intact either way and that the installed version agrees with the
           outcome
       UNINSTALL      -> assert the user's data survived removal
       ROLLBACK install v0.1.0     -> assert version, binary digest, canary digest
  3. EVIDENCE: writes .agent/evidence/version-matrix/report.json and STATUS.md with
     every step's measured exit code, version, digests and verdict. Exits non-zero
     if any assertion fails, so it can be a gate lane.

WHAT IT DOES NOT CLAIM
  * mixed-fleet / version-skew across DIFFERENT machines: this is a single-user
    desktop product with no server fleet and no shared state, and the product
    claims no such support. The clause's applicability for that half is recorded
    in the evidence, not silently dropped.
  * the installed executable is driven through its UI here; that is the
    exact-artifact lane's job (DOD-004). This lane asserts identity and state.
"""
from __future__ import annotations

import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

ROOT = pathlib.Path(".").resolve()
STAGE = ROOT / "target/version-matrix"
EVIDENCE = ROOT / ".agent/evidence/version-matrix"
MANIFESTS = [
    "apps/desktop/package.json",
    "apps/desktop/src-tauri/Cargo.toml",
    "apps/desktop/src-tauri/tauri.conf.json",
]
LOCKFILES = ["Cargo.lock", "pnpm-lock.yaml"]
PRODUCT = os.environ.get("LINCHPIN_PRODUCT_NAME", "LINCHPIN")
OLD = "0.1.0"
NEW = "0.2.0"


def run(argv: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, errors="replace", **kwargs)


def powershell(script: str) -> subprocess.CompletedProcess:
    return run(["powershell", "-NoProfile", "-Command", script])


def sha256_file(path: pathlib.Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def appdata_root() -> pathlib.Path:
    base = os.environ.get("LOCALAPPDATA") or str(pathlib.Path.home() / "AppData/Local")
    return pathlib.Path(base) / PRODUCT


def installed_state() -> dict:
    """The installed product's version, location and executable digest (or absent)."""
    script = (
        "$paths = @("
        "'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*',"
        "'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*',"
        "'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*');"
        f"$e = Get-ItemProperty -Path $paths -ErrorAction SilentlyContinue | Where-Object {{ $_.DisplayName -eq '{PRODUCT}' }} | Select-Object -First 1;"
        "if (-not $e) { '{\"installed\": false}' | Write-Output; exit 0 };"
        "$loc = $e.InstallLocation; if (-not $loc) { $loc = '' };"
        "$exe = Join-Path $loc 'linchpin-desktop.exe';"
        "$hash = ''; if (Test-Path $exe) { $hash = (Get-FileHash -Algorithm SHA256 $exe).Hash.ToLower() };"
        "@{ installed = $true; display_version = $e.DisplayVersion; install_location = $loc; exe = $exe; exe_sha256 = $hash } | ConvertTo-Json -Compress"
    )
    proc = powershell(script)
    text = (proc.stdout or "").strip().splitlines()
    payload = text[-1] if text else "{}"
    try:
        state = json.loads(payload)
    except json.JSONDecodeError:
        raise SystemExit(f"could not read installed state: {proc.stdout!r} {proc.stderr!r}")
    if not state.get("installed"):
        return {"installed": False}
    location = state.get("install_location") or ""
    if not location:
        return state
    # MSI InstallLocation for this product is the app folder; the exe may sit in a
    # versioned subfolder, so look one level down as well.
    exe = pathlib.Path(location) / "linchpin-desktop.exe"
    if not exe.exists():
        candidates = sorted(pathlib.Path(location).glob("**/linchpin-desktop.exe"))
        if candidates:
            exe = candidates[0]
    if exe.exists():
        state["exe"] = str(exe)
        state["exe_sha256"] = sha256_file(exe)
        state["exe_bytes"] = exe.stat().st_size
    return state


def msiexec(msi: pathlib.Path, action: str) -> int:
    script = (
        f"$p = Start-Process msiexec.exe -ArgumentList '/{action}','\"{msi}\"','/qn','/norestart' "
        "-Wait -PassThru; exit $p.ExitCode"
    )
    return powershell(script).returncode


def uninstall_any() -> int:
    """Remove any installed copy of the product, whatever MSI it came from."""
    script = (
        "$paths = @("
        "'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*',"
        "'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*',"
        "'HKCU:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*');"
        f"$e = Get-ItemProperty -Path $paths -ErrorAction SilentlyContinue | Where-Object {{ $_.DisplayName -eq '{PRODUCT}' }} | Select-Object -First 1;"
        "if (-not $e) { exit 0 };"
        "$p = Start-Process msiexec.exe -ArgumentList '/x',$e.PSChildName,'/qn','/norestart' -Wait -PassThru; exit $p.ExitCode"
    )
    return powershell(script).returncode


def write_canary(db: pathlib.Path, blob: pathlib.Path, token: str) -> None:
    db.parent.mkdir(parents=True, exist_ok=True)
    # The blob lives one level below the app-data root (the product's vault
    # directory), and this harness's first run failed with FileNotFoundError on it
    # because only the root was created.
    blob.parent.mkdir(parents=True, exist_ok=True)
    import sqlite3

    con = sqlite3.connect(db)
    con.execute("CREATE TABLE IF NOT EXISTS canary (id INTEGER PRIMARY KEY, token TEXT NOT NULL)")
    con.execute("INSERT INTO canary (token) VALUES (?)", (token,))
    con.commit()
    con.close()
    blob.write_text(token, encoding="utf-8")


def canary_state(db: pathlib.Path, blob: pathlib.Path, token: str) -> dict:
    import sqlite3

    state = {"db_exists": db.exists(), "blob_exists": blob.exists(), "token_present": False}
    if state["db_exists"]:
        con = sqlite3.connect(db)
        try:
            rows = [r[0] for r in con.execute("SELECT token FROM canary").fetchall()]
        finally:
            con.close()
        state["rows"] = rows
        state["token_present"] = token in rows
    if state["blob_exists"]:
        state["blob_matches"] = blob.read_text(encoding="utf-8") == token
    state["intact"] = bool(state["db_exists"] and state["token_present"] and state.get("blob_matches"))
    return state


def launch_check(exe: str, seconds: int = 6) -> dict:
    """Start the INSTALLED binary and prove it stays alive, then stop it.

    A digest match proves the installer put the right bytes on disk; it does not
    prove those bytes RUN. The upgraded install is launched from its installed
    location and observed for a few seconds, so a binary that cannot start (missing
    resource, broken side-by-side dependency) fails the matrix instead of passing on
    identity alone.
    """
    if not exe:
        return {"launched": False, "why": "no installed executable path"}
    script = (
        f"$p = Start-Process -FilePath '{exe}' -PassThru; Start-Sleep -Seconds {seconds};"
        "$alive = -not $p.HasExited; if ($alive) { Stop-Process -Id $p.Id -Force };"
        "@{ alive = $alive; pid = $p.Id } | ConvertTo-Json -Compress"
    )
    proc = powershell(script)
    text = (proc.stdout or "").strip().splitlines()
    try:
        result = json.loads(text[-1]) if text else {}
    except json.JSONDecodeError:
        result = {"alive": False, "raw": (proc.stdout or "")[-200:]}
    result["observed_seconds"] = seconds
    return result


def stage_dir(version: str) -> pathlib.Path:
    return STAGE / version


def extract_payload(version: str, msi: pathlib.Path) -> dict:
    """Lay the MSI's OWN payload on disk and digest it.

    THE INSTALLED BINARY IS BOUND TO THE PACKAGE, NOT TO target/release. Measured
    while building this matrix: `target/release/linchpin-desktop.exe` (0.1.0
    94c3dfb3..., 0.2.0 2d9a7083..., both 12 500 992 bytes) differs from the binary
    the MSI actually ships (0.1.0 0295f187..., 0.2.0 0cc1e24e..., same size), because
    the bundler relinks the application as part of packaging -- the same
    non-reproducible-link residual recorded under SUP-004. An assertion against the
    target/ copy would therefore have failed for a correct installer, and an
    assertion against nothing would have accepted a wrong binary. `msiexec /a`
    extracts the package's own payload, whose digest IS the shipped identity.
    """
    out = STAGE / f"payload-{version}"
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)
    proc = powershell(
        "$p = Start-Process msiexec.exe -ArgumentList '/a',"
        f"'\"{msi}\"','TARGETDIR=\"{out}\"','/qn','/norestart' -Wait -PassThru; exit $p.ExitCode"
    )
    if proc.returncode != 0:
        raise SystemExit(f"version-matrix: could not extract the v{version} package (exit {proc.returncode})")
    found = sorted(out.glob("**/linchpin-desktop.exe"))
    if not found:
        raise SystemExit(f"version-matrix: the v{version} package contains no linchpin-desktop.exe")
    exe = found[0]
    return {
        "payload_exe": str(exe.relative_to(ROOT)),
        "payload_exe_sha256": sha256_file(exe),
        "payload_exe_bytes": exe.stat().st_size,
    }


def staged(version: str) -> tuple[pathlib.Path, pathlib.Path]:
    directory = stage_dir(version)
    msis = sorted(directory.glob("*.msi"))
    exe = directory / "linchpin-desktop.exe"
    if not msis or not exe.exists():
        raise SystemExit(
            f"version-matrix: artifacts for v{version} are missing under {directory}; "
            "run with --provision to build them"
        )
    return msis[-1], exe


def bump(version: str) -> None:
    """Set the product version in every manifest that carries it."""
    pkg = ROOT / "apps/desktop/package.json"
    data = json.loads(pkg.read_text(encoding="utf-8"))
    data["version"] = version
    pkg.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")

    cargo = ROOT / "apps/desktop/src-tauri/Cargo.toml"
    text = cargo.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if line.startswith("version") and '"' in line:
            lines[index] = f'version = "{version}"\n'
            break
    else:
        raise SystemExit("version-matrix: no version line in the tauri Cargo.toml")
    cargo.write_text("".join(lines), encoding="utf-8")

    conf = ROOT / "apps/desktop/src-tauri/tauri.conf.json"
    data = json.loads(conf.read_text(encoding="utf-8"))
    data["version"] = version
    conf.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def provision() -> dict:
    """Build both versions without leaving the repository modified."""
    snapshot = {}
    for rel in MANIFESTS + LOCKFILES:
        path = ROOT / rel
        snapshot[rel] = path.read_bytes() if path.exists() else None
    built = {}
    try:
        for version in (OLD, NEW):
            bump(version)
            # Cargo.lock records the workspace member's version, so a bump makes it
            # stale and `--locked` refuses to build -- measured: "error: cannot
            # update the lock file Cargo.lock because --locked was passed to
            # prevent this", which is exactly the pinning behaviour REQ-LIC-003
            # wants. The lock is refreshed OFFLINE for workspace members only (no
            # network, no dependency moves) and is restored byte-for-byte with the
            # manifests afterwards.
            refresh = run(["cargo", "update", "--workspace", "--offline"])
            if refresh.returncode != 0:
                tail = "\n".join((refresh.stdout + refresh.stderr).splitlines()[-8:])
                raise SystemExit(f"version-matrix: could not refresh Cargo.lock for v{version}:\n{tail}")
            print(f"version-matrix: building v{version} (this runs the production build)")
            proc = run(["sh", "scripts/build.sh"])
            if proc.returncode != 0:
                tail = "\n".join((proc.stdout + proc.stderr).splitlines()[-15:])
                raise SystemExit(f"version-matrix: build of v{version} failed:\n{tail}")
            msis = sorted((ROOT / "target/release/bundle/msi").glob(f"*{version}*.msi"))
            if not msis:
                msis = sorted((ROOT / "target/release/bundle/msi").glob("*.msi"))
            if not msis:
                raise SystemExit(f"version-matrix: no MSI produced for v{version}")
            exe = ROOT / "target/release/linchpin-desktop.exe"
            target = stage_dir(version)
            if target.exists():
                shutil.rmtree(target)
            target.mkdir(parents=True, exist_ok=True)
            shutil.copy2(msis[-1], target / msis[-1].name)
            shutil.copy2(exe, target / "linchpin-desktop.exe")
            built[version] = {
                "msi": str((target / msis[-1].name).relative_to(ROOT)),
                "msi_sha256": sha256_file(target / msis[-1].name),
                "target_release_exe_sha256": sha256_file(target / "linchpin-desktop.exe"),
            }
            print(f"version-matrix: staged v{version}: {msis[-1].name}")
    finally:
        # Restore every manifest and lockfile byte-for-byte, then PROVE the tree is
        # unchanged. A version bump left behind would silently re-label the product.
        for rel, blob in snapshot.items():
            path = ROOT / rel
            if blob is None:
                path.unlink(missing_ok=True)
            else:
                path.write_bytes(blob)
    diff = run(["git", "diff", "--exit-code", "--", *MANIFESTS, *LOCKFILES])
    if diff.returncode != 0:
        raise SystemExit(
            "version-matrix: the repository was left modified by provisioning:\n"
            + (diff.stdout + diff.stderr)
        )
    print("version-matrix: provisioning restored every manifest and lockfile (git diff clean)")
    return built


def main() -> int:
    # MUTATION SUPPORT (DOD-018/DOD-035). `--mutate destructive-upgrade` injects the
    # defect this matrix exists to catch -- an upgrade that DESTROYS the user's
    # persistent state -- by deleting the seeded vault immediately after the upgrade
    # installs. The matrix must then FAIL, which is what makes its PASS meaningful
    # rather than merely green. Used by the mutation harness and recorded in the
    # evidence file.
    mutate = None
    if "--mutate" in sys.argv:
        mutate = sys.argv[sys.argv.index("--mutate") + 1]

    if "--provision" in sys.argv:
        built = provision()
    else:
        built = {}
    for version in (OLD, NEW):
        msi, exe = staged(version)
        entry = built.setdefault(version, {})
        entry.update(
            {
                "msi": str(msi.relative_to(ROOT)),
                "msi_sha256": sha256_file(msi),
                # Provenance only: the pre-bundle build output, recorded so the
                # measured difference between it and the shipped payload stays
                # visible instead of being quietly ignored.
                "target_release_exe_sha256": sha256_file(exe),
            }
        )
        entry.update(extract_payload(version, msi))
    if built[OLD]["payload_exe_sha256"] == built[NEW]["payload_exe_sha256"]:
        raise SystemExit(
            "version-matrix: the two versions ship the SAME binary digest, so an "
            "upgrade cannot be distinguished from a reinstall; refusing to report a matrix"
        )

    old_msi = (ROOT / built[OLD]["msi"]).resolve()
    new_msi = (ROOT / built[NEW]["msi"]).resolve()
    db = appdata_root() / "linchpin-vault.db"
    blob = appdata_root() / "vaults" / "canary-blob.bin"
    token = f"LINCHPIN-VMATRIX-{int(time.time())}-{os.getpid()}"

    steps: list[dict] = []
    failures: list[str] = []

    def record(step: str, detail: dict, verdict: str = "PASS") -> None:
        steps.append({"step": step, "verdict": verdict, **detail})
        if verdict != "PASS":
            failures.append(f"{step}: {detail.get('why', verdict)}")
        print(f"version-matrix: {step}: {verdict} -- {json.dumps(detail, sort_keys=True)[:200]}")

    try:
        print("=== 0/7 clean start: remove any installed copy ===")
        cleanup_code = uninstall_any()
        pre = installed_state()
        record("clean_start", {"msiexec_exit": cleanup_code, "installed": pre.get("installed", False)},
               "PASS" if not pre.get("installed") else "FAIL")

        print(f"=== 1/7 install v{OLD} ===")
        code = msiexec(old_msi, "i")
        state = installed_state()
        ok = code == 0 and state.get("display_version") == OLD
        record("install_old", {"msiexec_exit": code, "display_version": state.get("display_version"),
                               "exe_sha256": state.get("exe_sha256"),
                               "matches_shipped_payload": state.get("exe_sha256") == built[OLD]["payload_exe_sha256"]},
               "PASS" if ok and state.get("exe_sha256") == built[OLD]["payload_exe_sha256"]
               else "FAIL")
        # Under --mutate the run continues to the report so the injected defect is
        # recorded with every step it broke, instead of exiting at the first failure.
        if failures and not mutate:
            raise SystemExit(1)

        print("=== 2/7 seed realistic persistent state ===")
        write_canary(db, blob, token)
        seeded = canary_state(db, blob, token)
        record("seed_state", {"db": str(db), "blob": str(blob), "intact": seeded["intact"]},
               "PASS" if seeded["intact"] else "FAIL")
        # Under --mutate the run continues to the report so the injected defect is
        # recorded with every step it broke, instead of exiting at the first failure.
        if failures and not mutate:
            raise SystemExit(1)

        old_exe_digest = installed_state().get("exe_sha256")
        print(f"=== 3/7 UPGRADE to v{NEW} over v{OLD} ===")
        code = msiexec(new_msi, "i")
        state = installed_state()
        after = canary_state(db, blob, token)
        launched = launch_check(state.get("exe", ""))
        if mutate == "destructive-upgrade":
            # The injected defect: a "successful" upgrade that wipes the user's data.
            db.unlink(missing_ok=True)
            blob.unlink(missing_ok=True)
            after = canary_state(db, blob, token)
            print("version-matrix: MUTATION destructive-upgrade applied (seeded state deleted)")
        ok = (
            code == 0
            and state.get("display_version") == NEW
            and state.get("exe_sha256") == built[NEW]["payload_exe_sha256"]
            and after["intact"]
            and launched.get("alive") is True
        )
        record("upgrade", {
            "msiexec_exit": code,
            "display_version": state.get("display_version"),
            "exe_sha256": state.get("exe_sha256"),
            "matches_shipped_payload": state.get("exe_sha256") == built[NEW]["payload_exe_sha256"],
            "binary_replaced": state.get("exe_sha256") != old_exe_digest,
            "state_intact": after["intact"],
            "upgraded_binary_launches": launched.get("alive"),
            "launch_observation_seconds": launched.get("observed_seconds"),
        }, "PASS" if ok else "FAIL")
        # Under --mutate the run continues to the report so the injected defect is
        # recorded with every step it broke, instead of exiting at the first failure.
        if failures and not mutate:
            raise SystemExit(1)

        print(f"=== 4/7 DOWNGRADE attempt to v{OLD} over v{NEW} ===")
        code = msiexec(old_msi, "i")
        state = installed_state()
        after = canary_state(db, blob, token)
        # Windows Installer MAY refuse an in-place downgrade. Either outcome is
        # acceptable, but the installed version must AGREE with it and the user's
        # data must survive: that is what makes this a measurement instead of an
        # assumption.
        version = state.get("display_version")
        consistent = (
            (code == 0 and version == OLD and state.get("exe_sha256") == built[OLD]["payload_exe_sha256"])
            or (code != 0 and version == NEW)
        )
        record("downgrade_attempt", {
            "msiexec_exit": code,
            "display_version": version,
            "outcome": "applied" if code == 0 else "refused by Windows Installer",
            "consistent_with_outcome": consistent,
            "state_intact": after["intact"],
        }, "PASS" if consistent and after["intact"] else "FAIL")
        # Under --mutate the run continues to the report so the injected defect is
        # recorded with every step it broke, instead of exiting at the first failure.
        if failures and not mutate:
            raise SystemExit(1)

        print("=== 5/7 UNINSTALL: removal must not destroy user data ===")
        code = uninstall_any()
        state = installed_state()
        after = canary_state(db, blob, token)
        record("uninstall", {
            "msiexec_exit": code,
            "still_installed": state.get("installed", False),
            "state_intact": after["intact"],
        }, "PASS" if not state.get("installed") and after["intact"] else "FAIL")
        # Under --mutate the run continues to the report so the injected defect is
        # recorded with every step it broke, instead of exiting at the first failure.
        if failures and not mutate:
            raise SystemExit(1)

        print(f"=== 6/7 ROLLBACK: install v{OLD} again ===")
        code = msiexec(old_msi, "i")
        state = installed_state()
        after = canary_state(db, blob, token)
        ok = (
            code == 0
            and state.get("display_version") == OLD
            and state.get("exe_sha256") == built[OLD]["payload_exe_sha256"]
            and after["intact"]
        )
        record("rollback", {
            "msiexec_exit": code,
            "display_version": state.get("display_version"),
            "exe_sha256": state.get("exe_sha256"),
            "matches_shipped_payload": state.get("exe_sha256") == built[OLD]["payload_exe_sha256"],
            "state_intact": after["intact"],
        }, "PASS" if ok else "FAIL")
    finally:
        print("=== 7/7 cleanup ===")
        uninstall_any()
        for path in (db, blob):
            try:
                path.unlink()
            except FileNotFoundError:
                pass
        for directory in (appdata_root() / "vaults", appdata_root()):
            try:
                directory.rmdir()
            except OSError:
                pass
        record("cleanup", {"installed": installed_state().get("installed", False),
                           "canary_removed": not db.exists() and not blob.exists()})

    report = {
        "gate": "version-matrix",
        "covers": ["DOD-035", "DOD-036"],
        "harness": "scripts/version-matrix.py",
        "mutation": mutate,
        "versions": built,
        "canary": token,
        "environment": {
            "host": os.environ.get("COMPUTERNAME", "unknown"),
            "appdata": str(appdata_root()),
            "note": "single machine, single user; no shared server state exists in this product",
        },
        "steps": steps,
        "failures": failures,
        "applicability_notes": [
            "mixed-fleet and multi-machine version skew: NOT APPLICABLE -- a single-user desktop product with device-local storage and no server fleet; the product claims no such support, so there is no path to execute",
            "the installed executable's UI behaviour is covered by the exact-artifact lane (DOD-004); this matrix asserts version identity, installed-binary identity and persistent-state reconciliation",
        ],
        "verdict": "PASS" if not failures else "FAIL",
    }
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    # A mutation run must not overwrite the real evidence, or a deliberately red
    # run would become the recorded result of the lane.
    report_name = f"report-mutation-{mutate}.json" if mutate else "report.json"
    (EVIDENCE / report_name).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if mutate:
        print(f"version-matrix: mutation report written to {EVIDENCE / report_name}")
        return 1 if failures else 0

    lines = [
        "# DOD-035 cross-version matrix",
        "",
        "Generated by `scripts/version-matrix.py`. Do not hand-edit.",
        "",
        f"- Verdict: **{report['verdict']}**",
        f"- Artifacts: v{OLD} `{built[OLD]['msi_sha256'][:16]}...`, v{NEW} `{built[NEW]['msi_sha256'][:16]}...`",
        f"- Canary: `{token}`",
        "",
        "| Step | Verdict | Measured |",
        "| --- | --- | --- |",
    ]
    for step in steps:
        measured = {k: v for k, v in step.items() if k not in {"step", "verdict"}}
        lines.append(f"| {step['step']} | {step['verdict']} | `{json.dumps(measured, sort_keys=True)[:220]}` |")
    lines += [
        "",
        "## Not applicable, stated rather than dropped",
        "",
    ] + [f"- {note}" for note in report["applicability_notes"]] + [
        "",
        "## Reproduce",
        "",
        "```",
        "python3 scripts/version-matrix.py --provision   # builds both versions, then runs the matrix",
        "python3 scripts/version-matrix.py               # reuse staged artifacts under target/version-matrix",
        "```",
        "",
    ]
    (EVIDENCE / "STATUS.md").write_text("\n".join(lines), encoding="utf-8")

    if failures:
        print("version-matrix: FAIL", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    print(f"version-matrix: ok ({len(steps)} steps, canary survived upgrade, downgrade attempt, uninstall and rollback)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
