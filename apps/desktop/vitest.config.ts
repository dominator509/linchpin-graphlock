import { defineConfig } from "vitest/config";

/**
 * Vitest configuration for the desktop package.
 *
 * The E2E specs under `e2e/` are Playwright tests, not Vitest tests. Without an
 * explicit exclude, Vitest tries to collect them and fails with
 * "Playwright Test did not expect test.describe() to be called here", which
 * makes `test:unit` fail for the wrong reason.
 */
export default defineConfig({
  test: {
    include: ["tests/**/*.{test,spec}.{ts,tsx}"],
    exclude: ["e2e/**", "node_modules/**", "dist/**", "src-tauri/**"],
  },
});
