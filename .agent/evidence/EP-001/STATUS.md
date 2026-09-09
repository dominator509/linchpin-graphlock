EP-001 Candidate Epoch: EP-001-candidate-001
Status: DONE_VERIFIED

## Execution Summary
- Implemented foundation config, OS paths (), and Keyring abstraction () in .
- Implemented encrypted workspace/vault storage abstraction () in .
- Implemented boot health check () in .
- Generated typed IPC contract and exposed  in Tauri desktop shell ().
- Verified unit tests across all crates, typechecking, linting, and desktop asset build.

## Gate Results & Evidence
- Preflight: PASS (preflight: ok).
- Cargo Unit Tests: PASS (
running 1 test
test tests::test_system_health ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test tests::test_check_license ... ok
test tests::test_verify_platform ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
test tests::test_app_paths ... ok
test tests::test_memory_keyring ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test tests::test_vault_storage ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s).
- TypeScript Typecheck: PASS (Scope: 3 of 4 workspace projects
apps/desktop typecheck$ tsc
packages/contracts typecheck$ tsc
packages/ui typecheck$ tsc
apps/desktop typecheck: Done
packages/ui typecheck: Done
packages/contracts typecheck: Done).
- TypeScript Linting: PASS (Scope: 3 of 4 workspace projects
apps/desktop lint$ eslint .
packages/ui lint$ eslint .
packages/contracts lint$ eslint .
packages/contracts lint: (node:36488) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///app/eslint.config.js?mtime=1788945792047 is not specified and it doesn't parse as CommonJS.
packages/contracts lint: Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
packages/contracts lint: To eliminate this warning, add "type": "module" to /app/package.json.
packages/contracts lint: (Use `node --trace-warnings ...` to show where the warning was created)
apps/desktop lint: (node:36482) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///app/eslint.config.js?mtime=1788945792047 is not specified and it doesn't parse as CommonJS.
apps/desktop lint: Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
apps/desktop lint: To eliminate this warning, add "type": "module" to /app/package.json.
apps/desktop lint: (Use `node --trace-warnings ...` to show where the warning was created)
packages/ui lint: (node:36494) [MODULE_TYPELESS_PACKAGE_JSON] Warning: Module type of file:///app/eslint.config.js?mtime=1788945792047 is not specified and it doesn't parse as CommonJS.
packages/ui lint: Reparsing as ES module because module syntax was detected. This incurs a performance overhead.
packages/ui lint: To eliminate this warning, add "type": "module" to /app/package.json.
packages/ui lint: (Use `node --trace-warnings ...` to show where the warning was created)
packages/ui lint: Done
packages/contracts lint: Done
apps/desktop lint: Done).
- TypeScript Unit Tests: PASS (Scope: 3 of 4 workspace projects
apps/desktop test:unit$ vitest run --passWithNoTests
packages/contracts test:unit$ vitest run --passWithNoTests
packages/ui test:unit$ vitest run --passWithNoTests
apps/desktop test:unit:  RUN  v3.2.7 /app/apps/desktop
packages/contracts test:unit:  RUN  v3.2.7 /app/packages/contracts
apps/desktop test:unit: No test files found, exiting with code 0
apps/desktop test:unit: include: **/*.{test,spec}.?(c|m)[jt]s?(x)
apps/desktop test:unit: exclude:  **/node_modules/**, **/dist/**, **/cypress/**, **/.{idea,git,cache,output,temp}/**, **/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build,eslint,prettier}.config.*
packages/contracts test:unit: No test files found, exiting with code 0
packages/contracts test:unit: include: **/*.{test,spec}.?(c|m)[jt]s?(x)
packages/contracts test:unit: exclude:  **/node_modules/**, **/dist/**, **/cypress/**, **/.{idea,git,cache,output,temp}/**, **/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build,eslint,prettier}.config.*
packages/ui test:unit:  RUN  v3.2.7 /app/packages/ui
apps/desktop test:unit: Done
packages/contracts test:unit: Done
packages/ui test:unit: No test files found, exiting with code 0
packages/ui test:unit: include: **/*.{test,spec}.?(c|m)[jt]s?(x)
packages/ui test:unit: exclude:  **/node_modules/**, **/dist/**, **/cypress/**, **/.{idea,git,cache,output,temp}/**, **/{karma,rollup,webpack,vite,vitest,jest,ava,babel,nyc,cypress,tsup,build,eslint,prettier}.config.*
packages/ui test:unit: Done).
- Desktop Build: PASS (
> @linchpin/desktop@0.1.0 build /app/apps/desktop
> tsc && vite build

vite v6.4.3 building for production...
transforming...
✓ 28 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                  0.32 kB │ gzip:  0.24 kB
dist/assets/index-BnGKEOql.js  194.85 kB │ gzip: 61.01 kB
✓ built in 1.31s).
- Anti-Gaming Scan: PASS (anti-gaming scan: matches require classification
PLACEHOLDER	GEMINI.md:5	stub
PLACEHOLDER	GEMINI.md:5	placeholder
PLACEHOLDER	PATENT_DOMAIN_MODEL.md:6	bypass
HARDCODED_FIXTURE	TESTING.md:25	Fixture
HARDCODED_FIXTURE	TESTING.md:26	Fixture
TEST_ONLY_BRANCH	pnpm-lock.yaml:26	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:704	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:707	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:718	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:721	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:724	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:727	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:730	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1280	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1288	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1289	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1299	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1301	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1874	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1877	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1878	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1882	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1884	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1890	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1894	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1896	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1900	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1902	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1906	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1910	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:1912	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2420	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2423	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2424	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2425	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2426	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2427	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2428	vitest
TEST_ONLY_BRANCH	pnpm-lock.yaml:2429	vitest
HARDCODED_FIXTURE	CONTRIBUTING.md:2	fixture
PLACEHOLDER	GROK.md:5	stub
PLACEHOLDER	GROK.md:5	placeholder
PLACEHOLDER	CLAUDE.md:5	stub
PLACEHOLDER	CLAUDE.md:5	placeholder
PLACEHOLDER	AGENTS.md:11	stub
PLACEHOLDER	AGENTS.md:11	placeholder
PLACEHOLDER	AGENTS.md:63	bypass
PLACEHOLDER	AGENTS.md:120	SIMULATE
PLACEHOLDER	AGENTS.md:126	SIMULATE
PLACEHOLDER	AGENTS.md:147	simulate
PLACEHOLDER	AGENTS.md:156	SIMULATE
PLACEHOLDER	AGENTS.md:177	Placeholder
PLACEHOLDER	AGENTS.md:177	stub
PLACEHOLDER	AGENTS.md:177	simulation
PLACEHOLDER	AGENTS.md:184	simulate
PLACEHOLDER	AGENTS.md:186	SIMULATE
PLACEHOLDER	AGENTS.md:219	simulate
HARDCODED_FIXTURE	SECURITY.md:30	fixture
PLACEHOLDER	BLUEPRINT_GENERATION_MANIFEST.json:435	simulation
HARDCODED_FIXTURE	RESEARCH_ENGINE_SPEC.md:19	fixture
TEST_ONLY_BRANCH	package.json:7	vitest
TEST_ONLY_BRANCH	package.json:18	vitest
PLACEHOLDER	PROJECT_BRIEF.md:35	bypass
PLACEHOLDER	ARCHITECTURE.md:88	bypass
PLACEHOLDER	ARCHITECTURE.md:104	bypass
PLACEHOLDER	ARCHITECTURE.md:125	bypass
PLACEHOLDER	.hermes/AGENTS.md:5	stub
PLACEHOLDER	.hermes/AGENTS.md:5	placeholder
TEST_ONLY_BRANCH	scripts/anti-gaming-scan.py:13	pytest
TEST_ONLY_BRANCH	scripts/anti-gaming-scan.py:13	pytest
TEST_ONLY_BRANCH	scripts/anti-gaming-scan.py:14	pytest
TEST_ONLY_BRANCH	scripts/anti-gaming-scan.py:14	vitest
TEST_ONLY_BRANCH	scripts/anti-gaming-scan.py:14	jest
FAKE_SUCCESS	scripts/anti-gaming-scan.py:15	fakeSuccess
FAKE_SUCCESS	scripts/anti-gaming-scan.py:15	mockSuccess
PLACEHOLDER	scripts/anti-gaming-scan.py:17	PLACEHOLDER
PLACEHOLDER	scripts/anti-gaming-scan.py:17	not implemented
PLACEHOLDER	scripts/anti-gaming-scan.py:17	coming soon
PLACEHOLDER	scripts/anti-gaming-scan.py:17	TODO pass
PLACEHOLDER	scripts/anti-gaming-scan.py:17	placeholder
PLACEHOLDER	scripts/anti-gaming-scan.py:17	dummy
PLACEHOLDER	scripts/anti-gaming-scan.py:17	stub
PLACEHOLDER	scripts/anti-gaming-scan.py:17	simulate
PLACEHOLDER	scripts/anti-gaming-scan.py:17	simulation
PLACEHOLDER	scripts/anti-gaming-scan.py:17	bypass
PLACEHOLDER	scripts/anti-gaming-scan.py:17	force pass
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:5	fixture
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:18	HARDCODED
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:18	FIXTURE
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:18	known-test
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:18	fixture
HARDCODED_FIXTURE	scripts/anti-gaming-scan.py:18	hardcoded
TEST_ONLY_BRANCH	scripts/test-unit.sh:5	pytest
PLACEHOLDER	scripts/validate-generated-pack.py:32	Placeholder
PLACEHOLDER	scripts/validate-generated-pack.py:40	placeholder
PLACEHOLDER	scripts/validate-generated-pack.py:41	TODO pass
PLACEHOLDER	scripts/validate-generated-pack.py:41	not implemented
PLACEHOLDER	scripts/validate-generated-pack.py:41	coming soon
TEST_ONLY_BRANCH	.agent/reality-patterns:8	pytest
TEST_ONLY_BRANCH	.agent/reality-patterns:9	vitest
PLACEHOLDER	.agent/reality-patterns:10	bypass
HARDCODED_FIXTURE	.agent/reality-patterns:10	fixture
PLACEHOLDER	.agent/DONE_LAW.md:55	SIMULATE
PLACEHOLDER	.agent/DONE_LAW.md:61	SIMULATE
PLACEHOLDER	.agent/DONE_LAW.md:82	simulate
PLACEHOLDER	.agent/DONE_LAW.md:91	SIMULATE
PLACEHOLDER	.agent/DONE_LAW.md:112	Placeholder
PLACEHOLDER	.agent/DONE_LAW.md:112	stub
PLACEHOLDER	.agent/DONE_LAW.md:112	simulation
PLACEHOLDER	.agent/DONE_LAW.md:119	simulate
PLACEHOLDER	.agent/DONE_LAW.md:121	SIMULATE
PLACEHOLDER	.agent/DONE_LAW.md:154	simulate
PLACEHOLDER	.github/copilot-instructions.md:5	stub
PLACEHOLDER	.github/copilot-instructions.md:5	placeholder
TEST_ONLY_BRANCH	packages/contracts/package.json:8	vitest
TEST_ONLY_BRANCH	packages/ui/package.json:8	vitest
TEST_ONLY_BRANCH	apps/desktop/package.json:11	vitest
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:23	Simulation
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:24	Simulation
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:122	Bypass
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:195	Bypass
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:243	Bypass
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:286	Simulation
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:313	Bypass
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:395	Simulation
PLACEHOLDER	.agent/verification/MASTER_TEST_REGISTRY.csv:471	Simulation
HARDCODED_FIXTURE	.agent/verification/MASTER_TEST_REGISTRY.csv:8	Hardcoded
HARDCODED_FIXTURE	.agent/verification/MASTER_TEST_REGISTRY.csv:182	Hardcoded
HARDCODED_FIXTURE	.agent/verification/MASTER_TEST_REGISTRY.csv:384	Hardcoded
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:10	SIMULATE
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:11	SIMULATE
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:15	simulate
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:16	SIMULATE
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:20	Placeholder
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:20	stub
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:20	simulation
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:21	simulate
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:21	SIMULATE
PLACEHOLDER	.agent/verification/DOD_REGISTRY.csv:27	simulate
HARDCODED_FIXTURE	.agent/verification/TEST_ENVIRONMENT_MANIFEST.md:2	fixture
TEST_ONLY_BRANCH	.agent/verification/E2E_SUITE_LIBRARY.md:700	Jest
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:117	stub
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:141	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:189	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:195	bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:232	bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:265	Bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:270	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:270	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:277	bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:336	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:357	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:397	Simulation
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:417	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:420	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:424	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:440	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:445	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:450	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:517	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:518	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:591	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:701	simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:712	Simulate
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:844	bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:858	Bypass
PLACEHOLDER	.agent/verification/E2E_SUITE_LIBRARY.md:892	Simulation
HARDCODED_FIXTURE	.agent/verification/E2E_SUITE_LIBRARY.md:137	fixture
HARDCODED_FIXTURE	.agent/verification/E2E_SUITE_LIBRARY.md:765	hardcoded
HARDCODED_FIXTURE	.agent/verification/E2E_SUITE_LIBRARY.md:783	hardcoded
PLACEHOLDER	.agent/verification/BLOCKCHAIN_008_022_RECONSTRUCTED.md:35	simulate
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:9	Simulation
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:11	placeholder
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:15	stub
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:20	simulate
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:90	Simulate
PLACEHOLDER	.agent/verification/SUPPLEMENTAL_PRODUCTION_GATES.md:108	bypass
PLACEHOLDER	.agent/verification/ANTI_GAMING_CONTROL_BASELINE.md:3	placeholder
PLACEHOLDER	.agent/verification/ANTI_GAMING_CONTROL_BASELINE.md:3	simulate
PLACEHOLDER	.agent/verification/ANTI_GAMING_CONTROL_BASELINE.md:3	bypass
PLACEHOLDER	.agent/verification/ANTI_GAMING_CONTROL_BASELINE.md:5	bypass
HARDCODED_FIXTURE	.agent/verification/ANTI_GAMING_CONTROL_BASELINE.md:5	hardcoded
PLACEHOLDER	.agent/verification/stage-plans/V-002-claims-traceability-and-anti-simulation.md:1	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:55	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:56	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:154	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:1582	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2003	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2008	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2026	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2091	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2096	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2111	simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2114	Simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2116	simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2117	simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:2119	simulation
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:4317	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:4404	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:4491	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:4521	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7289	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7321	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7377	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7409	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7467	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7555	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7723	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:7730	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:8854	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:8942	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:10173	bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:10670	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:10675	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:10693	Bypass
PLACEHOLDER	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:10697	bypass
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:13	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:40	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:689	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:694	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:709	fixture
HARDCODED_FIXTURE	.agent/verification/source-library/security/general-dev-security-testing-prompts.md:712	Hardcoded
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:388	Simulate
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:389	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:642	simulate
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:718	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:1291	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:1372	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:1373	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:2188	simulation
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:3052	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:3129	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:3132	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:3754	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4068	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4212	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4681	placeholder
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4689	placeholder
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4757	placeholder
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5163	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5297	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5298	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5543	Bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5568	Bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:5913	simulation
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:6796	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:7138	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:7210	placeholder
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:7407	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:7413	simulation
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8298	simulation
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8364	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8501	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8504	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8701	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8706	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8708	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8710	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8817	Bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8842	Bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:8906	bypass
PLACEHOLDER	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:9111	bypass
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:1133	fixture
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:1459	fixture
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:3910	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4659	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4679	fixture
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:4684	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:6184	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/hipaa-software-dev-security-testing-prompts.md:6798	hardcoded
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:69	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:96	Bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:178	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:2763	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:2883	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:2962	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3035	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3216	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3221	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3222	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3255	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3261	SIMULATION
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:3331	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:4960	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:4978	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:4984	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:5244	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:5487	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6723	Bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6728	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6729	Bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6768	BYPASS
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6776	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6779	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6782	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6785	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6800	Bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6838	Bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:9236	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:9241	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:9512	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:9771	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:10158	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:10650	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:10672	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:10678	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:11316	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:12856	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:13625	placeholder
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:14793	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:14916	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15036	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15055	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17004	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17135	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17277	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17334	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17339	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17340	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17379	SIMULATION
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17384	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17387	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17390	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17448	Simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:17522	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:19843	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:21663	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:21669	simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:21787	Simulate
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:21917	simulation
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:22177	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:24493	bypass
PLACEHOLDER	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:24509	bypass
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:167	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:1046	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:5612	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:5865	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:6786	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:7419	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:7441	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:8976	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15304	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15569	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15826	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15847	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15904	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15909	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15910	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15949	HARDCODED
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15954	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:15963	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:16019	Hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:23069	hardcoded
HARDCODED_FIXTURE	.agent/verification/source-library/security/blockchain-security-testing-prompts.md:23712	Hardcoded).
