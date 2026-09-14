#!/usr/bin/env bash
# Exact-artifact smoke test (DOD-004, SUP-003).
#
# Verifies the REAL installed artifact rather than the source tree, by:
#   1. locating a built installer and pinning its SHA-256
#   2. installing it silently with Windows Installer
#   3. independently reading back the installed files and the Add/Remove entry
#   4. launching the INSTALLED binary and confirming a responsive window
#   5. uninstalling and confirming clean removal
#
# DOD-004 requires smoke tests to run against the exact production artifact
# digest, not a development server.
#
# By default this is NON-DESTRUCTIVE: it installs and then removes the product.
# Set LINCHPIN_SMOKE_KEEP=1 to leave the product installed for inspection.
set -eu

KEEP="${LINCHPIN_SMOKE_KEEP:-0}"
PRODUCT="LINCHPIN"
INSTALL_DIR="C:\\Program Files\\LINCHPIN"

find_msi() {
  # Prefer the newest built MSI.
  ls -1t target/release/bundle/msi/*.msi 2>/dev/null | head -n 1
}

MSI="$(find_msi || true)"
if [ -z "$MSI" ]; then
  echo "smoke: no MSI found; run 'sh scripts/build.sh' first" >&2
  exit 2
fi

MSI_ABS="$(cd "$(dirname "$MSI")" && pwd)/$(basename "$MSI")"
echo "smoke: artifact = $MSI_ABS"

# Git Bash reports POSIX paths (/c/dev/...) which Windows PowerShell cannot
# resolve. Convert to a native path before handing anything to powershell.exe.
winpath() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    # Fallback: /c/foo -> C:\foo
    printf '%s' "$1" | sed -e 's|^/\([a-zA-Z]\)/|\1:\\|' -e 's|/|\\|g'
  fi
}
MSI_WIN="$(winpath "$MSI_ABS")"
echo "smoke: artifact (win) = $MSI_WIN"

# Digest the artifact.
#
# `Get-FileHash` is NOT usable here: invoked from Git Bash, powershell.exe
# inherits a POSIX-style PATH (observed as `PATH=\`), so the
# Microsoft.PowerShell.Utility module fails to auto-load and the cmdlet is
# reported as "not recognized". Rather than depend on module auto-loading, use
# .NET directly, which needs no module import.
DIGEST_RAW="$(powershell -NoProfile -Command "
  \$sha = [System.Security.Cryptography.SHA256]::Create()
  \$fs  = [System.IO.File]::OpenRead('$MSI_WIN')
  try {
    \$hash = \$sha.ComputeHash(\$fs)
    (\$hash | ForEach-Object { \$_.ToString('x2') }) -join ''
  } finally { \$fs.Dispose(); \$sha.Dispose() }
" 2>&1 || true)"
DIGEST="$(printf '%s' "$DIGEST_RAW" | tr -d '\r\n ')"
echo "smoke: sha256 = $DIGEST"
case "$DIGEST" in
  [0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]*) : ;;
  *)
    echo "smoke: FAIL -- could not compute artifact digest" >&2
    printf 'smoke: raw output: %s\n' "$DIGEST_RAW" >&2
    exit 1
    ;;
esac

echo "smoke: installing (silent)"
powershell -NoProfile -Command "\$p = Start-Process msiexec.exe -ArgumentList '/i','\"$MSI_WIN\"','/qn','/norestart' -Wait -PassThru; exit \$p.ExitCode"
echo "smoke: install exit 0"

cleanup() {
  if [ "$KEEP" = "1" ]; then
    echo "smoke: LINCHPIN_SMOKE_KEEP=1 -- leaving product installed"
    return
  fi
  echo "smoke: uninstalling"
  powershell -NoProfile -Command "\$p = Start-Process msiexec.exe -ArgumentList '/x','\"$MSI_WIN\"','/qn','/norestart' -Wait -PassThru; exit \$p.ExitCode" || true
}
trap cleanup EXIT

# --- Independent read-back (DOD-012): filesystem and registry -------------
echo "smoke: reading back installed state"
powershell -NoProfile -Command "
  \$exe = Join-Path '$INSTALL_DIR' 'linchpin-desktop.exe'
  if (-not (Test-Path -LiteralPath \$exe)) { Write-Error 'installed exe missing'; exit 1 }
  \$size = (Get-Item -LiteralPath \$exe).Length
  if (\$size -le 0) { Write-Error 'installed exe is empty'; exit 1 }
  Write-Host \"smoke: installed exe = \$size bytes\"

  \$arp = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
    Where-Object { \$_.DisplayName -eq '$PRODUCT' }
  if (-not \$arp) { Write-Error 'Add/Remove Programs entry missing'; exit 1 }
  Write-Host \"smoke: ARP name=\$(\$arp.DisplayName) version=\$(\$arp.DisplayVersion) publisher=\$(\$arp.Publisher)\"
"

# --- Launch the INSTALLED binary (not the dev build) ----------------------
echo "smoke: launching installed artifact"
powershell -NoProfile -Command "
  \$exe = Join-Path '$INSTALL_DIR' 'linchpin-desktop.exe'
  \$p = Start-Process -FilePath \$exe -PassThru
  Start-Sleep -Seconds 5
  if (\$p.HasExited) { Write-Error \"installed app exited early with code \$(\$p.ExitCode)\"; exit 1 }
  \$title = \$p.MainWindowTitle
  if ([string]::IsNullOrWhiteSpace(\$title)) { Write-Error 'no window title'; Stop-Process -Id \$p.Id -Force; exit 1 }
  if (-not \$p.Responding) { Write-Error 'window not responding'; Stop-Process -Id \$p.Id -Force; exit 1 }
  Write-Host \"smoke: window title = '\$title' responding = \$(\$p.Responding)\"
  Stop-Process -Id \$p.Id -Force
  Write-Host 'smoke: launched and stopped cleanly'
"

if [ "$KEEP" != "1" ]; then
  cleanup
  trap - EXIT
  echo "smoke: verifying removal"
  powershell -NoProfile -Command "
    if (Test-Path -LiteralPath '$INSTALL_DIR') { Write-Error 'install dir survived uninstall'; exit 1 }
    \$arp = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue |
      Where-Object { \$_.DisplayName -eq '$PRODUCT' }
    if (\$arp) { Write-Error 'ARP entry survived uninstall'; exit 1 }
    Write-Host 'smoke: clean removal confirmed'
  "
fi

echo "smoke: ok (artifact $DIGEST verified through install/launch/uninstall)"
