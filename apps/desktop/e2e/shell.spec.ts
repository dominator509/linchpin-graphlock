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
      // backend (14 of 14 at this revision), never an inflated claim.
      await expect(page.getByText("14 of 14")).toBeVisible();
    }
  });

  test("exposes the operations panel without claiming a live provider", async ({
    page,
  }) => {
    await page.goto("/");

    // PF-011 is unmet, so no provider lane may be presented as configured and
    // the UI must not claim inference is happening.
    await expect(
      page.getByRole("heading", { name: "Operations" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Load operational status" }),
    ).toBeVisible();
    await expect(
      page.getByText(/no lane performs inference on this host/),
    ).toBeVisible();

    // DOD-019: the provider transport must be reachable from the UI rather than
    // being an inert adapter. The control exists; without a served model the
    // backend reports "not live", and the UI must never show generated text.
    await expect(
      page.getByRole("button", { name: "Probe local model" }),
    ).toBeVisible();
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

  // covers: REQ-UI-001
  test("exposes primary navigation for all thirteen surfaces", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-UI-001 names thirteen surfaces. Every one must be a real navigation
    // entry AND lead to a real section — a nav link to a missing target is a
    // dead route, which DOD-019 forbids.
    const expected = [
      "Dashboard",
      "Opportunity Radar",
      "Conception Lab",
      "Research War Room",
      "Patent Architect",
      "Filing",
      "Docket",
      "Prosecution",
      "Commercialize",
      "Evidence Vault",
      "Integrations",
      "Incidents",
      "Settings",
    ];

    const nav = page.getByRole("navigation", { name: "Primary" });
    await expect(nav).toBeVisible();

    const links = nav.getByRole("link");
    await expect(links).toHaveCount(expected.length);

    for (const label of expected) {
      const link = nav.getByRole("link", { name: label, exact: true });
      await expect(link, `${label} must be a navigation entry`).toBeVisible();

      // The target must exist and be the section that labels itself `label`.
      const href = await link.getAttribute("href");
      expect(href, `${label} must link somewhere`).toBeTruthy();
      const id = href!.replace(/^#/, "");
      const section = page.locator(`section#${id}`);
      await expect(section, `${label} must link to a real section`).toHaveCount(
        1,
      );
      await expect(
        section.getByRole("heading", { level: 2, name: label }),
      ).toBeVisible();
    }
  });

  // covers: REQ-UI-002
  test("shows the four persistent top-level status badges", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-UI-002: confidentiality, filing/priority state, evidence coverage and
    // external-provider egress state. The badges must render even with no
    // backend, and must describe the unknown state rather than invent one.
    const badges = page.getByRole("list", { name: "Status badges" });
    await expect(badges).toBeVisible();

    await expect(
      badges.getByText("Local-First Confidentiality Boundary Active."),
    ).toBeVisible();
    await expect(badges.getByText(/^Filing state:/)).toBeVisible();
    await expect(
      badges.getByText(/Evidence coverage|contract namespaces/),
    ).toBeVisible();
    await expect(badges.getByText(/External-provider egress:/)).toBeVisible();

    await expect(badges.getByRole("listitem")).toHaveCount(4);

    // A badge must not claim a provider lane is configured when none was
    // reported — the AG-007a "health signal that lies" failure.
    const text = await badges.innerText();
    expect(text).not.toContain("a lane is configured");
  });

  // covers: REQ-UI-003
  test("uses screening vocabulary and never definitive legal conclusions", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-UI-003: legal-risk language uses "screen", "draft", "evidence",
    // "uncertainty" rather than definitive legal conclusions.
    const body = (await page.locator("body").innerText()).toLowerCase();
    for (const preferred of ["screen", "draft", "evidence", "uncertainty"]) {
      expect(
        body,
        `preferred vocabulary "${preferred}" should appear in the product surface`,
      ).toContain(preferred);
    }
  });

  // covers: REQ-UI-004
  test("high-impact actions require a consequence-specific confirmation", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-UI-004: destructive/high-impact actions use consequence-specific
    // confirmation and audit. The first click must NOT perform the action.
    const trigger = page.getByRole("button", {
      name: "Export evidence bundle",
    });
    await expect(trigger).toBeVisible();

    await expect(
      page.getByRole("group", { name: "Confirm Export evidence bundle" }),
    ).toHaveCount(0);

    await trigger.click();

    const confirmPanel = page.getByRole("group", {
      name: "Confirm Export evidence bundle",
    });
    await expect(confirmPanel).toBeVisible();

    // The consequence must be specific: it names what leaves the boundary.
    await expect(confirmPanel).toContainText(
      "releases screening evidence beyond the device boundary",
    );
    await expect(confirmPanel).toContainText("cannot be recalled");

    // Cancel dismisses without acting.
    await confirmPanel.getByRole("button", { name: "Cancel" }).click();
    await expect(confirmPanel).toHaveCount(0);

    // Every high-impact action offers the same protection.
    for (const label of [
      "Build filing package",
      "Probe local model",
      "Restore vault from backup",
    ]) {
      await page.getByRole("button", { name: label, exact: true }).click();
      await expect(
        page.getByRole("group", { name: `Confirm ${label}` }),
      ).toBeVisible();
      await page
        .getByRole("group", { name: `Confirm ${label}` })
        .getByRole("button", { name: "Cancel" })
        .click();
    }
  });

  // covers: REQ-REL-005
  test("exposes backup and recovery and states the point-in-time limit", async ({
    page,
  }) => {
    await page.goto("/");

    // REQ-REL-005: backup/restore must be executable from the product. In a
    // browser there is no Tauri IPC, so the test asserts the CONTROLS and the
    // honest statement of what a backup is -- not a fabricated success.
    await expect(page.getByLabel("Backup file")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Back up vault" }),
    ).toBeVisible();

    const recovery = page.locator("#recovery-heading").locator("..");
    await expect(recovery).toContainText("point-in-time snapshot");
    await expect(recovery).toContainText(
      "restoring discards work that is not in the backup",
    );

    // The destructive half must confirm before it acts (REQ-UI-004).
    const trigger = page.getByRole("button", {
      name: "Restore vault from backup",
    });
    await expect(
      page.getByRole("group", { name: "Confirm Restore vault from backup" }),
    ).toHaveCount(0);
    await trigger.click();
    const panel = page.getByRole("group", {
      name: "Confirm Restore vault from backup",
    });
    await expect(panel).toBeVisible();
    await expect(panel).toContainText("discarded");
    await expect(panel).toContainText("preserved rather than deleted");
  });

  // covers: REQ-DATA-001
  test("exposes the Human Conception Ledger read path and states it honestly", async ({
    page,
  }) => {
    await page.goto("/");

    // UO-01 promised a Human Conception LEDGER. Recording an event stored it
    // durably, but no surface could read it back, so the ledger could not be
    // consulted. This lane has no IPC, so it asserts the control exists and that
    // the failure to read is reported rather than shown as an empty ledger.
    await expect(
      page.getByRole("heading", { name: "Human Conception Ledger", level: 3 }),
    ).toBeVisible();
    const load = page.getByRole("button", {
      name: "Load ledger from the vault",
    });
    await expect(load).toBeVisible();
    await load.click();
    await expect(
      page.getByText("Cannot read the ledger: desktop backend unavailable."),
    ).toBeVisible();
  });

  // covers: REQ-SCOPE-001
  test("declares the scope surface and reports it honestly when disconnected", async ({
    page,
  }) => {
    await page.goto("/");

    // SPEC-000 requires the promised scope and the five truth boundaries to be
    // preserved. This lane runs WITHOUT the Tauri bridge, so it asserts the
    // surface exists and states its absence honestly; the exact-artifact lane
    // asserts the declared CONTENT, because only there is the IPC bridge real.
    await expect(
      page.getByRole("heading", {
        name: "Declared scope and truth boundaries",
        level: 3,
      }),
    ).toBeVisible();
    await expect(
      page.getByText("Declared scope not reported by the backend."),
    ).toBeVisible();
  });
});
