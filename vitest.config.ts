import { defineConfig } from "vitest/config";

/**
 * Vitest configuration for the workspace root.
 *
 * The root `test:unit` script exists as a convenience, but the root has no tests of
 * its own: with no config, Vitest walks the whole monorepo and collects the
 * Playwright specs under `apps/desktop/e2e/`, then dies with
 *
 *   Error: Playwright Test did not expect test.describe() to be called here.
 *
 * which is a failure about the runner, not about the product. The desktop package
 * already solves this in its own `vitest.config.ts`; the root needs the same
 * boundary. The E2E specs are executed by Playwright in `test-e2e.sh`, so excluding
 * them here removes a false failure without removing any test from the suite: the
 * unit tests of every package are still collected (and `pnpm -r test:unit`, the lane
 * `scripts/test-unit.sh` runs, is unaffected because each package resolves its own
 * config).
 */
export default defineConfig({
  test: {
    include: ["**/tests/**/*.{test,spec}.{ts,tsx}"],
    exclude: ["**/node_modules/**", "**/dist/**", "**/src-tauri/**", "**/e2e/**"],
  },
});
