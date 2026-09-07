# Provider Transport Matrix

| Provider | Default path | Authentication owner | Programmatic mode | Default shipping state | Key rule |
| --- | --- | --- | --- | --- | --- |
| OpenAI | Official Codex CLI / supported app-server boundary | Codex/ChatGPT first-party flow | CLI structured output or documented local server | ENABLED_AFTER_PREFLIGHT | Never copy ChatGPT cookies/tokens; capability-probe installed Codex version. |
| Anthropic | Official Claude Code external CLI | Claude first-party account flow | `claude -p` JSON/stream modes | TERMS_REVIEW_REQUIRED | Do not redistribute auth or scrape credential storage; confirm current commercial third-party terms. |
| xAI | Grok Build ACP stdio | Grok Build browser/device flow | ACP stdio preferred; headless fallback | ENABLED_AFTER_PREFLIGHT | ACP is the preferred sanctioned integration boundary. |
| Google | Gemini CLI consumer-OAuth adapter slot | Google account OAuth | none in LINCHPIN while prohibited | DISABLED_BY_PROVIDER_POLICY | Current official FAQ prohibits third-party software from harvesting/piggybacking Gemini CLI OAuth; use a separate opt-in Gemini API/Vertex path unless policy changes. |
| Local | llama.cpp / Ollama | None/local OS | local process/HTTP on loopback | ENABLED | User-selected model; identical schema, redaction and tool gates. |
| Generic API fallback | Provider SDK/HTTP | User API key | documented API | OPT_IN | Separate budget controls; never silently fall back from subscription transport to billable API. |

## JobEnvelope
Fields: `job_id`, `workspace_id`, `purpose`, `egress_class`, `prompt_template_id`, `input_refs`, `expected_schema_id`, `tool_policy_id`, `deadline`, `max_output`, `provider_preference`, `research_evidence_ids`, `human_conception_boundary`, `cancellation_token`.

## Result evidence
Store provider, model label reported by tool, first-party executable version, transport version, start/end times, exit status, schema validation result, input evidence IDs, output artifact hash, safety/policy blocks, retry chain and user-cost class. Do not store hidden reasoning or pretend it is required.
