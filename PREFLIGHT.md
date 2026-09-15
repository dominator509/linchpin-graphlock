# PREFLIGHT

Preflight must finish before implementation nodes. Missing optional credentials block only their capability lane.

| ID | Requirement | Lane | Machine probe / evidence | Failure effect |
| --- | --- | --- | --- | --- |
| PF-001 | Git >= 2.45 | REQUIRED_NOW | `git --version` | RUN_BLOCKED |
| PF-002 | Rust 1.98.0 pinned by rust-toolchain.toml | REQUIRED_NOW | `rustc --version && cargo --version` | RUN_BLOCKED |
| PF-003 | Node 24.20.0 LTS + pnpm 11.21.0 pinned | REQUIRED_NOW | `node --version && pnpm --version` | RUN_BLOCKED |
| PF-004 | Python 3.13.15 + uv 0.12.0 for optional sidecar | REQUIRED_BEFORE_INTEGRATION | `python --version && uv --version` | blocks sidecar only |
| PF-005 | Windows 10/11 x64 primary build environment | REQUIRED_BEFORE_E2E | `powershell -NoProfile -Command "[Environment]::OSVersion.VersionString"` | blocks Windows artifact gates |
| PF-006 | WebView2/Tauri Windows prerequisites | REQUIRED_BEFORE_E2E | Tauri core 2.11.5 / CLI 2.11.4 environment probe | blocks UI build/E2E |
| PF-007 | OpenAI Codex installed/authenticated when enabled | OPTIONAL | `codex --version`; first-party auth probe without token extraction | blocks OpenAI lane only |
| PF-008 | Claude Code installed/authenticated + terms decision | OPTIONAL | `claude --version`; non-secret auth status; ADR | blocks Anthropic lane only |
| PF-009 | Grok Build installed/authenticated + ACP available | OPTIONAL | `grok --version`; ACP capability probe | blocks xAI lane only |
| PF-010 | Gemini CLI consumer-OAuth policy gate | OPTIONAL | first-party FAQ/terms review + ADR; do not inspect token store | DISABLED_BY_PROVIDER_POLICY unless first-party policy changes |
| PF-011 | local llama.cpp/Ollama provider | **SATISFIED** (2026-09-14) | loopback capability/version/model-hash probe | resolved: Ollama 0.34.0 bound to 127.0.0.1:11434 serving `smollm2:135m`; proven by `sh scripts/live-fire-local-provider.sh`, 11/11 assertions |
| PF-012 | USPTO ODP credential if selected endpoint requires it | REQUIRED_BEFORE_INTEGRATION | official API positive/negative/readback test | blocks ODP live lane only |
| PF-013 | EPO OPS OAuth credential if enabled | OPTIONAL | official OAuth probe + quota capture | blocks EPO live lane only |
| PF-014 | GitHub/gh auth for optional repair PR flow | OPTIONAL | `gh auth status` without logging token | blocks PR automation only |
| PF-015 | Windows signing identity for GA | REQUIRED_BEFORE_DEPLOY | certificate/provider presence + test signing/readback | blocks GA release |
| PF-016 | Clean Win10 + Win11 VM/physical reference targets | REQUIRED_BEFORE_E2E | environment manifests and snapshot IDs | blocks artifact/cleanroom gates |
| PF-017 | Human UAT and manual accessibility validators | HUMAN_EXTERNAL | named/authorized validation window | blocks broad GA if absent |
| PF-018 | Patent-workflow independent reviewer | HUMAN_EXTERNAL | signed review scope/evidence | blocks broad GA if absent |
| PF-019 | Public data-source terms/rate limits inventoried | REQUIRED_BEFORE_INTEGRATION | adapter registry with source date/terms/quota | blocks affected adapter |
| PF-020 | License/SBOM scanners | REQUIRED_NOW | `cargo deny --version` plus JS/Python scanner versions | blocks dependency additions |

## Credential inventory law
No credential discovered after `preflight: ok` may be silently added. Add it here, classify its lane, update `.env.example`/ENVIRONMENT.md, invalidate dependent nodes and rerun preflight. Provider consumer OAuth material is deliberately absent because first-party executables own it.

## Preflight sentinel
`scripts/preflight.sh` exits zero only when all REQUIRED_NOW probes pass and prints exactly `preflight: ok`. Optional/HUMAN_EXTERNAL lanes are reported separately and never converted to PASS.
