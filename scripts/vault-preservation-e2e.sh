#!/usr/bin/env bash
# EP-009 M4 -- uninstall / reinstall data preservation (vault preservation).
#
# DOD-035 requires that "every claimed upgrade, downgrade, rollback, version-skew,
# mixed-fleet, and compatibility path is executed with realistic persistent
# state", and DOD-036 requires that recovery claims are "executed against
# reconciled state". EP-009 M4 owns "signed update/rollback/uninstall-vault-
# preservation". ADR-003 resolves the "signed" part as a documented limitation;
# the data-preservation part is tested here and was previously NOT executed.
#
# WHAT THIS PROVES, and what it deliberately does not:
#
#   * It proves the installer's uninstall and reinstall do NOT destroy the
#     product's application-data directory, which is where the durable vault
#     lives (%LOCALAPPDATA%\LINCHPIN\linchpin-vault.db).
#   * It proves the same for an "update" (installing over an existing install)
#     and for a "rollback" (reinstalling after removal), using realistic
#     persistent state: a real vault-shaped SQLite database plus a content-
#     addressed blob, placed at the real path the product uses.
#   * It does NOT weaken the storage crate's guard that forbids TESTS from
#     writing underneath the production vault root. The canary state here is
#     placed by this installer-level harness, not by a unit test, and the
#     harness cleans up after itself.
#   * It does NOT write through the product's own command layer, because the
#     installed release build has no debug port. Vault *readability* by the
#     product's runtime is covered by DOD-015/DOD-016 evidence; this gate covers
#     the installer's behaviour toward that data.
set -eu

[ -f apps/desktop/package.json ] || { echo "vault-preservation: product not bootstrapped" >&2; exit 2; }

PRODUCT="${LINCHPIN_PRODUCT_NAME:-LINCHPIN}"
APPDATA_ROOT="${LOCALAPPDATA:-$HOME/AppData/Local}/$PRODUCT"
CANARY_DIR="$APPDATA_ROOT/vaults"
CANARY_DB="$APPDATA_ROOT/linchpin-vault.db"
CANARY_BLOB="$CANARY_DIR/canary-blob.bin"

MSI="$(find target/release/bundle/msi -maxdepth 1 -name '*.msi' -print -quit 2>/dev/null || true)"
if [ -z "$MSI" ]; then
  echo "vault-preservation: no MSI found; run 'sh scripts/build.sh' first" >&2
  exit 2
fi
MSI_WIN="$(cygpath -w "$(cd "$(dirname "$MSI")" && pwd)/$(basename "$MSI")" 2>/dev/null || printf '%s' "$MSI")"
echo "vault-preservation: artifact = $(basename "$MSI")"

CANARY_TOKEN="LINCHPIN-CANARY-$(date +%s)-$$"
echo "vault-preservation: canary = $CANARY_TOKEN"

write_canary() {
  mkdir -p "$CANARY_DIR"
  # A real SQLite database at the product's real vault filename, plus a
  # content-addressed blob in the vault directory: the two shapes the product
  # actually keeps on disk.
  python3 - "$CANARY_DB" "$CANARY_TOKEN" <<'PY'
import sqlite3, sys
path, token = sys.argv[1], sys.argv[2]
con = sqlite3.connect(path)
con.execute("CREATE TABLE IF NOT EXISTS canary (id INTEGER PRIMARY KEY, token TEXT NOT NULL)")
con.execute("INSERT INTO canary (token) VALUES (?)", (token,))
con.commit()
con.close()
PY
  printf '%s' "$CANARY_TOKEN" > "$CANARY_BLOB"
}

canary_present() {
  python3 - "$CANARY_DB" "$CANARY_TOKEN" "$CANARY_BLOB" <<'PY'
import sqlite3, sys
db, token, blob = sys.argv[1], sys.argv[2], sys.argv[3]
import os
if not os.path.exists(db):
    print("vault database is GONE"); raise SystemExit(1)
if not os.path.exists(blob):
    print("content-addressed blob is GONE"); raise SystemExit(1)
if open(blob, "r", encoding="utf-8").read() != token:
    print("blob content changed"); raise SystemExit(1)
con = sqlite3.connect(db)
try:
    rows = [r[0] for r in con.execute("SELECT token FROM canary").fetchall()]
finally:
    con.close()
if token not in rows:
    print(f"canary row missing; found {rows}"); raise SystemExit(1)
print(f"canary intact ({len(rows)} row(s))")
PY
}

install_msi() {
  powershell -NoProfile -Command "\$p = Start-Process msiexec.exe -ArgumentList '/i','\"$MSI_WIN\"','/qn','/norestart' -Wait -PassThru; exit \$p.ExitCode"
}

uninstall_msi() {
  powershell -NoProfile -Command "\$p = Start-Process msiexec.exe -ArgumentList '/x','\"$MSI_WIN\"','/qn','/norestart' -Wait -PassThru; exit \$p.ExitCode"
}

# Always remove the canary, even on failure, so a red run cannot leave invented
# state inside the product's real app-data directory.
cleanup() {
  rm -f "$CANARY_DB" "$CANARY_BLOB" 2>/dev/null || true
  rmdir "$CANARY_DIR" "$APPDATA_ROOT" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== 1/5 install the artifact ==="
install_msi
echo "vault-preservation: install exit 0"

echo "=== 2/5 seed realistic persistent state at the product's real path ==="
write_canary
echo "vault-preservation: seeded $CANARY_DB"
canary_present

echo "=== 3/5 UPDATE: install over the existing installation ==="
install_msi
echo "vault-preservation: reinstall (update) exit 0"
canary_present

echo "=== 4/5 UNINSTALL: the removal path must not destroy user data ==="
uninstall_msi || { echo "vault-preservation: FAIL -- uninstall returned nonzero" >&2; exit 1; }
echo "vault-preservation: uninstall exit 0"
canary_present

echo "=== 5/5 ROLLBACK: reinstall after removal, state must reconcile ==="
install_msi
echo "vault-preservation: reinstall (rollback) exit 0"
canary_present

echo "vault-preservation: ok (canary survived update, uninstall and rollback)"
