# ADR-004: Clean-room support scope is Windows 10 or higher

Status: **ACCEPTED** (human decision, 2026-09-14)
Candidate: `75fc35c` / `x86_64-pc-windows-msvc`
Related: PF-005, PF-016, DOD-034, DOD-035

## Context

PF-016 asks for "Clean Win10 + Win11 VM/physical reference targets" and its
failure effect is "blocks artifact/cleanroom gates". Probed on this host:

```text
VBoxManage  -> absent
vmrun       -> absent
virsh       -> absent
docker      -> present   (not usable for an MSI/WebView2 desktop install)
[Environment]::OSVersion.VersionString -> Microsoft Windows NT 10.0.19045.0
```

So exactly one real target exists (Windows 10 22H2, build 19045) and no
hypervisor is available to create a second. Windows 11 was genuinely unavailable.

## Decision

The owner scoped the **supported clean-room matrix to Windows 10 or higher**,
rather than claiming an untested platform or leaving the gate blocked.

## What this does and does not change

- **Does:** define the supported platform boundary as Windows 10+, which is what
  the project can actually test. DOD-034 and DOD-035 scope to that boundary.
- **Does not:** verify Windows 11. Windows 11 is **not** claimed as verified and
  no artifact, README or release note may describe the product as
  "Windows 11 compatible" or "Win11 tested" until a real Windows 11 target
  produces artifact evidence. A claim beyond the tested boundary would violate
  AGENTS.md §6 ("Product claims in UI/docs must not exceed proven behavior").
- **Does not** by itself satisfy DOD-034. That clause requires a *virgin* clean
  room — a zero-state host installing the exact artifact from public
  documentation. This host is a developer machine with the toolchain installed,
  so it is not virgin. DOD-034 therefore stays EXTERNAL_REQUIRED for the
  zero-state install, while the *platform scope* question is now settled and no
  longer blocks on an unavailable Windows 11 target.

## Consequences

`README.md`, `DEPLOYMENT.md` and the support matrix must state "Windows 10 or
higher" and must not name Windows 11 as verified. If a Windows 11 target is later
provided, DOD-034/035 evidence is added and this ADR is amended to record the
widened matrix.
