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

  test("exposes the research kill-search controls", async ({ page }) => {
    await page.goto("/");

    // REQ-RES-002: the kill-search lifecycle must be operable from the UI.
    await expect(
      page.getByRole("heading", { name: "Research Kill-Search" }),
    ).toBeVisible();
    await expect(page.getByLabel("Task ID")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Start search" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Record kill" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Complete search" }),
    ).toBeVisible();
  });

  test("exposes the Disclosure Firewall sensitivity control", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-DOM-008 / REQ-COM-004: public export must be gated by the firewall,
    // and the UI must let the operator set the sensitivity.
    await expect(
      page.getByRole("heading", { name: "Disclosure Firewall" }),
    ).toBeVisible();

    const sensitivity = page.getByLabel("Export sensitivity");
    await expect(sensitivity).toBeVisible();
    await expect(sensitivity.locator("option")).toHaveCount(3);
    await expect(
      page.getByRole("button", { name: "Evaluate export" }),
    ).toBeVisible();
  });

  test("reports capability coverage without over-claiming", async ({
    page,
  }) => {
    await page.goto("/");

    // Outside Tauri the namespace list cannot load, so the coverage section
    // must be absent rather than showing a fabricated "14 of 14".
    const body = await page.locator("body").innerText();
    expect(body).not.toContain("14 of 14");

    if ((await page.getByText("Capability Coverage").count()) > 0) {
      // If it did render, it must match the honest count reported by the
      // backend (6 of 14 at this revision), never a full-coverage claim.
      await expect(page.getByText("6 of 14")).toBeVisible();
    }
  });

  test("exposes the Patent Architect claim linter", async ({ page }) => {
    await page.goto("/");

    // REQ-PAT-001: claims must be lintable from the UI.
    await expect(
      page.getByRole("heading", { name: "Patent Architect" }),
    ).toBeVisible();
    await expect(page.getByLabel("Claims")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Lint claims" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Build filing package" }),
    ).toBeVisible();
  });

  test("exposes filing receipt import and states submission is manual", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-PAT-005: receipt import exists, and the UI must never imply that
    // LINCHPIN signs, pays or submits on the user's behalf.
    await expect(page.getByRole("heading", { name: "Filing" })).toBeVisible();
    await expect(page.getByLabel("Acknowledgement receipt")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Import receipt" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Check handoff readiness" }),
    ).toBeVisible();

    await expect(
      page.getByText(
        "Human submission only: LINCHPIN does not sign, pay or submit.",
      ),
    ).toBeVisible();
  });
});
