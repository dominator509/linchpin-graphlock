import { expect, test } from "@playwright/test";

/**
 * Real browser E2E for the LINCHPIN desktop UI.
 *
 * These run against the production bundle served over loopback — the same
 * artifact the packaged app loads — in a real browser engine. They are the
 * first automated evidence for REQ-UI-001 (navigation surface), REQ-UI-003
 * (legal-risk language) and REQ-UI-002 (confidentiality state).
 *
 * Before ADR-004 this repository had zero JavaScript tests and the E2E gate
 * exited 1 on a missing script.
 */

test.describe("LINCHPIN desktop shell", () => {
  test("loads and renders the product heading", async ({ page }) => {
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

    // Accessibility baseline (REQ-A11Y-001, WCAG 2.2 AA): a page must declare
    // its language and title. Manual AT validation remains EXTERNAL_REQUIRED.
    await expect(page).toHaveTitle(/LINCHPIN/i);

    const lang = await page.locator("html").getAttribute("lang");
    expect(lang, "html lang attribute must be set").toBeTruthy();
    expect(lang!.toLowerCase().startsWith("en")).toBe(true);
  });

  test("mounts a single root element without duplicate rendering", async ({
    page,
  }) => {
    await page.goto("/");

    // A StrictMode double-render bug or a duplicated mount would show up here.
    const mains = await page.locator("main").count();
    expect(mains, "expected exactly one <main> landmark").toBe(1);
  });
});
