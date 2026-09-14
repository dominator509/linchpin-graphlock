import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * LINCHPIN desktop shell.
 *
 * GraphLock context (SUP-001 "disconnected UI" finding): this component
 * previously rendered two static sentences and called nothing. The Tauri
 * backend now exposes real commands (`get_system_health`, `record_conception`,
 * `get_namespace_status`) and this UI invokes them, so the wiring is
 * bidirectional and observable.
 *
 * Runs outside Tauri (plain browser / E2E) degrade explicitly rather than
 * silently: `invoke` rejects when there is no IPC bridge, and the UI surfaces
 * that instead of pretending to be connected.
 */

interface SystemHealth {
  status: string;
  version: string;
  storage_ok: boolean;
}

interface NamespaceStatus {
  namespace: string;
  implemented: boolean;
  detail: string;
}

interface ConceptionEventView {
  event_id: string;
  content_hash: string;
  content_bytes: number;
  origin: string;
}

interface RecordConceptionOutcome {
  event: ConceptionEventView;
  persisted: boolean;
  storage_detail: string;
}

interface CommandResult<T> {
  correlation_id: string;
  ok: boolean;
  value?: T;
  error?: { class: string; message: string };
}

interface ExportDecision {
  allowed: boolean;
  sensitivity: string;
  reason: string;
}

interface ResearchTaskView {
  task_id: string;
  status: string;
  citations: string[];
  citation_count: number;
}

interface LintOutcome {
  passed: boolean;
  findings: string[];
}

interface FilingPackageView {
  format: string;
  manifest: string;
}

interface ReceiptView {
  application_number: string;
  confirmation_number: string;
}

interface HandoffReadiness {
  ready_for_human_submission: boolean;
  blockers: string[];
  note: string;
}

interface ProviderLane {
  lane: string;
  transport_available: boolean;
  configured: boolean;
  detail: string;
}

interface McpDecision {
  requested: string;
  allowed: boolean;
  reason: string;
}

interface InferenceOutcome {
  live: boolean;
  transport: string;
  text?: string;
  error_class?: string;
  detail: string;
}

/** True when running inside the Tauri shell where IPC is available. */
export function hasIpc(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
      "undefined"
  );
}

export default function App() {
  const [health, setHealth] = useState<SystemHealth | null>(null);
  const [namespaces, setNamespaces] = useState<NamespaceStatus[]>([]);
  const [ipcError, setIpcError] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [authorIsHuman, setAuthorIsHuman] = useState(true);
  const [lastRecord, setLastRecord] =
    useState<CommandResult<RecordConceptionOutcome> | null>(null);
  const [researchTaskId, setResearchTaskId] = useState("kill-search-1");
  const [researchStatus, setResearchStatus] = useState("Pending");
  const [researchCitations, setResearchCitations] = useState<string[]>([]);
  const [researchError, setResearchError] = useState<string | null>(null);
  const [exportSensitivity, setExportSensitivity] = useState("Restricted");
  const [exportResult, setExportResult] =
    useState<CommandResult<ExportDecision> | null>(null);
  const [claimsDraft, setClaimsDraft] = useState(
    "1. A device comprising a valve",
  );
  const [lintResult, setLintResult] =
    useState<CommandResult<LintOutcome> | null>(null);
  const [packageResult, setPackageResult] =
    useState<CommandResult<FilingPackageView> | null>(null);
  const [receiptText, setReceiptText] = useState(
    "AppNumber: 17/123,456\nConfNumber: 4321",
  );
  const [receiptResult, setReceiptResult] =
    useState<CommandResult<ReceiptView> | null>(null);
  const [handoffResult, setHandoffResult] =
    useState<CommandResult<HandoffReadiness> | null>(null);
  const [providerLanes, setProviderLanes] = useState<ProviderLane[]>([]);
  const [mcpDecision, setMcpDecision] = useState<McpDecision | null>(null);
  const [opsError, setOpsError] = useState<string | null>(null);
  const [inference, setInference] =
    useState<CommandResult<InferenceOutcome> | null>(null);

  useEffect(() => {
    if (!hasIpc()) {
      // Honest state: outside the Tauri shell there is no backend to report on.
      setIpcError(
        "Desktop backend unavailable (running outside the Tauri shell).",
      );
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const [h, ns] = await Promise.all([
          invoke<SystemHealth>("get_system_health"),
          invoke<NamespaceStatus[]>("get_namespace_status"),
        ]);
        if (cancelled) return;
        setHealth(h);
        setNamespaces(ns);
      } catch (err) {
        if (cancelled) return;
        setIpcError(String(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const onRecord = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot record: desktop backend unavailable.");
      return;
    }
    try {
      const result = await invoke<CommandResult<RecordConceptionOutcome>>(
        "record_conception",
        {
          workspaceId: "local-workspace",
          content: draft,
          authorIsHuman,
        },
      );
      setLastRecord(result);
    } catch (err) {
      setIpcError(String(err));
    }
  }, [draft, authorIsHuman]);

  const onLoadOps = useCallback(async () => {
    if (!hasIpc()) {
      setOpsError("Cannot load status: desktop backend unavailable.");
      return;
    }
    try {
      const lanes =
        await invoke<CommandResult<ProviderLane[]>>("provider_status");
      if (lanes.ok && lanes.value) setProviderLanes(lanes.value);

      const mcp = await invoke<CommandResult<McpDecision>>(
        "check_mcp_capability",
        { granted: ["ReadVault"], requested: "WriteVault" },
      );
      if (mcp.ok && mcp.value) setMcpDecision(mcp.value);
      setOpsError(null);
    } catch (err) {
      setOpsError(String(err));
    }
  }, []);

  const implemented = namespaces.filter((n) => n.implemented);

  const onRunInference = useCallback(async () => {
    if (!hasIpc()) {
      setOpsError("Cannot probe provider: desktop backend unavailable.");
      return;
    }
    try {
      setInference(
        await invoke<CommandResult<InferenceOutcome>>("run_local_inference", {
          endpoint: "http://127.0.0.1:11434",
          modelId: "unset",
          prompt: "ping",
        }),
      );
    } catch (err) {
      setOpsError(String(err));
    }
  }, []);

  const onLintClaims = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot lint: desktop backend unavailable.");
      return;
    }
    try {
      setLintResult(
        await invoke<CommandResult<LintOutcome>>("lint_claims", {
          claims: claimsDraft,
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [claimsDraft]);

  const onBuildPackage = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot build package: desktop backend unavailable.");
      return;
    }
    try {
      setPackageResult(
        await invoke<CommandResult<FilingPackageView>>("build_filing_package", {
          claims: claimsDraft,
          specification: draft || "Draft specification",
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [claimsDraft, draft]);

  const onImportReceipt = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot import receipt: desktop backend unavailable.");
      return;
    }
    try {
      setReceiptResult(
        await invoke<CommandResult<ReceiptView>>("import_receipt", {
          receiptText,
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [receiptText]);

  const onCheckHandoff = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot check handoff: desktop backend unavailable.");
      return;
    }
    try {
      setHandoffResult(
        await invoke<CommandResult<HandoffReadiness>>("check_filing_handoff", {
          forms: ["Ads", "Sba"],
          feePaid: false,
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, []);

  const onResearchAction = useCallback(
    async (action: "start" | "kill" | "complete") => {
      if (!hasIpc()) {
        setResearchError("Cannot act: desktop backend unavailable.");
        return;
      }
      try {
        const result = await invoke<CommandResult<ResearchTaskView>>(
          "apply_research_action",
          {
            taskId: researchTaskId,
            currentStatus: researchStatus,
            citations: researchCitations,
            action,
            citation: null,
          },
        );
        if (result.ok && result.value) {
          setResearchStatus(result.value.status);
          setResearchCitations(result.value.citations);
          setResearchError(null);
        } else {
          setResearchError(
            `${result.error?.class ?? "ERROR"}: ${result.error?.message ?? "unknown"}`,
          );
        }
      } catch (err) {
        setResearchError(String(err));
      }
    },
    [researchTaskId, researchStatus, researchCitations],
  );

  const onEvaluateExport = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot evaluate export: desktop backend unavailable.");
      return;
    }
    try {
      const result = await invoke<CommandResult<ExportDecision>>(
        "evaluate_export",
        {
          workspaceId: "local-workspace",
          content: draft || "sample export content",
          sensitivity: exportSensitivity,
        },
      );
      setExportResult(result);
    } catch (err) {
      setIpcError(String(err));
    }
  }, [draft, exportSensitivity]);

  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <h1>LINCHPIN Patent Intelligence OS</h1>
      <p>Local-First Confidentiality Boundary Active.</p>

      <section aria-labelledby="health-heading">
        <h2 id="health-heading">System Health</h2>
        {ipcError && <p role="alert">{ipcError}</p>}
        {health ? (
          <ul>
            <li>Status: {health.status}</li>
            <li>Version: {health.version}</li>
            <li>Storage writable: {String(health.storage_ok)}</li>
          </ul>
        ) : (
          !ipcError && <p>Checking…</p>
        )}
      </section>

      <section aria-labelledby="conception-heading">
        <h2 id="conception-heading">Conception Lab</h2>
        <label htmlFor="conception-input">Describe your conception</label>
        <textarea
          id="conception-input"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          rows={4}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <label htmlFor="origin-select">Origin</label>
        <select
          id="origin-select"
          value={authorIsHuman ? "human" : "ai"}
          onChange={(e) => setAuthorIsHuman(e.target.value === "human")}
        >
          <option value="human">Human conception</option>
          <option value="ai">AI suggestion</option>
        </select>
        <button type="button" onClick={() => void onRecord()}>
          Record conception event
        </button>

        {lastRecord && (
          <div>
            {lastRecord.ok && lastRecord.value ? (
              <>
                <p>Origin: {lastRecord.value.event.origin}</p>
                <p>Content hash: {lastRecord.value.event.content_hash}</p>
                <p>
                  Persisted: {String(lastRecord.value.persisted)} —{" "}
                  {lastRecord.value.storage_detail}
                </p>
              </>
            ) : (
              <p role="alert">
                {lastRecord.error?.class}: {lastRecord.error?.message}
              </p>
            )}
            <p>Correlation: {lastRecord.correlation_id}</p>
          </div>
        )}
      </section>

      <section aria-labelledby="research-heading">
        <h2 id="research-heading">Research Kill-Search</h2>
        <label htmlFor="research-task">Task ID</label>
        <input
          id="research-task"
          value={researchTaskId}
          onChange={(e) => setResearchTaskId(e.target.value)}
        />
        <p>Status: {researchStatus}</p>
        <p>Citations: {researchCitations.length}</p>
        {researchError && <p role="alert">{researchError}</p>}
        <button type="button" onClick={() => void onResearchAction("start")}>
          Start search
        </button>
        <button type="button" onClick={() => void onResearchAction("kill")}>
          Record kill
        </button>
        <button type="button" onClick={() => void onResearchAction("complete")}>
          Complete search
        </button>
      </section>

      <section aria-labelledby="firewall-heading">
        <h2 id="firewall-heading">Disclosure Firewall</h2>
        <label htmlFor="sensitivity-select">Export sensitivity</label>
        <select
          id="sensitivity-select"
          value={exportSensitivity}
          onChange={(e) => setExportSensitivity(e.target.value)}
        >
          <option value="Public">Public</option>
          <option value="Confidential">Confidential</option>
          <option value="Restricted">Restricted</option>
        </select>
        <button type="button" onClick={() => void onEvaluateExport()}>
          Evaluate export
        </button>
        {exportResult && exportResult.value && (
          <p>
            {exportResult.value.allowed ? "Allowed" : "Blocked"} (
            {exportResult.value.sensitivity}): {exportResult.value.reason}
          </p>
        )}
      </section>

      <section aria-labelledby="patent-heading">
        <h2 id="patent-heading">Patent Architect</h2>
        <label htmlFor="claims-input">Claims</label>
        <textarea
          id="claims-input"
          value={claimsDraft}
          onChange={(e) => setClaimsDraft(e.target.value)}
          rows={4}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onLintClaims()}>
          Lint claims
        </button>
        <button type="button" onClick={() => void onBuildPackage()}>
          Build filing package
        </button>
        {lintResult?.value && (
          <p>
            Lint: {lintResult.value.passed ? "passed" : "failed"}
            {lintResult.value.findings.length > 0 &&
              ` — ${lintResult.value.findings.join("; ")}`}
          </p>
        )}
        {packageResult?.value && (
          <p>
            Package format: {packageResult.value.format} (
            {packageResult.value.manifest})
          </p>
        )}
      </section>

      <section aria-labelledby="filing-heading">
        <h2 id="filing-heading">Filing</h2>
        <label htmlFor="receipt-input">Acknowledgement receipt</label>
        <textarea
          id="receipt-input"
          value={receiptText}
          onChange={(e) => setReceiptText(e.target.value)}
          rows={3}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onImportReceipt()}>
          Import receipt
        </button>
        {receiptResult?.value && (
          <p>
            Application {receiptResult.value.application_number}, confirmation{" "}
            {receiptResult.value.confirmation_number}
          </p>
        )}
        {receiptResult && !receiptResult.ok && (
          <p role="alert">{receiptResult.error?.message}</p>
        )}
        <button type="button" onClick={() => void onCheckHandoff()}>
          Check handoff readiness
        </button>
        {handoffResult?.value && (
          <p>
            Ready for human submission:{" "}
            {String(handoffResult.value.ready_for_human_submission)}
            {handoffResult.value.blockers.length > 0 &&
              ` — blockers: ${handoffResult.value.blockers.join("; ")}`}
          </p>
        )}
        <p>Human submission only: LINCHPIN does not sign, pay or submit.</p>
      </section>

      <section aria-labelledby="ops-heading">
        <h2 id="ops-heading">Operations</h2>
        <p>
          Provider lanes, MCP grants and incident capsules are reported from the
          backend; no lane performs inference on this host.
        </p>
        <button type="button" onClick={() => void onLoadOps()}>
          Load operational status
        </button>
        {opsError && <p role="alert">{opsError}</p>}
        {providerLanes.length > 0 && (
          <ul>
            {providerLanes.map((l) => (
              <li key={l.lane}>
                {l.lane}: {l.configured ? "configured" : "not configured"} —{" "}
                {l.detail}
              </li>
            ))}
          </ul>
        )}
        {mcpDecision && (
          <p>
            MCP {mcpDecision.requested}:{" "}
            {mcpDecision.allowed ? "allowed" : "denied"} — {mcpDecision.reason}
          </p>
        )}

        <button type="button" onClick={() => void onRunInference()}>
          Probe local model
        </button>
        {inference && inference.value && (
          <p>
            Local inference:{" "}
            {inference.value.live
              ? `live via ${inference.value.transport}`
              : `not live (${inference.value.error_class}): ${inference.value.detail}`}
          </p>
        )}
      </section>

      {namespaces.length > 0 && (
        <section aria-labelledby="ns-heading">
          <h2 id="ns-heading">Capability Coverage</h2>
          <p>
            {implemented.length} of {namespaces.length} contract namespaces
            implemented.
          </p>
          <ul>
            {namespaces.map((n) => (
              <li key={n.namespace}>
                {n.namespace}:{" "}
                {n.implemented ? "available" : "not yet available"}
              </li>
            ))}
          </ul>
        </section>
      )}
    </main>
  );
}
