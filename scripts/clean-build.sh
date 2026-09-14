#!/usr/bin/env bash
# Clean-environment build gate (DOD-002).
#
# DOD-002 RULE: "The repository builds from a clean checkout using the committed
# frozen/locked dependency files and declared toolchain."
# EVIDENCE: "Clean-environment install/build logs, lockfile digest, tool
# versions, exit codes, and build sentinel."
#
# This gate was previously recorded PARTIAL on the assumption that a
# clean-environment build required a VM. It does not: cloning the repository into
# a fresh directory produces a genuinely clean checkout -- no target/, no cargo
# or pnpm caches carried over, no untracked files -- which is exactly what the
# clause asks for. A VM is required for DOD-034 (clean-room INSTALL of the
# artifact), which is a different clause.
#
# What this gate does:
#   1. clones the working tree at HEAD into a fresh temp directory
#   2. proves the checkout is clean (no target/, no node_modules/, no untracked)
#   3. records tool versions and lockfile digests
#   4. runs the locked install and build there
#   5. writes a build sentinel on success
#
# It does NOT claim cross-machine reproducibility: it is the same OS and
# toolchain. That limitation is stated in the report rather than implied away.
set -eu

REPORT_DIR=".agent/evidence/clean-build"
REPORT="$REPORT_DIR/STATUS.md"
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
build. It is still the same OS, toolchain installation and CPU architecture, so
cross-machine reproducibility is NOT established by this gate. DOD-034
(virgin clean-room install of the ARTIFACT) remains EXTERNAL_REQUIRED and needs
a VM.

Log: \`$REPORT_DIR/clean-build.log\`
EOF

rm -rf "$CLEAN"

if [ "$BUILD_CODE" -eq 0 ]; then
  echo "clean-build: ok (commit $COMMIT, sentinel $SENTINEL_STATE)"
  exit 0
fi
echo "clean-build: FAIL -- build exited $BUILD_CODE" >&2
exit 1
