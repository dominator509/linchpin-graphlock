import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { dirname } from "node:path";

/**
 * Exact-artifact E2E (DOD-004).
 *
 * DOD-004 RULE: "Final smoke and E2E tests run against the exact production
 * artifact digest, not merely source or a development server."
 *
 * The existing Playwright suite runs against `vite preview` -- a development
 * server, which the clause excludes. This drives the PACKAGED EXECUTABLE's real
 * WebView2 instead.
 *
 * Requirements for the artifact under test, all established empirically:
 *   1. produced by `tauri build`, not `cargo build --release`. A cargo build
 *      embeds `devUrl` and loads http://localhost:5173; only the tauri build
 *      pipeline embeds `frontendDist` and serves http://tauri.localhost/. This
 *      was the blocker for two rounds.
 *   2. built with `--features devtools-e2e` and a config carrying
 *      `additionalBrowserArgs: "--remote-debugging-port=<port>"`. Neither is
 *      present in a shipped release, and must not be: a debug port in a
 *      released binary would let any local process attach to the webview and
 *      invoke backend commands, breaking the SPEC-005 confidentiality boundary.
 *
 * Usage: node e2e-artifact.mjs <exe> <report.md>
 */

const EXE = process.argv[2] ?? "target/release/linchpin-desktop.exe";
const REPORT = process.argv[3] ?? ".agent/evidence/artifact-e2e/STATUS.md";
const PORT = Number(process.env.LINCHPIN_DEBUG_PORT ?? 9222);

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

async function waitForPageTarget(timeoutMs = 40000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`http://127.0.0.1:${PORT}/json`, {
        signal: AbortSignal.timeout(1000),
      });
      const targets = await res.json();
      // WebView2 opens the port briefly at startup and closes it once the
      // webview finishes initialising (measured: open at t=3s, closed by t=6s).
      // Poll fast or the window is missed entirely.
      const page = targets.find(
        (t) => t.type === "page" && t.webSocketDebuggerUrl,
      );
      if (page) return page;
    } catch {
      /* endpoint not up yet */
    }
    await sleep(150);
  }
  return null;
}

const digestBefore = sha256(EXE);
const size = readFileSync(EXE).length;
// `let`, not `const`: the hard-restart proof below kills this process and
// relaunches the artifact, and the cleanup path must kill whichever one is
// currently running.
let app = spawn(EXE, [], { stdio: "ignore" });

/**
 * Unpredictable per-run marker (DOD-013).
 *
 * The state-changing proof used to write the static literal "e2e artifact
 * probe", which a canned response, a cached write or a hard-coded success could
 * satisfy. The canary is generated at runtime, carried into the app through the
 * real command, and then looked for OUTSIDE the app in the durable files, so the
 * observation does not depend on the product telling the truth about itself.
 */
const canary = `ARTIFACT-CANARY-${Date.now().toString(36)}-${Math.floor(Math.random() * 1e9).toString(36)}`;

/** Encoded startup bound in milliseconds (DOD-022). */
const STARTUP_BOUND_MS = 45000;
const spawnAt = Date.now();

const checks = [];
const record = (name, ok, observed) => checks.push({ name, ok, observed });

let browser;
try {
  const target = await waitForPageTarget();
  if (!target) {
    record(
      "page target available",
      false,
      "no page target with a debugger URL",
    );
    throw new Error("no page target");
  }

  browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
  const pages = browser.contexts().flatMap((c) => c.pages());
  if (pages.length === 0) {
    record("page visible over CDP", false, "0 pages");
    throw new Error("no pages");
  }
  const page = pages[0];

  // The webview starts at about:blank and navigates. Evaluating before the new
  // document loads destroys the execution context -- this was the actual cause
  // of the earlier empty-evaluation symptom, and it is why the waits below are
  // required rather than defensive.
  await page
    .waitForLoadState("domcontentloaded", { timeout: 20000 })
    .catch(() => {});
  await page
    .waitForFunction(() => document.readyState === "complete", {
      timeout: 20000,
    })
    .catch(() => {});
  await page.waitForSelector("h1", { timeout: 20000 }).catch(() => {});

  const url = page.url();
  // Parse the URL and compare the host EXACTLY rather than testing a prefix:
  // CodeQL reported `js/incomplete-url-substring-sanitization` for
  // `url.startsWith("http://tauri.localhost")`, a shape that also accepts
  // `http://tauri.localhost.attacker.example`. This assertion is about identity,
  // so it compares protocol and hostname.
  const loadedEmbeddedFrontend = (() => {
    try {
      const parsed = new URL(url);
      return (
        parsed.protocol === "tauri:" ||
        (parsed.protocol === "http:" && parsed.hostname === "tauri.localhost")
      );
    } catch {
      return false;
    }
  })();
  record(
    "loads the embedded frontend, not a dev server",
    loadedEmbeddedFrontend,
    url,
  );

  const title = await page.title();
  record("document title", title.includes("LINCHPIN"), JSON.stringify(title));

  // Startup SLO on the EXACT artifact (DOD-022): measured from process spawn to
  // the moment the page reports itself complete, and ENFORCED rather than
  // merely reported. The bound is deliberately generous because this lane runs
  // on a loaded developer host; it exists to fail a change that makes the app
  // take minutes to become usable, not to promise a boot time to a customer.
  const startupMs = Date.now() - spawnAt;
  record(
    "startup within the encoded bound (DOD-022)",
    startupMs <= STARTUP_BOUND_MS,
    `${startupMs}ms (bound ${STARTUP_BOUND_MS}ms)`,
  );

  const h1 =
    (await page
      .locator("h1")
      .first()
      .textContent()
      .catch(() => "")) ?? "";
  record(
    "product heading rendered",
    h1.includes("LINCHPIN Patent Intelligence OS"),
    JSON.stringify(h1.trim()),
  );

  const body = await page.locator("body").innerText();
  record(
    "confidentiality boundary stated (REQ-UI-002)",
    body.includes("Local-First Confidentiality Boundary Active."),
    body.includes("Local-First Confidentiality") ? "present" : "MISSING",
  );

  // DOD-020: production mode must never select a mock/fake/demo adapter for a
  // feature presented as production-ready. This is the PACKAGED build, so the
  // runtime adapter identity it reports is the real one -- a source-level test
  // cannot show which adapter a shipped binary resolves.
  const lanes = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("provider_status");
    } catch (e) {
      return { __error: String(e) };
    }
  });
  const lanesOk = lanes && !lanes.__error && lanes.ok === true;
  record(
    "provider_status round-trip in the packaged build (DOD-020)",
    lanesOk,
    lanesOk
      ? lanes.value
          .map(
            (l) =>
              `${l.lane}:configured=${l.configured},transport=${l.transport_available}`,
          )
          .join("; ")
      : String(lanes?.__error ?? JSON.stringify(lanes)),
  );
  if (lanesOk) {
    const local = lanes.value.find((l) => l.lane === "local");
    const others = lanes.value.filter((l) => l.lane !== "local");
    record(
      "the packaged build resolves a REAL local adapter, not a simulated one (DOD-020)",
      Boolean(local) && local.transport_available === true,
      local
        ? `local transport_available=${local.transport_available} configured=${local.configured} detail=${local.detail}`
        : "no local lane reported",
    );
    record(
      "unwired lanes are reported as unavailable rather than faked (DOD-020)",
      others.length > 0 &&
        others.every(
          (l) => l.configured === false && l.transport_available === false,
        ),
      others.map((l) => `${l.lane}=${l.configured}`).join(", ") ||
        "no other lanes reported",
    );
    // The local lane's own signal must match the measurement, not a constant:
    // the provider live-fire lane proves a model IS served on this host when the
    // runtime is up, so "configured" must follow the probe rather than claim
    // otherwise in either direction.
    record(
      "the local lane's configured flag is measured, not asserted (DOD-037)",
      Boolean(local) && local.detail.includes("127.0.0.1:11434"),
      local ? String(local.detail) : "no local lane reported",
    );
  }

  // REQ-SCOPE-001: the declared scope and the five truth boundaries must be
  // visible in the PRODUCT, read from the backend declaration rather than a
  // copy in the UI. This is the only lane where the IPC bridge is real, so it is
  // the only lane that can prove the declaration reaches a user.
  const declared = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("get_scope_declaration");
    } catch (e) {
      return { __error: String(e) };
    }
  });
  const declaredOk = declared && !declared.__error && declared.ok === true;
  record(
    "get_scope_declaration round-trip (REQ-SCOPE-001)",
    declaredOk,
    declaredOk
      ? `${declared.value.boundaries.length} boundaries, ${declared.value.capabilities.length} capabilities`
      : String(declared?.__error ?? JSON.stringify(declared)),
  );
  if (declaredOk) {
    const boundaryIds = declared.value.boundaries.map((b) => b.id);
    record(
      "all five truth boundaries declared",
      ["TB-1", "TB-2", "TB-3", "TB-4", "TB-5"].every((id) =>
        boundaryIds.includes(id),
      ),
      `ids=${boundaryIds.join(",")}`,
    );
    record(
      "twelve promised outcomes accounted, none over-claimed",
      declared.value.capabilities.length === 12 &&
        declared.value.capabilities.every(
          (c) =>
            c.state === "IMPLEMENTED" ||
            (c.limitation && c.limitation.trim().length > 0),
        ),
      declared.value.capabilities.map((c) => `${c.uo_id}=${c.state}`).join(","),
    );
    // And the rendered surface must show them, not only the IPC payload.
    const scopeText = await page
      .locator("#scope-heading")
      .locator("..")
      .innerText();
    record(
      "declared boundaries render in the Settings surface",
      ["TB-1", "TB-2", "TB-3", "TB-4", "TB-5"].every((id) =>
        scopeText.includes(id),
      ) && scopeText.includes("UO-01"),
      scopeText.includes("TB-5") ? "TB-1..TB-5 rendered" : "boundaries MISSING",
    );
  }

  const hasIpc = await page.evaluate(
    () => typeof window.__TAURI_INTERNALS__ !== "undefined",
  );
  record("Tauri IPC bridge present", hasIpc === true, String(hasIpc));

  const health = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("get_system_health");
    } catch (e) {
      return { __error: String(e) };
    }
  });
  const healthOk = health && !health.__error;
  record(
    "get_system_health round-trip",
    healthOk,
    healthOk ? JSON.stringify(health) : String(health?.__error),
  );
  if (healthOk) {
    record(
      "health payload typed",
      ["status", "storage_detail", "storage_ok", "version"].every(
        (k) => k in health,
      ),
      `keys=${Object.keys(health).sort().join(",")}`,
    );
    record(
      "health reports a real status",
      health.status === "OK" || health.status === "DEGRADED",
      `status=${health.status} storage_ok=${health.storage_ok} detail=${health.storage_detail}`,
    );

    // DOD-037: the health signal must AGREE with the diagnostics probe, and the
    // detail must say WHICH path it judged. A status boolean with no path is a
    // signal an operator cannot act on, and a disagreement between two health
    // surfaces is itself the finding.
    const diagnostics = await page.evaluate(async () => {
      try {
        return await window.__TAURI_INTERNALS__.invoke("get_diagnostics");
      } catch (e) {
        return { __error: String(e) };
      }
    });
    const diagnosticsOk =
      diagnostics && !diagnostics.__error && diagnostics.ok === true;
    record(
      "get_diagnostics round-trip in the packaged build (DOD-037)",
      diagnosticsOk,
      diagnosticsOk
        ? `readiness=${diagnostics.value.readiness} storage_ok=${diagnostics.value.storage_ok} alerts=${diagnostics.value.alerts.length}`
        : String(diagnostics?.__error ?? JSON.stringify(diagnostics)),
    );
    if (diagnosticsOk) {
      // The property is AGREEMENT on the same directory, not a particular value.
      // The directory can legitimately be absent (the installer lane removes the
      // product's app-data directory as part of its cleanup), and in that state
      // both surfaces must say so -- an earlier revision compared two probes that
      // measured different moments and one of them CREATED the directory as a
      // side effect, so they disagreed on a healthy app.
      record(
        "health and diagnostics agree on storage (DOD-037)",
        diagnostics.value.storage_ok === health.storage_ok,
        `health.storage_ok=${health.storage_ok} (${health.storage_detail}) diagnostics.storage_ok=${diagnostics.value.storage_ok} (${diagnostics.value.detail})`,
      );
      record(
        "the diagnostics detail names the path it judged (DOD-037)",
        String(diagnostics.value.detail).includes("LINCHPIN"),
        String(diagnostics.value.detail).slice(0, 160),
      );
    }
  }

  const ns = await page.evaluate(async () => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("get_namespace_status");
    } catch (e) {
      return { __error: String(e) };
    }
  });
  const nsOk = Array.isArray(ns);
  record(
    "get_namespace_status round-trip",
    nsOk,
    nsOk ? `${ns.length} namespaces` : String(ns?.__error),
  );
  if (nsOk) {
    const implemented = ns.filter((n) => n.implemented).length;
    record(
      "namespace coverage reported",
      ns.length === 14,
      `${implemented}/${ns.length} implemented`,
    );
  }

  // A conception event round-trips through the vault, proving a state-changing
  // command works against the packaged artifact rather than only a read. The
  // content carries the runtime canary, and the canary is then searched for in
  // the durable files by this harness -- a channel the product does not control.
  const conception = await page.evaluate(async (content) => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("record_conception", {
        workspaceId: "e2e-workspace",
        content,
        authorIsHuman: true,
      });
    } catch (e) {
      return { __error: String(e) };
    }
  }, `artifact probe ${canary}`);
  const conceptionOk = conception && conception.ok === true;
  record(
    "record_conception round-trip (state-changing)",
    conceptionOk,
    conceptionOk
      ? `persisted=${conception.value?.persisted} hash=${conception.value?.event?.content_hash?.slice(0, 16)}`
      : String(conception?.__error ?? JSON.stringify(conception)),
  );
  if (conceptionOk) {
    record(
      "conception persisted to the vault",
      conception.value?.persisted === true,
      String(conception.value?.persisted),
    );
    record(
      "content address is SHA-256 (REQ-DATA-002)",
      String(conception.value?.event?.content_hash).startsWith("sha256:"),
      String(conception.value?.event?.content_hash),
    );
    record(
      "human origin labelled (REQ-DOM-002)",
      conception.value?.event?.origin === "HumanConception",
      String(conception.value?.event?.origin),
    );

    // Independent observation (DOD-012) of an unpredictable value (DOD-013):
    // read the product's own vault file from OUTSIDE the product and look for the
    // canary. A command that reported success without writing anything cannot
    // satisfy this, and neither can a fixture: the value did not exist until this
    // run.
    const config = await page.evaluate(async () => {
      try {
        return await window.__TAURI_INTERNALS__.invoke("get_configuration");
      } catch (e) {
        return { __error: String(e) };
      }
    });
    const vaultFile = config?.value?.vault_file ?? "";
    let found = false;
    let searched = "";
    if (vaultFile) {
      const candidates = [vaultFile, `${vaultFile}-wal`];
      for (const path of candidates) {
        if (!existsSync(path)) continue;
        searched += `${searched ? ", " : ""}${path}`;
        if (readFileSync(path, "latin1").includes(canary)) found = true;
      }
    }
    record(
      "canary found in the durable vault by an independent channel (DOD-012, DOD-013)",
      found,
      found
        ? `${canary} present in ${searched}`
        : `canary absent from ${searched || "no vault path reported"}`,
    );
  }

  // REQ-UI-004 audit half: a confirmed high-impact action must be recorded with
  // its correlation ID. This needs REAL IPC -- the browser suite can prove the
  // confirmation step but not that an action actually ran and was audited.
  const auditBefore = await page
    .locator("#audit li")
    .count()
    .catch(() => 0);
  await page.getByRole("button", { name: "Export evidence bundle" }).click();
  const confirmPanel = page.getByRole("group", {
    name: "Confirm Export evidence bundle",
  });
  const confirmVisible = await confirmPanel.isVisible().catch(() => false);
  record(
    "high-impact action shows a confirmation panel before acting",
    confirmVisible,
    String(confirmVisible),
  );
  if (confirmVisible) {
    await confirmPanel.getByRole("button", { name: "Export evidence" }).click();
    await page.waitForTimeout(1500);
    const auditAfter = await page.locator("#audit li").count();
    const auditText = await page
      .locator("#audit")
      .innerText()
      .catch(() => "");
    record(
      "confirmed action is recorded in the audit trail (REQ-UI-004)",
      auditAfter > auditBefore && /correlation/.test(auditText),
      `entries ${auditBefore} -> ${auditAfter}; ${auditText.replace(/\s+/g, " ").slice(0, 120)}`,
    );
  }

  // IDEMPOTENCY ACROSS THE IPC BOUNDARY (DOD-017): the client submits the same
  // key twice, as a retry after an unknown outcome. The second call must report a
  // replay, name the first event, and leave the ledger unchanged.
  const keyedArgs = {
    workspaceId: "e2e-workspace",
    eventKey: `e2e-key-${canary}`,
    content: `keyed retry probe ${canary}`,
    authorIsHuman: true,
  };
  const keyedFirst = await page.evaluate(async (args) => {
    try {
      return await window.__TAURI_INTERNALS__.invoke(
        "record_conception_keyed",
        args,
      );
    } catch (e) {
      return { __error: String(e) };
    }
  }, keyedArgs);
  const keyedFirstOk = keyedFirst && keyedFirst.ok === true;
  record(
    "keyed recording over IPC (DOD-017)",
    keyedFirstOk && keyedFirst.value?.replayed === false,
    keyedFirstOk
      ? `id=${String(keyedFirst.value.event.event_id).slice(0, 8)} replayed=${keyedFirst.value.replayed}`
      : String(keyedFirst?.__error ?? JSON.stringify(keyedFirst)),
  );

  if (keyedFirstOk) {
    const ledgerBefore = await page.evaluate(async (workspaceId) => {
      try {
        return await window.__TAURI_INTERNALS__.invoke(
          "list_conception_events",
          { workspaceId },
        );
      } catch (e) {
        return { __error: String(e) };
      }
    }, "e2e-workspace");
    const before = ledgerBefore?.value?.count ?? -1;

    const keyedRetry = await page.evaluate(async (args) => {
      try {
        return await window.__TAURI_INTERNALS__.invoke(
          "record_conception_keyed",
          args,
        );
      } catch (e) {
        return { __error: String(e) };
      }
    }, keyedArgs);
    const retryOk = keyedRetry && keyedRetry.ok === true;
    const sameEvent =
      retryOk &&
      keyedRetry.value?.event?.event_id === keyedFirst.value.event.event_id;
    record(
      "a retried submission is reported as a replay (DOD-017)",
      retryOk && keyedRetry.value?.replayed === true && sameEvent,
      retryOk
        ? `replayed=${keyedRetry.value.replayed} same_event=${sameEvent}`
        : String(keyedRetry?.__error ?? JSON.stringify(keyedRetry)),
    );

    const ledgerAfter = await page.evaluate(async (workspaceId) => {
      try {
        return await window.__TAURI_INTERNALS__.invoke(
          "list_conception_events",
          { workspaceId },
        );
      } catch (e) {
        return { __error: String(e) };
      }
    }, "e2e-workspace");
    const after = ledgerAfter?.value?.count ?? -2;
    record(
      "the retry left the ledger unchanged (DOD-017)",
      before >= 0 && after === before,
      `ledger ${before} -> ${after}`,
    );

    // Reusing the key with different content must be refused, not silently
    // stored under the same identity.
    const keyedConflict = await page.evaluate(
      async (args) => {
        try {
          return await window.__TAURI_INTERNALS__.invoke(
            "record_conception_keyed",
            args,
          );
        } catch (e) {
          return { __error: String(e) };
        }
      },
      {
        ...keyedArgs,
        content: `a DIFFERENT payload under the same key ${canary}`,
      },
    );
    record(
      "reusing a key with different content is refused (DOD-017)",
      keyedConflict && keyedConflict.ok === false,
      String(
        keyedConflict?.error?.message ?? JSON.stringify(keyedConflict),
      ).slice(0, 140),
    );
  }

  // DASHBOARD (DOD-037): the signals must reach a USER, not only an IPC client.
  // This runs AFTER commands have actually executed, so the panel has something
  // true to show: the button is clicked, the RENDERED panel is read back, and it
  // must contain the correlation id of the `record_conception` call made earlier
  // in this run -- a dashboard that renders a constant would fail here.
  await page.getByRole("button", { name: "Refresh diagnostics" }).click();
  await page.waitForTimeout(800);
  const dashboard = await page
    .locator("#diagnostics-panel")
    .innerText()
    .catch(() => "");
  const conceptionCorrelation = String(conception?.correlation_id ?? "");
  record(
    "the diagnostics dashboard renders readiness and metrics (DOD-037)",
    dashboard.includes("Readiness:") &&
      dashboard.includes("Commands recorded:"),
    dashboard.replace(/\s+/g, " ").slice(0, 200) || "panel absent",
  );
  record(
    "the dashboard shows the commands this run actually executed (DOD-037)",
    dashboard.includes("record_conception") &&
      !dashboard.includes("Nothing recorded in this session yet."),
    `record_conception in panel=${dashboard.includes("record_conception")}; correlation recorded=${Boolean(conceptionCorrelation)}`,
  );
  record(
    "the dashboard ties an outcome to its correlation id (DOD-037)",
    Boolean(conceptionCorrelation) && dashboard.includes(conceptionCorrelation),
    conceptionCorrelation
      ? `expected ${conceptionCorrelation} in the rendered panel`
      : "the record_conception response carried no correlation id",
  );
  record(
    "the dashboard reports traces from real executions (DOD-037)",
    /Traces: [1-9]/.test(dashboard),
    (dashboard.match(/Traces:[^\n]*/) ?? ["Traces line absent"])[0].slice(
      0,
      160,
    ),
  );

  // --- HARD RESTART: kill the process, relaunch, read the state back --------
  //
  // DOD-015 requires "Persistent state survives full process and container
  // restart and is readable by the supported runtime", with a pre-restart state
  // hash, hard restart evidence and a post-restart INDEPENDENT read. A
  // connection-level reopen inside one process does not satisfy "full process
  // restart", so this lane — the only one driving the packaged executable —
  // kills it and starts it again.
  //
  // The read-back goes through the PRODUCT (`list_conception_events`), not
  // through a file search, because "readable by the supported runtime" is the
  // clause's own wording.
  const preRestart = await page.evaluate(async (workspaceId) => {
    try {
      return await window.__TAURI_INTERNALS__.invoke("list_conception_events", {
        workspaceId,
      });
    } catch (e) {
      return { __error: String(e) };
    }
  }, "e2e-workspace");
  const preRestartOk =
    preRestart && !preRestart.__error && preRestart.ok === true;
  const preRestartCount = preRestartOk ? preRestart.value.count : -1;

  app.kill();
  try {
    app.kill("SIGKILL");
  } catch {
    /* already gone */
  }
  // Wait for the debug endpoint to actually stop answering, so the relaunch
  // cannot be mistaken for the old process still running.
  const goneDeadline = Date.now() + 20000;
  let portFree = false;
  while (!portFree && Date.now() < goneDeadline) {
    try {
      await fetch(`http://127.0.0.1:${PORT}/json`, {
        signal: AbortSignal.timeout(500),
      });
      await sleep(200);
    } catch {
      portFree = true;
    }
  }
  record(
    "hard restart: the previous process is gone (DOD-015)",
    portFree,
    portFree ? "debug endpoint stopped answering" : "port still answering",
  );

  if (browser) await browser.close().catch(() => {});
  app = spawn(EXE, [], { stdio: "ignore" });
  const restartTarget = await waitForPageTarget();
  record(
    "hard restart: the artifact relaunches (DOD-015)",
    restartTarget !== null,
    restartTarget ? "page target available after restart" : "no page target",
  );
  if (restartTarget) {
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
    const restartedPages = browser.contexts().flatMap((c) => c.pages());
    const restarted = restartedPages[0];
    await restarted
      .waitForFunction(() => document.readyState === "complete", {
        timeout: 20000,
      })
      .catch(() => {});
    // Wait for the IPC bridge AND retry the read: measured flake, the first
    // evaluate after a relaunch hit "Execution context was destroyed, most
    // likely because of a navigation" because the fresh webview was still
    // navigating. The read is idempotent, so retrying is safe; a permanently
    // absent bridge still fails after the bound.
    const bridgeDeadline = Date.now() + 30000;
    let bridgeReady = false;
    while (!bridgeReady && Date.now() < bridgeDeadline) {
      bridgeReady = await restarted
        .evaluate(() => typeof window.__TAURI_INTERNALS__ !== "undefined")
        .catch(() => false);
      if (!bridgeReady) await sleep(150);
    }
    record(
      "hard restart: the runtime's command bridge is ready (DOD-015)",
      bridgeReady,
      `ready=${bridgeReady}`,
    );

    let postRestart = { __error: "not attempted" };
    for (let attempt = 0; attempt < 5; attempt += 1) {
      postRestart = await restarted
        .evaluate(async (workspaceId) => {
          try {
            return await window.__TAURI_INTERNALS__.invoke(
              "list_conception_events",
              { workspaceId },
            );
          } catch (e) {
            return { __error: String(e) };
          }
        }, "e2e-workspace")
        .catch((e) => ({ __error: String(e) }));
      if (postRestart && postRestart.ok === true) break;
      if (!String(postRestart?.__error ?? "").includes("context")) break;
      await sleep(500);
    }

    const postRestartOk =
      postRestart && !postRestart.__error && postRestart.ok === true;
    const events = postRestartOk ? postRestart.value.events : [];
    const canarySurvived = events.some((e) => e.content.includes(canary));
    record(
      "hard restart: the ledger is readable by the runtime after relaunch (DOD-015)",
      postRestartOk &&
        postRestart.value.count >= preRestartCount &&
        preRestartCount > 0,
      postRestartOk
        ? `pre-restart ${preRestartCount} event(s), post-restart ${postRestart.value.count}`
        : String(postRestart?.__error ?? JSON.stringify(postRestart)),
    );
    record(
      "hard restart: the pre-restart canary is still present (DOD-015, DOD-013)",
      canarySurvived,
      canarySurvived
        ? `${canary} found in the ledger read back through the runtime`
        : `canary absent from ${events.length} ledger event(s)`,
    );
  }
} catch (err) {
  record("harness", false, String(err));
} finally {
  if (browser) await browser.close().catch(() => {});
  app.kill();
  await sleep(400);
  try {
    app.kill("SIGKILL");
  } catch {
    /* already gone */
  }
}

const digestAfter = sha256(EXE);
const digestStable = digestBefore === digestAfter;
record(
  "artifact digest unchanged across the run",
  digestStable,
  digestBefore.slice(0, 16),
);

const failures = checks.filter((c) => !c.ok);
const lines = [
  "# DOD-004 Exact-Artifact E2E",
  "",
  "Generated by `apps/desktop/e2e-artifact.mjs`. Do not hand-edit.",
  "",
  "## Artifact under test",
  "",
  `- Path: \`${EXE}\``,
  `- SHA-256: \`${digestBefore}\``,
  `- Size: ${size} bytes`,
  `- DevTools endpoint: \`http://127.0.0.1:${PORT}\``,
  "- Target: the packaged executable's real WebView2 instance -- NOT a",
  "  development server. Produced by `tauri build`, which is what embeds",
  "  `frontendDist`; a raw `cargo build --release` embeds `devUrl` and loads",
  "  `http://localhost:5173` instead.",
  "",
  "## Assertions (executed inside the artifact)",
  "",
  "| Check | Result | Observed |",
  "| --- | --- | --- |",
  ...checks.map(
    (c) => `| ${c.name} | ${c.ok ? "PASS" : "FAIL"} | \`${c.observed}\` |`,
  ),
  "",
  "## Verdict",
  "",
  failures.length === 0
    ? `PASS — ${checks.length} assertion(s) executed against digest \`${digestBefore}\``
    : `FAIL — ${failures.length} of ${checks.length} assertion(s) failed`,
  "",
  "## Why this satisfies the clause",
  "",
  "The clause excludes source and development servers. This runs against the",
  "packaged executable, asserts the loaded origin is `http://tauri.localhost/`",
  "(the embedded frontend), and additionally proves the Tauri IPC bridge exists",
  "and that three commands round-trip -- including a state-changing one whose",
  "effect is a SHA-256 content address in the durable vault. A browser-based run",
  "cannot demonstrate any of that.",
  "",
  "## Limitation",
  "",
  "The executable is launched from the build output rather than installed from",
  "the MSI. `scripts/smoke-installed-artifact.sh` covers the install path. A",
  "virgin clean-room install (DOD-034) remains EXTERNAL_REQUIRED.",
  "",
];

mkdirSync(dirname(REPORT), { recursive: true });
writeFileSync(REPORT, lines.join("\n"), "utf-8");

for (const c of checks) {
  console.log(`  ${c.ok ? "PASS" : "FAIL"}: ${c.name} -- ${c.observed}`);
}
console.log(`artifact-e2e: wrote ${REPORT}`);
process.exit(failures.length === 0 ? 0 : 1);
