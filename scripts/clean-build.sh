#!/usr/bin/env bash
# Clean-environment build gate (DOD-002) and environment-manifest evidence
# (DOD-005).
#
# DOD-002 RULE: "The repository builds from a clean checkout using the committed
# frozen/locked dependency files and declared toolchain."
# EVIDENCE: "Clean-environment install/build logs, lockfile digest, tool
# versions, exit codes, and build sentinel."
#
# DOD-005 RULE: "Required tests execute in an ephemeral clean environment or a
# documented persistent environment created from a known baseline."
# EVIDENCE: "Environment manifest, image/VM digest, cache policy, provisioning
# logs, and teardown proof."
# DOD-005 was recorded FAIL because no environment manifest, cache policy,
# provisioning log or teardown proof existed. This script now emits all four,
# machine-readably, in `.agent/evidence/clean-build/ENVIRONMENT.json`:
#   * MANIFEST   -- OS caption/version/build, architecture, CPU count, Python,
#                   rustc/cargo/node/pnpm with resolved paths, WebView2 runtime,
#                   free disk, and the baseline commit.
#   * CACHE POLICY -- what the clean checkout starts WITHOUT (target/,
#                   node_modules/, untracked files) and, stated honestly, the
#                   caches it still shares (the cargo registry and the pnpm
#                   store), so "no network and no cache" is not claimed.
#   * PROVISIONING -- every step with its exit code.
#   * TEARDOWN   -- proof that the ephemeral directory is gone afterwards.
#
# It is still a clean CHECKOUT, not a clean MACHINE: same OS, toolchain
# installation and architecture, so cross-machine reproducibility is NOT
# established. That limitation is stated in the report rather than implied away.
set -eu

REPORT_DIR=".agent/evidence/clean-build"
REPORT="$REPORT_DIR/STATUS.md"
ENVIRONMENT="$REPORT_DIR/ENVIRONMENT.json"
mkdir -p "$REPORT_DIR"

SRC="$(pwd)"
CLEAN="$(mktemp -d 2>/dev/null || echo "${TMPDIR:-/tmp}/linchpin-clean-$$")"
LOG="$SRC/$REPORT_DIR/clean-build.log"

echo "clean-build: source   = $SRC"
echo "clean-build: checkout = $CLEAN"

# --- 0. guard against untested working-tree edits -------------------------
# `git clone` copies COMMITTED state only. Any uncommitted or untracked source
# change is therefore invisible to this gate, which would let it report PASS for
# a revision the developer is not actually working on -- a DOD-024 masking
# pattern. Observed: a deliberately broken crates/domain/src/lib.rs that was
# never committed produced "clean-build: ok". Either commit the work, or opt in
# explicitly with LINCHPIN_CLEAN_BUILD_DIRTY=1 to test the tree as it stands
# (the gate then reports DIRTY and copies the working tree instead of cloning).
DIRTY="$(git status --porcelain | wc -l | tr -d ' ')"
DIRTY_MODE="no"
if [ "$DIRTY" != "0" ]; then
  if [ "${LINCHPIN_CLEAN_BUILD_DIRTY:-0}" != "1" ]; then
    echo "clean-build: FAIL -- $DIRTY uncommitted change(s); this gate clones" >&2
    echo "  from git HEAD and would otherwise test a revision you are not on." >&2
    echo "  Commit the work, or set LINCHPIN_CLEAN_BUILD_DIRTY=1 to test the" >&2
    echo "  working tree as it stands." >&2
    git status --short | head -20 >&2
    exit 2
  fi
  DIRTY_MODE="yes"
  echo "clean-build: WARNING -- testing a DIRTY working tree (uncommitted changes present)"
fi

# --- 1. materialise the checkout ------------------------------------------
if [ "$DIRTY_MODE" = "yes" ]; then
  # Copy the working tree, excluding build output and installed packages, so
  # the checkout is still clean of the state this gate exists to remove.
  mkdir -p "$CLEAN"
  tar -cf - --exclude=./target --exclude=./node_modules --exclude=./.git \
      --exclude=./apps/desktop/dist --exclude=./apps/desktop/src-tauri/gen \
      . 2>/dev/null | ( cd "$CLEAN" && tar -xf - )
  git -C "$SRC" rev-parse HEAD > "$CLEAN/.git-head"
else
  git clone --quiet "$SRC" "$CLEAN" 2>&1 | tee "$LOG"
  if [ ! -d "$CLEAN/.git" ]; then
    echo "clean-build: FAIL -- clone did not produce a repository" >&2
    exit 1
  fi
fi
COMMIT="$(git -C "$SRC" rev-parse HEAD)"

# --- 2. prove the checkout is clean ---------------------------------------
# `target/` and `node_modules/` must be absent either way: their presence is the
# hidden state this gate exists to remove.
CLEAN_TARGET="no"
if [ -d "$CLEAN/target" ]; then
  CLEAN_TARGET="yes"
fi
CLEAN_NM="no"
if [ -d "$CLEAN/node_modules" ]; then
  CLEAN_NM="yes"
fi
if [ "$DIRTY_MODE" = "yes" ]; then
  # A working-tree copy has no .git, so `git status` cannot be used; the
  # exclusion list above is what guarantees cleanliness.
  UNTRACKED="n/a (working-tree copy)"
else
  UNTRACKED="$(git -C "$CLEAN" status --porcelain | wc -l | tr -d ' ')"
fi

if [ "$CLEAN_TARGET" = "yes" ] || [ "$CLEAN_NM" = "yes" ] || [ "$UNTRACKED" = "1" ]; then
  echo "clean-build: FAIL -- checkout is not clean (target=$CLEAN_TARGET node_modules=$CLEAN_NM untracked=$UNTRACKED)" >&2
  exit 1
fi
echo "clean-build: checkout verified clean (target=$CLEAN_TARGET node_modules=$CLEAN_NM)"

# --- 3. tool versions and lockfile digests --------------------------------
RUSTC_V="$(rustc --version)"
CARGO_V="$(cargo --version)"
NODE_V="$(node --version 2>/dev/null || echo unavailable)"
PNPM_V="$(pnpm --version 2>/dev/null || echo unavailable)"
sha_of() { python3 -c "import hashlib,sys;print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$1"; }
CARGO_LOCK_SHA="$(sha_of "$CLEAN/Cargo.lock")"
PNPM_LOCK_SHA="$(sha_of "$CLEAN/pnpm-lock.yaml")"
TOOLCHAIN_PIN="$(cat "$CLEAN/rust-toolchain.toml" | tr -d '\r' | tr '\n' ' ')"

# --- 4. locked install and build in the clean checkout --------------------
# The exit code is captured via a file, not a variable: in `cmd | tee` the
# command runs in a subshell, so an assignment inside it would not survive.
set +e
( cd "$CLEAN" && cargo build --workspace --locked ) > "$LOG.raw" 2>&1
BUILD_CODE=$?
set -e
cat "$LOG.raw" | tee -a "$LOG" | tail -3
rm -f "$LOG.raw"

echo "clean-build: cargo build exit=$BUILD_CODE"

# --- 4b. the REQUIRED RUST TESTS in the same ephemeral environment (DOD-005) ---
# DOD-005 asks that required tests execute in an ephemeral clean environment OR a
# documented persistent one. Running the whole Rust suite here satisfies the
# first branch for every Rust lane, instead of resting on the second branch and a
# caveat. The JavaScript lanes (vitest, Playwright, the artifact and installer
# lanes) still run on the documented persistent host; that split is stated in the
# report rather than glossed.
set +e
( cd "$CLEAN" && cargo test --workspace --locked ) > "$LOG.tests" 2>&1
TEST_CODE=$?
set -e
# `grep -c` prints 0 AND exits 1 when there is no match, so a bare `|| echo 0`
# appends a SECOND zero and the manifest writer sees "0\n0" (measured: it crashed
# the gate after a green test run). `|| true` keeps the single printed count.
TEST_SUMMARY="$(grep -c 'test result: ok' "$LOG.tests" 2>/dev/null || true)"
TEST_FAILED="$(grep -c 'test result: FAILED' "$LOG.tests" 2>/dev/null || true)"
TEST_SUMMARY="${TEST_SUMMARY:-0}"
TEST_FAILED="${TEST_FAILED:-0}"
cat "$LOG.tests" | tee -a "$LOG" | tail -3
rm -f "$LOG.tests"
echo "clean-build: cargo test exit=$TEST_CODE (ok-suites=$TEST_SUMMARY failed-suites=$TEST_FAILED)"

# --- 5. sentinel ----------------------------------------------------------
SENTINEL="$CLEAN/target/debug/linchpin-desktop.exe"
SENTINEL_STATE="absent"
SENTINEL_SHA="n/a"
if [ -f "$SENTINEL" ]; then
  SENTINEL_STATE="present"
  SENTINEL_SHA="$(sha_of "$SENTINEL")"
fi

cat > "$SRC/$REPORT" <<EOF
# DOD-002 Clean-Environment Build

Generated by \`scripts/clean-build.sh\`. Do not hand-edit.

## Result

| Field | Value |
| --- | --- |
| Verdict | $([ "$BUILD_CODE" -eq 0 ] && echo PASS || echo FAIL) |
| Working tree | $([ "$DIRTY_MODE" = "yes" ] && echo "DIRTY -- uncommitted changes were tested" || echo "clean -- cloned from HEAD") |
| Commit | \`$COMMIT\` |
| Checkout | \`$CLEAN\` |
| \`target/\` present in fresh clone | $CLEAN_TARGET |
| \`node_modules/\` present in fresh clone | $CLEAN_NM |
| Untracked files in fresh clone | $UNTRACKED |
| \`cargo build --workspace --locked\` exit | $BUILD_CODE |
| \`cargo test --workspace --locked\` exit (DOD-005) | $TEST_CODE (ok suites $TEST_SUMMARY, failed suites $TEST_FAILED) |
| Build sentinel | $SENTINEL_STATE |

## Toolchain (declared vs observed)

| Tool | Version |
| --- | --- |
| \`rust-toolchain.toml\` | \`$TOOLCHAIN_PIN\` |
| rustc | \`$RUSTC_V\` |
| cargo | \`$CARGO_V\` |
| node | \`$NODE_V\` |
| pnpm | \`$PNPM_V\` |

## Lockfile digests (from the clean checkout)

- \`Cargo.lock\` sha256 \`$CARGO_LOCK_SHA\`
- \`pnpm-lock.yaml\` sha256 \`$PNPM_LOCK_SHA\`

## Build sentinel digest

- \`target/debug/linchpin-desktop.exe\` sha256 \`$SENTINEL_SHA\`

## Limitation

This is a clean CHECKOUT, not a clean MACHINE. It removes the two forms of
hidden state a repository can carry -- build output and installed packages --
and proves the committed lockfiles plus the declared toolchain are sufficient to
build AND to run the Rust test suite (DOD-005). It is still the same OS,
toolchain installation and CPU architecture, so cross-machine reproducibility is
NOT established by this gate. DOD-034 (virgin clean-room install of the
ARTIFACT) remains EXTERNAL_REQUIRED and needs a VM.

The environment manifest for DOD-005 -- OS build, architecture, WebView2 runtime,
resolved tool paths, cache policy, provisioning log and teardown proof -- is
\`$REPORT_DIR/ENVIRONMENT.json\`.

Log: \`$REPORT_DIR/clean-build.log\`
EOF

rm -rf "$CLEAN"

# --- 6. teardown proof and environment manifest (DOD-005) -----------------
TEARDOWN="absent"
if [ -d "$CLEAN" ]; then
  TEARDOWN="STILL PRESENT"
fi
python3 - "$SRC/$ENVIRONMENT" "$COMMIT" "$CLEAN" "$TEARDOWN" "$CLEAN_TARGET" \
        "$CLEAN_NM" "$UNTRACKED" "$BUILD_CODE" "$DIRTY_MODE" "$SENTINEL_SHA" \
        "$CARGO_LOCK_SHA" "$PNPM_LOCK_SHA" "$RUSTC_V" "$CARGO_V" "$NODE_V" "$PNPM_V" \
        "$TEST_CODE" "$TEST_SUMMARY" "$TEST_FAILED" <<'PY'
"""Write the machine-readable environment manifest for DOD-005.

Everything here is MEASURED on this host at this moment, not transcribed from a
document. Fields that cannot be measured on this platform are reported as
"unavailable" rather than guessed, and the cache policy states plainly which
caches the build still shares -- a manifest that implied a hermetic build would
be a stronger claim than this gate earns.
"""
import json
import shutil
import sys
import platform
import subprocess

(
    out_path, commit, checkout, teardown, clean_target, clean_nm, untracked,
    build_code, dirty_mode, sentinel_sha, cargo_lock_sha, pnpm_lock_sha,
    rustc_v, cargo_v, node_v, pnpm_v, test_code, test_summary, test_failed,
) = sys.argv[1:20]


def powershell(script: str) -> str:
    try:
        result = subprocess.run(
            ["powershell", "-NoProfile", "-Command", script],
            capture_output=True, text=True, timeout=30, shell=False,
        )
    except Exception:
        return "unavailable"
    text = (result.stdout or "").strip()
    return text.splitlines()[0].strip() if text else "unavailable"


def disk_free_bytes() -> int | None:
    try:
        return shutil.disk_usage(".").free
    except Exception:
        return None


manifest = {
    "clause": "DOD-005",
    "harness": "scripts/clean-build.sh",
    "baseline": {
        "commit": commit,
        "ephemeral_checkout": checkout,
        "working_tree_mode": "DIRTY (working tree copied)" if dirty_mode == "yes" else "clean (cloned from HEAD)",
        "note": "the baseline is the committed revision plus the toolchain below; no VM image exists, so no image digest is claimed",
    },
    "os": {
        "caption": powershell("(Get-CimInstance Win32_OperatingSystem).Caption"),
        "version": powershell("(Get-CimInstance Win32_OperatingSystem).Version"),
        "build": powershell("(Get-CimInstance Win32_OperatingSystem).BuildNumber"),
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "cpu_count": platform.os.cpu_count() if hasattr(platform, "os") else None,
    },
    "runtime_prerequisites": {
        "webview2": powershell(
            "(Get-ItemProperty 'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' "
            "-ErrorAction SilentlyContinue).pv"
        ),
    },
    "toolchain": {
        "rust_toolchain_pin": "see rust-toolchain.toml",
        "rustc": rustc_v,
        "cargo": cargo_v,
        "rustc_path": shutil.which("rustc") or "unavailable",
        "cargo_path": shutil.which("cargo") or "unavailable",
        "node": node_v,
        "node_path": shutil.which("node") or "unavailable",
        "pnpm": pnpm_v,
        "pnpm_path": shutil.which("pnpm") or "unavailable",
    },
    "cache_policy": {
        "absent_in_checkout": {
            "target_dir": clean_target,
            "node_modules_dir": clean_nm,
            "untracked_files": untracked,
        },
        "shared_on_this_host": [
            "the cargo registry cache (CARGO_HOME)",
            "the pnpm content-addressable store",
        ],
        "statement": (
            "build output and installed packages are absent from the checkout, which is the "
            "hidden state this gate removes; the dependency DOWNLOAD caches are shared with the "
            "host, so this is not a claim of a hermetic, cache-free or offline build. Version "
            "resolution is pinned by the committed lockfiles and enforced with --locked."
        ),
    },
    "provisioning": [
        {"step": "materialise checkout", "detail": "clone from HEAD, or copy the working tree excluding target/, node_modules/, dist/ and gen/", "exit_code": 0},
        {"step": "verify checkout cleanliness", "detail": f"target={clean_target} node_modules={clean_nm} untracked={untracked}", "exit_code": 0 if (clean_target == "no" and clean_nm == "no" and untracked in ("0", "n/a (working-tree copy)")) else 1},
        {"step": "cargo build --workspace --locked", "detail": "in the ephemeral checkout", "exit_code": int(build_code)},
        {"step": "cargo test --workspace --locked", "detail": f"in the ephemeral checkout; ok suites={test_summary} failed suites={test_failed}", "exit_code": int(test_code)},
    ],
    "tests_in_clean_environment": {
        "rust_lanes": "executed in the ephemeral checkout (cargo test --workspace --locked)",
        "javascript_lanes": "NOT executed here; vitest, Playwright and the artifact/installer lanes run on the documented persistent host below",
        "exit_code": int(test_code),
        "ok_suites": int(test_summary),
        "failed_suites": int(test_failed),
    },
    "lockfiles": {"Cargo.lock_sha256": cargo_lock_sha, "pnpm-lock.yaml_sha256": pnpm_lock_sha},
    "build_sentinel": {"path": "target/debug/linchpin-desktop.exe", "sha256": sentinel_sha},
    "resources": {"disk_free_bytes": disk_free_bytes()},
    "teardown": {
        "ephemeral_dir": checkout,
        "state_after_teardown": teardown,
        "proof": "the directory is removed and its absence is checked after removal",
    },
    "limits": [
        "clean CHECKOUT, not a clean MACHINE: same OS, toolchain installation and architecture",
        "dependency download caches (cargo registry, pnpm store) are shared with the host",
        "no VM image exists, so no image digest is claimed; DOD-034 clean-room install remains EXTERNAL_REQUIRED",
    ],
}

with open(out_path, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2)
    handle.write("\n")
print(f"clean-build: environment manifest written to {out_path} (teardown={teardown})")
PY

if [ "$TEARDOWN" != "absent" ]; then
  echo "clean-build: FAIL -- the ephemeral checkout was not torn down" >&2
  exit 1
fi

if [ "$BUILD_CODE" -eq 0 ] && [ "$TEST_CODE" -eq 0 ]; then
  echo "clean-build: ok (commit $COMMIT, sentinel $SENTINEL_STATE, rust suites ok=$TEST_SUMMARY)"
  exit 0
fi
echo "clean-build: FAIL -- build exited $BUILD_CODE, tests exited $TEST_CODE" >&2
exit 1
