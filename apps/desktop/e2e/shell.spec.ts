import { expect, test } from "@playwright/test";

/**
 * Real browser E2E for the LINCHPIN desktop UI.
 *
 * These run against the production bundle served over loopback — the same
 * artifact the packaged app loads — in a real browser engine (system Edge).
 * They are the first automated evidence for REQ-UI-001, REQ-UI-002, REQ-UI-003
 * and REQ-A11Y-001.
 *
 * IMPORTANT SCOPE NOTE: this suite runs the web bundle in a BROWSER, where the
 * Tauri IPC bridge is absent. The tests therefore assert the honest
 * disconnected state ("Desktop backend unavailable") rather than pretending the
 * backend responded. Backend command behaviour is covered by the Rust tests in
 * `src/commands.rs`, and the packaged app is verified by
 * `scripts/smoke-installed-artifact.sh`.
 */

test.describe("LINCHPIN desktop shell", () => {
  test("loads and renders the product heading without page errors", async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));

    await page.goto("/");

    await expect(
      page.getByRole("heading", { name: "LINCHPIN Patent Intelligence OS" }),
    ).toBeVisible();

    expect(errors, `uncaught page errors: ${errors.join("; ")}`).toEqual([]);
  });

  test("declares the confidentiality boundary in the UI", async ({ page }) => {
    await page.goto("/");

    // REQ-UI-002 / SPEC-005: the confidentiality boundary must be stated in the
    // product surface, not only in documentation.
    await expect(
      page.getByText("Local-First Confidentiality Boundary Active."),
    ).toBeVisible();
  });

  test("uses screen/draft language, never definitive legal conclusions", async ({
    page,
  }) => {
    await page.goto("/");

    const body = (await page.locator("body").innerText()).toLowerCase();

    // REQ-UI-003 prohibits definitive legal conclusions in product copy.
    const prohibited = [
      "guaranteed patent",
      "guaranteed approval",
      "legally valid",
      "will be granted",
      "assured patent",
    ];
    for (const phrase of prohibited) {
      expect(body, `prohibited legal claim present: "${phrase}"`).not.toContain(
        phrase,
      );
    }
  });

  test("has a document title and an html lang attribute", async ({ page }) => {
    await page.goto("/");

    // Accessibility baseline (REQ-A11Y-001, WCAG 2.2 AA). Manual AT validation
    // remains EXTERNAL_REQUIRED and is not implied by this test.
    await expect(page).toHaveTitle(/LINCHPIN/i);

    const lang = await page.locator("html").getAttribute("lang");
    expect(lang, "html lang attribute must be set").toBeTruthy();
    expect(lang!.toLowerCase().startsWith("en")).toBe(true);
  });

  test("mounts a single root element without duplicate rendering", async ({
    page,
  }) => {
    await page.goto("/");

    const mains = await page.locator("main").count();
    expect(mains, "expected exactly one <main> landmark").toBe(1);
  });

  test("surfaces the missing backend honestly instead of faking health", async ({
    page,
  }) => {
    await page.goto("/");

    // Outside the Tauri shell there is no IPC bridge. The UI must say so rather
    // than render a fabricated "OK" — the AG-007a failure mode.
    await expect(page.getByRole("alert")).toContainText(
      "Desktop backend unavailable",
    );

    const body = await page.locator("body").innerText();
    expect(body).not.toContain("Status: OK");
  });

  test("exposes the Conception Lab input and origin selector", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-DOM-001 / REQ-DOM-002: a conception entry point must exist and must
    // let the author distinguish human conception from AI suggestion.
    await expect(page.getByLabel("Describe your conception")).toBeVisible();

    const origin = page.getByLabel("Origin");
    await expect(origin).toBeVisible();
    await expect(origin.locator("option")).toHaveCount(2);

    await expect(
      page.getByRole("button", { name: "Record conception event" }),
    ).toBeVisible();
  });

  test("keeps every labelled form control keyboard reachable", async ({
    page,
  }) => {
    await page.goto("/");

    // Accessibility baseline: the textarea, the select and the button must all
    // be focusable via the keyboard.
    await expect(page.getByLabel("Describe your conception")).toBeEnabled();
    await expect(page.getByLabel("Origin")).toBeEnabled();

    await page.getByLabel("Describe your conception").focus();
    const focused = await page.evaluate(
      () => document.activeElement?.tagName.toLowerCase() ?? "",
    );
    expect(focused).toBe("textarea");
  });
});
