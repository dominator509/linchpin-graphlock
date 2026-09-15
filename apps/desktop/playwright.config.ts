import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright configuration for LINCHPIN desktop UI verification.
 *
 * GraphLock context (ADR-004): this project had NO JavaScript tests at all and
 * `scripts/test-e2e.sh` failed with ERR_PNPM_RECURSIVE_RUN_NO_SCRIPT because no
 * package declared a `test:e2e` script. The desktop UI is the only user-facing
 * surface in the product, and it was entirely unverified.
 *
 * Design decisions:
 *
 * 1. `channel: "msedge"` / fallback to the system Chrome. The bundled browser
 *    download is deliberately NOT used, both to avoid a ~500 MB fetch and to
 *    avoid redistributing a browser build (ADR-004).
 * 2. The suite runs against the REAL built bundle served over loopback, not a
 *    mocked component tree. A mock would prove the test author's expectation,
 *    not the product (DOD-010).
 * 3. `reporter: "list"` keeps output machine-readable for the evidence log.
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  // `list` keeps the console output readable; `json` writes the machine-readable
  // report that scripts/bind-requirements.py reads to decide whether an
  // E2E-bound requirement was actually COLLECTED and whether it PASSED. Without
  // it, requirements bound to Playwright tests are reported BOUND_NOT_COLLECTED
  // or PARTIAL even though the tests ran and passed -- a false negative in the
  // traceability accounting.
  //
  // Written under .agent/state/ rather than into this workspace: it is GraphLock
  // run evidence, and generating it inside apps/desktop made
  // scripts/format-check.sh fail on a generated file instead of on authored code.
  reporter: [
    ["list"],
    ["json", { outputFile: "../../.agent/state/e2e-report.json" }],
  ],
  timeout: 30_000,
  expect: { timeout: 5_000 },
  use: {
    // NOTE: `vite preview` binds IPv6 loopback (`::1`) only on this host, so
    // `127.0.0.1` is refused. Using `localhost` resolves correctly for both
    // families and was verified to return HTTP 200. Binding IPv4 explicitly
    // would require `--host 127.0.0.1`, which changes the served origin.
    baseURL: "http://localhost:4173",
    trace: "off",
    screenshot: "off",
    video: "off",
  },
  projects: [
    {
      name: "msedge",
      use: { ...devices["Desktop Chrome"], channel: "msedge" },
    },
  ],
  webServer: {
    // Serve the production bundle exactly as the packaged app does.
    command: "npx vite preview --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
