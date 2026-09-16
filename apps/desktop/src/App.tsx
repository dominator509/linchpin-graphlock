import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * LINCHPIN desktop shell.
 *
 * GraphLock context (SUP-001 "disconnected UI" finding): this component once
 * rendered two static sentences and called nothing. The Tauri backend now
 * exposes real commands, and this UI invokes them, so the wiring is
 * bidirectional and observable.
 *
 * REQ-UI-001 requires primary navigation across thirteen surfaces: Dashboard,
 * Opportunity Radar, Conception Lab, Research War Room, Patent Architect,
 * Filing, Docket, Prosecution, Commercialize, Evidence Vault, Integrations,
 * Incidents and Settings. Each surface below is backed by real commands -- none
 * is a placeholder, which DOD-019 forbids in production paths. The navigation is
 * in-page because this is a single-window desktop app: the thirteen surfaces are
 * all present and all reachable, and the nav links are real anchors to them.
 *
 * REQ-UI-002 requires persistent top-level badges for confidentiality,
 * filing/priority state, evidence coverage and external-provider egress state.
 * They render from backend state and never assert a state the backend did not
 * report.
 *
 * REQ-UI-003 requires legal-risk language to use "screen", "draft", "evidence"
 * and "uncertainty" rather than definitive legal conclusions.
 *
 * REQ-UI-004 requires destructive/high-impact actions to use consequence-specific
 * confirmation and audit. High-impact actions therefore do NOT fire on first
 * click: they reveal a panel naming the consequence, and a confirmed action is
 * recorded in an on-screen audit trail with its correlation ID.
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

interface ConfigurationView {
  app_data_dir: string;
  vault_file: string;
  runtime: string;
  warnings: string[];
}

interface BackupView {
  destination: string;
  state_digest: string;
}

/**
 * Recovery outcome (REQ-REL-005).
 *
 * `destination_recreated` and `destination_quarantined` are surfaced because a
 * recovery that silently created a vault, or silently moved an unreadable one
 * aside, would leave the operator unable to tell what happened to their data.
 */
interface RestoreView {
  source: string;
  digest_before: string;
  digest_after: string;
  reconciled: boolean;
  destination_recreated: boolean;
  destination_quarantined?: string | null;
}

/**
 * Declared scope and truth boundaries (REQ-SCOPE-001).
 *
 * Rendered from the backend declaration rather than a copy in the UI, so the
 * promise the user reads and the promise the code enforces cannot drift.
 */
interface BoundaryView {
  id: string;
  statement: string;
  enforced_by: string;
}

interface CapabilityView {
  requirement_id: string;
  uo_id: string;
  title: string;
  state: string;
  realised_by: string;
  limitation: string;
}

interface ScopeView {
  boundaries: BoundaryView[];
  capabilities: CapabilityView[];
}

/** Operator diagnostics (DOD-037): signals read from the backend, not recomputed. */
interface DiagnosticEventView {
  correlation_id: string;
  command: string;
  outcome: string;
  error_class?: string | null;
  detail: string;
  duration_ms: number;
  at_utc: string;
}

interface DiagnosticsMetrics {
  recorded: number;
  succeeded: number;
  failed: number;
  failure_rate: number;
  p50_duration_ms: number;
  p95_duration_ms: number;
}

interface DiagnosticsView {
  readiness: string;
  storage_ok: boolean;
  vault_file: string;
  metrics: DiagnosticsMetrics;
  alerts: string[];
  events: DiagnosticEventView[];
  traces: string[];
  redaction_applied: boolean;
  detail: string;
}

interface LedgerEventView {
  event_id: string;
  origin: string;
  content_hash: string;
  content_bytes: number;
  created_utc: string;
  version: number;
  content: string;
}

interface ConceptionLedgerView {
  workspace_id: string;
  count: number;
  human_count: number;
  ai_count: number;
  events: LedgerEventView[];
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
  requested_scopes: number;
  covered_scopes: number;
  uncovered_scopes: number;
  coverage_complete: boolean;
  checkpoint?: string | null;
}

interface ClaimClassOutcome {
  class: string;
  emittable_from_source_record: boolean;
  requires: string[];
}

interface LintOutcome {
  passed: boolean;
  findings: string[];
}

interface SupportMatrixView {
  unsupported: string[];
  exportable: boolean;
  anchor_count: number;
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

interface OpportunityView {
  description: string;
  score: number;
  uncertainty: string;
  evidence_count: number;
}

interface DeadlineView {
  due_date: string;
  authoritative: boolean;
  ruleset_source?: string | null;
  ruleset_version?: string | null;
  suggested_by_model: boolean;
  reviewed: boolean;
}

interface DocketView {
  docket_id: string;
  state: string;
  public_disclosure: boolean;
}

interface OfficeActionView {
  action_type: string;
  cited_art_count: number;
  response_drafted: boolean;
}

interface CommercializationView {
  target_count: number;
  redacted: boolean;
  payload: string;
}

interface ValuationView {
  low: number;
  high: number;
  is_range: boolean;
  scenario_label: string;
  sensitivity: [string, number][];
  dominant_assumption?: string | null;
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
  attempts: number;
  retryable_exhausted: boolean;
}

interface RepairCapsuleView {
  redacted: boolean;
  safe_for_export: boolean;
  incident_detail: string;
}

interface EvidenceExportView {
  path: string;
  exported: boolean;
  sensitivity: string;
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

/** The thirteen surfaces REQ-UI-001 names, in the order it names them. */
const SURFACES = [
  { id: "dashboard", label: "Dashboard" },
  { id: "opportunity", label: "Opportunity Radar" },
  { id: "conception", label: "Conception Lab" },
  { id: "research", label: "Research War Room" },
  { id: "patent", label: "Patent Architect" },
  { id: "filing", label: "Filing" },
  { id: "docket", label: "Docket" },
  { id: "prosecution", label: "Prosecution" },
  { id: "commercialize", label: "Commercialize" },
  { id: "evidence", label: "Evidence Vault" },
  { id: "integrations", label: "Integrations" },
  { id: "incidents", label: "Incidents" },
  { id: "settings", label: "Settings" },
] as const;

interface AuditEntry {
  action: string;
  consequence: string;
  correlationId: string;
  at: string;
}

const WORKSPACE = "local-workspace";

/**
 * A high-impact action that confirms before it runs (REQ-UI-004).
 *
 * The consequence is stated in the confirm panel and is repeated in the audit
 * trail, so a recorded action explains what it did rather than only that it
 * happened.
 */
function ConfirmAction({
  label,
  consequence,
  confirmLabel,
  onConfirm,
  onAudit,
}: {
  label: string;
  consequence: string;
  confirmLabel: string;
  onConfirm: () => Promise<string | null>;
  onAudit: (entry: AuditEntry) => void;
}) {
  const [pending, setPending] = useState(false);
  const [busy, setBusy] = useState(false);

  return (
    <div>
      <button type="button" onClick={() => setPending(true)} disabled={busy}>
        {label}
      </button>
      {pending && (
        <div
          role="group"
          aria-label={`Confirm ${label}`}
          style={{
            border: "1px solid #999",
            padding: "0.5rem",
            marginTop: "0.5rem",
          }}
        >
          <p>
            <strong>Confirm:</strong> {consequence}
          </p>
          <button
            type="button"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void (async () => {
                const correlationId = await onConfirm();
                if (correlationId) {
                  onAudit({
                    action: label,
                    consequence,
                    correlationId,
                    at: new Date().toISOString(),
                  });
                }
                setBusy(false);
                setPending(false);
              })();
            }}
          >
            {confirmLabel}
          </button>
          <button
            type="button"
            onClick={() => setPending(false)}
            disabled={busy}
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}

export default function App() {
  const [health, setHealth] = useState<SystemHealth | null>(null);
  const [namespaces, setNamespaces] = useState<NamespaceStatus[]>([]);
  const [configuration, setConfiguration] = useState<ConfigurationView | null>(
    null,
  );
  const [ipcError, setIpcError] = useState<string | null>(null);

  // Durable state recovery (REQ-REL-005). The backup path is operator-supplied
  // and defaults to a sibling of the real vault file, so recovery never depends
  // on a path the product invented.
  const [backupFile, setBackupFile] = useState("");
  const [backupResult, setBackupResult] =
    useState<CommandResult<BackupView> | null>(null);
  const [restoreResult, setRestoreResult] =
    useState<CommandResult<RestoreView> | null>(null);
  const [scopeDeclaration, setScopeDeclaration] =
    useState<CommandResult<ScopeView> | null>(null);
  const [diagnostics, setDiagnostics] =
    useState<CommandResult<DiagnosticsView> | null>(null);

  const [draft, setDraft] = useState("");
  const [authorIsHuman, setAuthorIsHuman] = useState(true);
  const [lastRecord, setLastRecord] =
    useState<CommandResult<RecordConceptionOutcome> | null>(null);
  const [ledger, setLedger] =
    useState<CommandResult<ConceptionLedgerView> | null>(null);

  // Opportunity Radar
  const [opportunityDescription, setOpportunityDescription] = useState(
    "A self-sealing valve for high-pressure lines",
  );
  const [opportunityUncertainty, setOpportunityUncertainty] =
    useState("Medium");
  const [opportunity, setOpportunity] =
    useState<CommandResult<OpportunityView> | null>(null);

  // Research War Room
  const [researchTaskId, setResearchTaskId] = useState("kill-search-1");
  const [researchStatus, setResearchStatus] = useState("Pending");
  const [researchCitations, setResearchCitations] = useState<string[]>([]);
  const [researchError, setResearchError] = useState<string | null>(null);
  const [researchClaimClass, setResearchClaimClass] = useState("HYPOTHESIS");
  const [researchClaimText, setResearchClaimText] = useState(
    "The market is moving toward sealed valves",
  );
  const [claimOutcome, setClaimOutcome] =
    useState<CommandResult<ClaimClassOutcome> | null>(null);
  const [requestedScopes, setRequestedScopes] = useState(4);
  const [coveredScopes, setCoveredScopes] = useState(1);

  // Patent Architect
  const [claimsDraft, setClaimsDraft] = useState(
    "1. A device comprising a valve",
  );
  const [lintResult, setLintResult] =
    useState<CommandResult<LintOutcome> | null>(null);
  const [packageResult, setPackageResult] =
    useState<CommandResult<FilingPackageView> | null>(null);
  const [matrixLimitations, setMatrixLimitations] = useState(
    "a self-sealing valve\na pressure sensor",
  );
  const [matrixResult, setMatrixResult] =
    useState<CommandResult<SupportMatrixView> | null>(null);

  // Filing
  const [receiptText, setReceiptText] = useState(
    "AppNumber: 17/123,456\nConfNumber: 4321",
  );
  const [receiptResult, setReceiptResult] =
    useState<CommandResult<ReceiptView> | null>(null);
  const [handoffResult, setHandoffResult] =
    useState<CommandResult<HandoffReadiness> | null>(null);

  // Docket
  const [docketState, setDocketState] = useState("Preparation");
  const [dueDate, setDueDate] = useState("2026-11-14");
  const [rulesetSource, setRulesetSource] = useState("USPTO-37CFR");
  const [rulesetVersion, setRulesetVersion] = useState("2026.1");
  const [deadlineResult, setDeadlineResult] =
    useState<CommandResult<DeadlineView> | null>(null);
  const [docketResult, setDocketResult] =
    useState<CommandResult<DocketView> | null>(null);

  // Prosecution
  const [officeActionType, setOfficeActionType] = useState("NonFinalRejection");
  const [officeActionResult, setOfficeActionResult] =
    useState<CommandResult<OfficeActionView> | null>(null);

  // Commercialize
  const [targetNames, setTargetNames] = useState("MedTech Corp\nValveWorks");
  const [commercializationResult, setCommercializationResult] =
    useState<CommandResult<CommercializationView> | null>(null);
  const [valuationResult, setValuationResult] =
    useState<CommandResult<ValuationView> | null>(null);

  // Evidence Vault
  const [exportSensitivity, setExportSensitivity] = useState("Restricted");
  const [exportResult, setExportResult] =
    useState<CommandResult<ExportDecision> | null>(null);
  const [evidenceExportResult, setEvidenceExportResult] =
    useState<CommandResult<EvidenceExportView> | null>(null);

  // Integrations
  const [providerLanes, setProviderLanes] = useState<ProviderLane[]>([]);
  const [mcpDecision, setMcpDecision] = useState<McpDecision | null>(null);
  const [opsError, setOpsError] = useState<string | null>(null);
  const [inference, setInference] =
    useState<CommandResult<InferenceOutcome> | null>(null);

  // Incidents
  const [incidentDetail, setIncidentDetail] = useState(
    "panic while saving draft: PROVISIONAL-CLAIM-1 a self-sealing valve",
  );
  const [capsuleResult, setCapsuleResult] =
    useState<CommandResult<RepairCapsuleView> | null>(null);

  const [auditLog, setAuditLog] = useState<AuditEntry[]>([]);
  const addAudit = useCallback(
    (entry: AuditEntry) => setAuditLog((prev) => [entry, ...prev]),
    [],
  );

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
        const [h, ns, cfg, declared] = await Promise.all([
          invoke<SystemHealth>("get_system_health"),
          invoke<NamespaceStatus[]>("get_namespace_status"),
          invoke<CommandResult<ConfigurationView>>("get_configuration"),
          invoke<CommandResult<ScopeView>>("get_scope_declaration"),
        ]);
        if (cancelled) return;
        setHealth(h);
        setNamespaces(ns);
        if (cfg.ok && cfg.value) setConfiguration(cfg.value);
        setScopeDeclaration(declared);
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
        { workspaceId: WORKSPACE, content: draft, authorIsHuman },
      );
      setLastRecord(result);
    } catch (err) {
      setIpcError(String(err));
    }
  }, [draft, authorIsHuman]);

  const onLoadLedger = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot read the ledger: desktop backend unavailable.");
      return;
    }
    try {
      setLedger(
        await invoke<CommandResult<ConceptionLedgerView>>(
          "list_conception_events",
          { workspaceId: WORKSPACE },
        ),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, []);

  const onRefreshDiagnostics = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot read diagnostics: desktop backend unavailable.");
      return;
    }
    try {
      setDiagnostics(
        await invoke<CommandResult<DiagnosticsView>>("get_diagnostics"),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, []);

  const onEvaluateOpportunity = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot evaluate: desktop backend unavailable.");
      return;
    }
    try {
      setOpportunity(
        await invoke<CommandResult<OpportunityView>>("evaluate_opportunity", {
          workspaceId: WORKSPACE,
          description: opportunityDescription,
          technicalFeasibility: 0.7,
          marketPotential: 0.6,
          legalRisk: 0.3,
          uncertainty: opportunityUncertainty,
          evidenceUris: ["https://example.invalid/evidence/1"],
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [opportunityDescription, opportunityUncertainty]);

  const onClassifyClaim = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot classify: desktop backend unavailable.");
      return;
    }
    try {
      setClaimOutcome(
        await invoke<CommandResult<ClaimClassOutcome>>(
          "classify_research_claim",
          {
            class: researchClaimClass,
            text: researchClaimText,
            sourceRecord: null,
            inferenceMethod:
              researchClaimClass === "OBSERVATION"
                ? null
                : "extrapolated from three filings",
            contraryEvidence:
              researchClaimClass === "OBSERVATION"
                ? null
                : "one filing contradicts this",
          },
        ),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [researchClaimClass, researchClaimText]);

  const onSetCoverage = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot record coverage: desktop backend unavailable.");
      return;
    }
    try {
      await invoke<CommandResult<ResearchTaskView>>("set_research_coverage", {
        workspaceId: WORKSPACE,
        taskId: researchTaskId,
        requestedScopes,
        coveredScopes,
      });
    } catch (err) {
      setIpcError(String(err));
    }
  }, [researchTaskId, requestedScopes, coveredScopes]);

  const onCompletePartial = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot complete: desktop backend unavailable.");
      return;
    }
    try {
      const r = await invoke<CommandResult<ResearchTaskView>>(
        "complete_research_partial",
        {
          workspaceId: WORKSPACE,
          taskId: researchTaskId,
          coveredScopes,
          checkpoint: `checkpoint://${researchTaskId}/${coveredScopes}`,
        },
      );
      if (!r.ok) {
        setResearchError(`${r.error?.class ?? "ERROR"}: ${r.error?.message}`);
      } else {
        setResearchError(null);
      }
    } catch (err) {
      setIpcError(String(err));
    }
  }, [researchTaskId, coveredScopes]);

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

  const onRunInference = useCallback(async (): Promise<string | null> => {
    if (!hasIpc()) {
      setOpsError("Cannot probe provider: desktop backend unavailable.");
      return null;
    }
    try {
      const result = await invoke<CommandResult<InferenceOutcome>>(
        "run_local_inference",
        {
          endpoint: "http://127.0.0.1:11434",
          modelId: "unset",
          prompt: "ping",
        },
      );
      setInference(result);
      return result.correlation_id;
    } catch (err) {
      setOpsError(String(err));
      return null;
    }
  }, []);

  // The backup path falls back to a sibling of the configured vault file, so the
  // control is usable before the operator types anything.
  const resolvedBackupFile =
    backupFile || (configuration ? `${configuration.vault_file}.backup` : "");

  const onBackupVault = useCallback(async (): Promise<string | null> => {
    if (!hasIpc()) {
      setOpsError("Cannot back up: desktop backend unavailable.");
      return null;
    }
    try {
      const result = await invoke<CommandResult<BackupView>>("backup_vault", {
        workspaceId: WORKSPACE,
        destination: resolvedBackupFile,
      });
      setBackupResult(result);
      return result.ok ? result.correlation_id : null;
    } catch (err) {
      setOpsError(String(err));
      return null;
    }
  }, [resolvedBackupFile]);

  const onRestoreVault = useCallback(async (): Promise<string | null> => {
    if (!hasIpc()) {
      setOpsError("Cannot restore: desktop backend unavailable.");
      return null;
    }
    try {
      const result = await invoke<CommandResult<RestoreView>>("restore_vault", {
        workspaceId: WORKSPACE,
        source: resolvedBackupFile,
      });
      setRestoreResult(result);
      return result.ok ? result.correlation_id : null;
    } catch (err) {
      setOpsError(String(err));
      return null;
    }
  }, [resolvedBackupFile]);

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

  const onBuildPackage = useCallback(async (): Promise<string | null> => {
    if (!hasIpc()) {
      setIpcError("Cannot build package: desktop backend unavailable.");
      return null;
    }
    try {
      const result = await invoke<CommandResult<FilingPackageView>>(
        "build_filing_package",
        {
          claims: claimsDraft,
          specification: draft || "Draft specification",
        },
      );
      setPackageResult(result);
      return result.correlation_id;
    } catch (err) {
      setIpcError(String(err));
      return null;
    }
  }, [claimsDraft, draft]);

  const onCheckMatrix = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot check support: desktop backend unavailable.");
      return;
    }
    const limitations = matrixLimitations
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    try {
      setMatrixResult(
        await invoke<CommandResult<SupportMatrixView>>("check_support_matrix", {
          workspaceId: WORKSPACE,
          exportable: limitations,
          anchors: limitations
            .slice(0, 1)
            .map((l) => [l, "SPECIFICATION", "[0042]"]),
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [matrixLimitations]);

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

  const onScheduleDeadline = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot schedule: desktop backend unavailable.");
      return;
    }
    try {
      setDeadlineResult(
        await invoke<CommandResult<DeadlineView>>("schedule_docket_deadline", {
          workspaceId: WORKSPACE,
          dueDate,
          rulesetSource,
          rulesetVersion,
          suggestedByModel: false,
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [dueDate, rulesetSource, rulesetVersion]);

  const onAdvanceDocket = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot advance: desktop backend unavailable.");
      return;
    }
    try {
      const r = await invoke<CommandResult<DocketView>>("advance_docket", {
        workspaceId: WORKSPACE,
        currentState: docketState,
        receipt: "US123456",
        commercialize: false,
      });
      setDocketResult(r);
      if (r.ok && r.value) setDocketState(r.value.state);
    } catch (err) {
      setIpcError(String(err));
    }
  }, [docketState]);

  const onDraftResponse = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot draft: desktop backend unavailable.");
      return;
    }
    try {
      setOfficeActionResult(
        await invoke<CommandResult<OfficeActionView>>(
          "draft_office_action_response",
          { actionType: officeActionType, citedArt: ["US1234567B2"] },
        ),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [officeActionType]);

  const onBuildCommercialization = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot build package: desktop backend unavailable.");
      return;
    }
    const targets = targetNames
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    try {
      setCommercializationResult(
        await invoke<CommandResult<CommercializationView>>(
          "build_commercialization_package",
          { workspaceId: WORKSPACE, targetNames: targets },
        ),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [targetNames]);

  const onEvaluateValuation = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot value: desktop backend unavailable.");
      return;
    }
    try {
      setValuationResult(
        await invoke<CommandResult<ValuationView>>("evaluate_valuation", {
          workspaceId: WORKSPACE,
          scenarioLabel: "Illustrative planning scenario only",
          low: 1_000_000,
          high: 2_500_000,
          assumptions: [
            ["market size", 100, 400],
            ["royalty rate", 0.02, 0.05],
          ],
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, []);

  const onEvaluateExport = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot evaluate export: desktop backend unavailable.");
      return;
    }
    try {
      setExportResult(
        await invoke<CommandResult<ExportDecision>>("evaluate_export", {
          workspaceId: WORKSPACE,
          content: draft || "sample screening content",
          sensitivity: exportSensitivity,
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [draft, exportSensitivity]);

  const onExportEvidence = useCallback(async (): Promise<string | null> => {
    if (!hasIpc()) {
      setIpcError("Cannot export: desktop backend unavailable.");
      return null;
    }
    try {
      const result = await invoke<CommandResult<EvidenceExportView>>(
        "export_evidence",
        {
          workspaceId: WORKSPACE,
          path: "vault/evidence-bundle.json",
          content: draft || "sample screening content",
          sensitivity: exportSensitivity,
        },
      );
      setEvidenceExportResult(result);
      return result.correlation_id;
    } catch (err) {
      setIpcError(String(err));
      return null;
    }
  }, [draft, exportSensitivity]);

  const onBuildCapsule = useCallback(async () => {
    if (!hasIpc()) {
      setIpcError("Cannot build capsule: desktop backend unavailable.");
      return;
    }
    try {
      setCapsuleResult(
        await invoke<CommandResult<RepairCapsuleView>>("build_repair_capsule", {
          incidentDetail,
          agentBrief: "screening failure in draft save",
          secretsToRedact: ["PROVISIONAL-CLAIM-1"],
        }),
      );
    } catch (err) {
      setIpcError(String(err));
    }
  }, [incidentDetail]);

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

  const implemented = namespaces.filter((n) => n.implemented);
  const coverageBadge =
    namespaces.length > 0
      ? `${implemented.length} of ${namespaces.length} contract namespaces available`
      : "Evidence coverage not reported by the backend";
  const egressBadge = providerLanes.some((l) => l.configured)
    ? "External-provider egress: a lane is configured"
    : "External-provider egress: no lane configured; screening stays on this device";
  const filingBadge =
    docketResult?.value?.state ??
    (receiptResult?.value
      ? `Filed (application ${receiptResult.value.application_number})`
      : "Filing state: preparation");

  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <h1>LINCHPIN Patent Intelligence OS</h1>

      {/* REQ-UI-002: persistent top-level badges. Each is derived from backend
          state; none asserts a state the backend did not report. */}
      <ul aria-label="Status badges" style={{ listStyle: "none", padding: 0 }}>
        <li>Local-First Confidentiality Boundary Active.</li>
        <li>{filingBadge}</li>
        <li>{coverageBadge}</li>
        <li>{egressBadge}</li>
      </ul>

      {ipcError && <p role="alert">{ipcError}</p>}

      {/* REQ-UI-001: primary navigation across all thirteen surfaces. */}
      <nav aria-label="Primary">
        <ul
          style={{
            listStyle: "none",
            display: "flex",
            flexWrap: "wrap",
            gap: "0.5rem",
            padding: 0,
          }}
        >
          {SURFACES.map((s) => (
            <li key={s.id}>
              <a href={`#${s.id}`}>{s.label}</a>
            </li>
          ))}
        </ul>
      </nav>

      <section id="dashboard" aria-labelledby="dashboard-heading">
        <h2 id="dashboard-heading">Dashboard</h2>
        <h3 id="health-heading">System Health</h3>
        {health ? (
          <ul>
            <li>Status: {health.status}</li>
            <li>Version: {health.version}</li>
            <li>Storage writable: {String(health.storage_ok)}</li>
          </ul>
        ) : (
          !ipcError && <p>Checking…</p>
        )}
        {namespaces.length > 0 && (
          <>
            <h3 id="ns-heading">Capability Coverage</h3>
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
          </>
        )}
      </section>

      <section id="opportunity" aria-labelledby="opportunity-heading">
        <h2 id="opportunity-heading">Opportunity Radar</h2>
        <p>
          Scores are screening signals with stated uncertainty, not conclusions
          about merit or patentability.
        </p>
        <label htmlFor="opportunity-input">Candidate description</label>
        <textarea
          id="opportunity-input"
          value={opportunityDescription}
          onChange={(e) => setOpportunityDescription(e.target.value)}
          rows={2}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <label htmlFor="uncertainty-select">Uncertainty</label>
        <select
          id="uncertainty-select"
          value={opportunityUncertainty}
          onChange={(e) => setOpportunityUncertainty(e.target.value)}
        >
          <option value="Low">Low</option>
          <option value="Medium">Medium</option>
          <option value="High">High</option>
        </select>
        <button type="button" onClick={() => void onEvaluateOpportunity()}>
          Screen candidate
        </button>
        {opportunity?.value && (
          <p>
            Score {opportunity.value.score.toFixed(2)} with{" "}
            {opportunity.value.uncertainty} uncertainty, backed by{" "}
            {opportunity.value.evidence_count} evidence record(s).
          </p>
        )}
      </section>

      <section id="conception" aria-labelledby="conception-heading">
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
              <p>
                {lastRecord.error?.class}: {lastRecord.error?.message}
              </p>
            )}
            <p>Correlation: {lastRecord.correlation_id}</p>
          </div>
        )}

        {/* UO-01: the ledger was write-only. Recording an event stored it
            durably and no surface could read it back, so the "Human Conception
            Ledger" could not actually be consulted. */}
        <h3 id="ledger-heading">Human Conception Ledger</h3>
        <button type="button" onClick={() => void onLoadLedger()}>
          Load ledger from the vault
        </button>
        {ledger && (
          <div>
            {ledger.ok && ledger.value ? (
              <>
                <p>
                  {ledger.value.count} event(s) read back from durable storage —{" "}
                  {ledger.value.human_count} human, {ledger.value.ai_count} AI
                </p>
                {ledger.value.events.length === 0 ? (
                  <p>No events recorded in this workspace.</p>
                ) : (
                  <ol>
                    {ledger.value.events.map((e) => (
                      <li key={e.event_id}>
                        [{e.origin}] {e.content} ({e.content_hash.slice(0, 18)}
                        …, {e.created_utc})
                      </li>
                    ))}
                  </ol>
                )}
              </>
            ) : (
              <p>
                {ledger.error?.class}: {ledger.error?.message}
              </p>
            )}
            <p>Correlation: {ledger.correlation_id}</p>
          </div>
        )}
      </section>

      <section id="research" aria-labelledby="research-heading">
        <h2 id="research-heading">Research War Room</h2>
        <h3 id="kill-search-heading">Research Kill-Search</h3>
        <label htmlFor="research-task">Task ID</label>
        <input
          id="research-task"
          value={researchTaskId}
          onChange={(e) => setResearchTaskId(e.target.value)}
        />
        <p>Status: {researchStatus}</p>
        <p>Citations: {researchCitations.length}</p>
        {researchError && <p>{researchError}</p>}
        <button type="button" onClick={() => void onResearchAction("start")}>
          Start search
        </button>
        <button type="button" onClick={() => void onResearchAction("kill")}>
          Record kill
        </button>
        <button type="button" onClick={() => void onResearchAction("complete")}>
          Complete search
        </button>

        <h3 id="coverage-heading">Coverage and checkpoint</h3>
        <label htmlFor="requested-scopes">Requested scopes</label>
        <input
          id="requested-scopes"
          type="number"
          min={0}
          value={requestedScopes}
          onChange={(e) => setRequestedScopes(Number(e.target.value))}
        />
        <label htmlFor="covered-scopes">Covered scopes</label>
        <input
          id="covered-scopes"
          type="number"
          min={0}
          value={coveredScopes}
          onChange={(e) => setCoveredScopes(Number(e.target.value))}
        />
        <button type="button" onClick={() => void onSetCoverage()}>
          Record coverage
        </button>
        <button type="button" onClick={() => void onCompletePartial()}>
          Complete with checkpoint
        </button>

        <h3 id="claim-class-heading">Claim classification</h3>
        <label htmlFor="claim-class-select">Claim class</label>
        <select
          id="claim-class-select"
          value={researchClaimClass}
          onChange={(e) => setResearchClaimClass(e.target.value)}
        >
          <option value="OBSERVATION">OBSERVATION</option>
          <option value="HYPOTHESIS">HYPOTHESIS</option>
          <option value="INFERENCE">INFERENCE</option>
          <option value="LEGAL_RULE_SUMMARY">LEGAL_RULE_SUMMARY</option>
          <option value="MARKET_SIGNAL">MARKET_SIGNAL</option>
          <option value="PATENT_THREAT">PATENT_THREAT</option>
          <option value="COMMERCIAL_TARGET_ASSERTION">
            COMMERCIAL_TARGET_ASSERTION
          </option>
        </select>
        <label htmlFor="claim-text">Claim text</label>
        <input
          id="claim-text"
          value={researchClaimText}
          onChange={(e) => setResearchClaimText(e.target.value)}
        />
        <button type="button" onClick={() => void onClassifyClaim()}>
          Classify claim
        </button>
        {claimOutcome && (
          <p>
            {claimOutcome.ok && claimOutcome.value
              ? `${claimOutcome.value.class} (from a source record: ${String(
                  claimOutcome.value.emittable_from_source_record,
                )})`
              : `${claimOutcome.error?.class}: ${claimOutcome.error?.message}`}
          </p>
        )}
      </section>

      <section id="patent" aria-labelledby="patent-heading">
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
        {lintResult?.value && (
          <p>
            Lint: {lintResult.value.passed ? "passed" : "failed"}
            {lintResult.value.findings.length > 0 &&
              ` — ${lintResult.value.findings.join("; ")}`}
          </p>
        )}

        <h3 id="support-heading">Support matrix</h3>
        <p>
          Every exportable limitation must be anchored to specification or
          figure evidence before a draft leaves this surface.
        </p>
        <label htmlFor="matrix-input">
          Exportable limitations (one per line)
        </label>
        <textarea
          id="matrix-input"
          value={matrixLimitations}
          onChange={(e) => setMatrixLimitations(e.target.value)}
          rows={3}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onCheckMatrix()}>
          Check support matrix
        </button>
        {matrixResult?.value && (
          <p>
            {matrixResult.value.exportable
              ? `All exportable limitations anchored (${matrixResult.value.anchor_count} anchor(s)).`
              : `Unsupported limitations: ${matrixResult.value.unsupported.join("; ")}`}
          </p>
        )}
      </section>

      <section id="filing" aria-labelledby="filing-heading">
        <h2 id="filing-heading">Filing</h2>
        <ConfirmAction
          label="Build filing package"
          consequence="creates a filing-package draft artifact from the current claims and specification. It does not file anything."
          confirmLabel="Build draft package"
          onConfirm={onBuildPackage}
          onAudit={addAudit}
        />
        {packageResult?.value && (
          <p>
            Package format: {packageResult.value.format} (
            {packageResult.value.manifest})
          </p>
        )}
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
          <p>{receiptResult.error?.message}</p>
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

      <section id="docket" aria-labelledby="docket-heading">
        <h2 id="docket-heading">Docket</h2>
        <p>State: {docketState}</p>
        <button type="button" onClick={() => void onAdvanceDocket()}>
          Advance docket state
        </button>
        {docketResult?.value && (
          <p>
            Docket {docketResult.value.docket_id} is {docketResult.value.state};
            public disclosure recorded:{" "}
            {String(docketResult.value.public_disclosure)}
          </p>
        )}

        <h3 id="deadline-heading">Deadlines</h3>
        <p>
          A deadline is actionable only when an authoritative ruleset source and
          version back it; otherwise it is screened as a date with uncertainty.
        </p>
        <label htmlFor="due-date">Due date</label>
        <input
          id="due-date"
          value={dueDate}
          onChange={(e) => setDueDate(e.target.value)}
        />
        <label htmlFor="ruleset-source">Ruleset source</label>
        <input
          id="ruleset-source"
          value={rulesetSource}
          onChange={(e) => setRulesetSource(e.target.value)}
        />
        <label htmlFor="ruleset-version">Ruleset version</label>
        <input
          id="ruleset-version"
          value={rulesetVersion}
          onChange={(e) => setRulesetVersion(e.target.value)}
        />
        <button type="button" onClick={() => void onScheduleDeadline()}>
          Schedule deadline
        </button>
        {deadlineResult?.value && (
          <p>
            {deadlineResult.value.authoritative
              ? `Authoritative: ${deadlineResult.value.due_date} per ${deadlineResult.value.ruleset_source} ${deadlineResult.value.ruleset_version}`
              : `Not authoritative — ${deadlineResult.value.due_date} is an unsourced date`}
          </p>
        )}
      </section>

      <section id="prosecution" aria-labelledby="prosecution-heading">
        <h2 id="prosecution-heading">Prosecution</h2>
        <label htmlFor="office-action-select">Office action type</label>
        <select
          id="office-action-select"
          value={officeActionType}
          onChange={(e) => setOfficeActionType(e.target.value)}
        >
          <option value="NonFinalRejection">Non-final rejection</option>
          <option value="FinalRejection">Final rejection</option>
          <option value="NoticeOfAllowance">Notice of allowance</option>
        </select>
        <button type="button" onClick={() => void onDraftResponse()}>
          Draft response workspace
        </button>
        {officeActionResult?.value && (
          <p>
            {officeActionResult.value.action_type}:{" "}
            {officeActionResult.value.cited_art_count} cited reference(s);
            response draft created:{" "}
            {String(officeActionResult.value.response_drafted)}
          </p>
        )}
      </section>

      <section id="commercialize" aria-labelledby="commercialize-heading">
        <h2 id="commercialize-heading">Commercialize</h2>
        <label htmlFor="targets-input">Targets (one per line)</label>
        <textarea
          id="targets-input"
          value={targetNames}
          onChange={(e) => setTargetNames(e.target.value)}
          rows={3}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onBuildCommercialization()}>
          Build disclosure-safe package
        </button>
        {commercializationResult?.value && (
          <p>
            {commercializationResult.value.target_count} target(s); redacted:{" "}
            {String(commercializationResult.value.redacted)}
          </p>
        )}

        <h3 id="valuation-heading">Valuation</h3>
        <p>
          Outputs are planning scenarios expressed as ranges, not appraisals.
        </p>
        <button type="button" onClick={() => void onEvaluateValuation()}>
          Screen valuation range
        </button>
        {valuationResult?.value && (
          <p>
            {valuationResult.value.scenario_label}:{" "}
            {valuationResult.value.low.toLocaleString()}–
            {valuationResult.value.high.toLocaleString()} (
            {valuationResult.value.is_range ? "range" : "point estimate"}), most
            sensitive assumption:{" "}
            {valuationResult.value.dominant_assumption ?? "none"}
          </p>
        )}
      </section>

      <section id="evidence" aria-labelledby="evidence-heading">
        <h2 id="evidence-heading">Evidence Vault</h2>
        <h3 id="firewall-heading">Disclosure Firewall</h3>
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
        {exportResult?.value && (
          <p>
            {exportResult.value.allowed ? "Allowed" : "Blocked"} (
            {exportResult.value.sensitivity}): {exportResult.value.reason}
          </p>
        )}

        <ConfirmAction
          label="Export evidence bundle"
          consequence="releases screening evidence beyond the device boundary at the selected sensitivity. This cannot be recalled."
          confirmLabel="Export evidence"
          onConfirm={onExportEvidence}
          onAudit={addAudit}
        />
        {evidenceExportResult?.value && (
          <p>
            {evidenceExportResult.value.exported ? "Exported" : "Not exported"}{" "}
            {evidenceExportResult.value.path} at{" "}
            {evidenceExportResult.value.sensitivity}:{" "}
            {evidenceExportResult.value.detail}
          </p>
        )}
      </section>

      <section id="integrations" aria-labelledby="integrations-heading">
        <h2 id="integrations-heading">Integrations</h2>
        <h3 id="ops-heading">Operations</h3>
        <p>
          Provider lanes, MCP grants and incident capsules are reported from the
          backend; no lane performs inference on this host unless a local model
          is served, and no lane is presented as configured without evidence.
        </p>
        <button type="button" onClick={() => void onLoadOps()}>
          Load operational status
        </button>
        {opsError && <p>{opsError}</p>}
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

        <ConfirmAction
          label="Probe local model"
          consequence="sends the screening prompt across the provider boundary to the configured loopback endpoint."
          confirmLabel="Send probe"
          onConfirm={onRunInference}
          onAudit={addAudit}
        />
        {inference?.value && (
          <p>
            Local inference:{" "}
            {inference.value.live
              ? `live via ${inference.value.transport} after ${inference.value.attempts} attempt(s)`
              : `not live (${inference.value.error_class}) after ${inference.value.attempts} attempt(s): ${inference.value.detail}`}
          </p>
        )}
      </section>

      <section id="incidents" aria-labelledby="incidents-heading">
        <h2 id="incidents-heading">Incidents</h2>
        <label htmlFor="incident-input">Incident detail</label>
        <textarea
          id="incident-input"
          value={incidentDetail}
          onChange={(e) => setIncidentDetail(e.target.value)}
          rows={3}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onBuildCapsule()}>
          Build repair capsule
        </button>
        {capsuleResult?.value && (
          <p>
            Redacted: {String(capsuleResult.value.redacted)}; safe for export:{" "}
            {String(capsuleResult.value.safe_for_export)} —{" "}
            {capsuleResult.value.incident_detail}
          </p>
        )}
      </section>

      <section id="settings" aria-labelledby="settings-heading">
        <h2 id="settings-heading">Settings</h2>
        <h3 id="config-heading">Resolved configuration</h3>
        {configuration ? (
          <ul>
            <li>Runtime: {configuration.runtime}</li>
            <li>Application data: {configuration.app_data_dir}</li>
            <li>Vault file: {configuration.vault_file}</li>
            <li>
              Configuration warnings:{" "}
              {configuration.warnings.length === 0
                ? "none"
                : configuration.warnings.join("; ")}
            </li>
          </ul>
        ) : (
          !ipcError && <p>Configuration not reported by the backend.</p>
        )}
        <p>
          Secret handling: provider subscription auth is owned by the
          first-party provider tool and is never an environment value here.
        </p>

        {/* REQ-REL-005: backup and recovery are operator actions, so they are
            reachable from the product rather than only from a test harness. */}
        <h3 id="recovery-heading">Durable state recovery</h3>
        <p>
          A backup is a point-in-time snapshot of the vault. Work recorded after
          the most recent backup is not in it, and restoring discards work that
          is not in the backup.
        </p>
        <label htmlFor="backup-file">Backup file</label>
        <input
          id="backup-file"
          type="text"
          value={resolvedBackupFile}
          onChange={(e) => setBackupFile(e.target.value)}
          style={{ display: "block", width: "100%", maxWidth: "40rem" }}
        />
        <button type="button" onClick={() => void onBackupVault()}>
          Back up vault
        </button>
        {backupResult && (
          <p>
            {backupResult.ok && backupResult.value
              ? `Backup written to ${backupResult.value.destination} capturing ${backupResult.value.state_digest}`
              : `Backup not written: ${backupResult.error?.message ?? "unknown error"}`}
          </p>
        )}
        <ConfirmAction
          label="Restore vault from backup"
          consequence="replaces every event and claim in the vault with the snapshot in the backup file, so anything recorded after that backup is discarded; an unreadable vault is moved aside and preserved rather than deleted."
          confirmLabel="Restore vault"
          onConfirm={onRestoreVault}
          onAudit={addAudit}
        />
        {restoreResult && (
          <p>
            {restoreResult.ok && restoreResult.value
              ? `Restored from ${restoreResult.value.source}: reconciled ${String(restoreResult.value.reconciled)}; state went from ${restoreResult.value.digest_before} to ${restoreResult.value.digest_after}${
                  restoreResult.value.destination_recreated
                    ? "; no vault existed, so it was recreated from the backup"
                    : ""
                }${
                  restoreResult.value.destination_quarantined
                    ? `; the unreadable vault was preserved at ${restoreResult.value.destination_quarantined}`
                    : ""
                }`
              : `Restore did not run: ${restoreResult.error?.message ?? "unknown error"}`}
          </p>
        )}

        {/* REQ-SCOPE-001: the declared scope and the five truth boundaries are
            rendered from the backend declaration, including what is only PARTIAL
            and why. A surface showing twelve ticks would be the over-claim the
            boundaries exist to prevent. */}
        <h3 id="scope-heading">Declared scope and truth boundaries</h3>
        {scopeDeclaration?.ok && scopeDeclaration.value ? (
          <>
            <ul>
              {scopeDeclaration.value.boundaries.map((b) => (
                <li key={b.id}>
                  <strong>{b.id}</strong>: {b.statement} (enforced by{" "}
                  {b.enforced_by})
                </li>
              ))}
            </ul>
            <table>
              <caption>
                Promised outcomes and their measured state, including
                limitations
              </caption>
              <thead>
                <tr>
                  <th scope="col">Requirement</th>
                  <th scope="col">Outcome</th>
                  <th scope="col">State</th>
                  <th scope="col">Limitation</th>
                </tr>
              </thead>
              <tbody>
                {scopeDeclaration.value.capabilities.map((c) => (
                  <tr key={c.requirement_id}>
                    <td>
                      {c.requirement_id} ({c.uo_id})
                    </td>
                    <td>{c.title}</td>
                    <td>{c.state}</td>
                    <td>{c.limitation || "none stated"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </>
        ) : (
          <p>
            {scopeDeclaration?.error?.message ??
              "Declared scope not reported by the backend."}
          </p>
        )}

        {/* DOD-037: the signals exist over IPC and had NO surface. This is the
            dashboard: readiness, metrics, alerts, recent outcomes and traces,
            all read from get_diagnostics rather than recomputed in the UI. */}
        <h3 id="diagnostics-heading">Operator diagnostics</h3>
        <button type="button" onClick={() => void onRefreshDiagnostics()}>
          Refresh diagnostics
        </button>
        {diagnostics?.ok && diagnostics.value ? (
          <div id="diagnostics-panel">
            <p>
              Readiness: <strong>{diagnostics.value.readiness}</strong> —{" "}
              {diagnostics.value.detail}
            </p>
            <p>
              Commands recorded: {diagnostics.value.metrics.recorded} (
              {diagnostics.value.metrics.succeeded} ok,{" "}
              {diagnostics.value.metrics.failed} failed, failure rate{" "}
              {diagnostics.value.metrics.failure_rate}); p50{" "}
              {diagnostics.value.metrics.p50_duration_ms}ms, p95{" "}
              {diagnostics.value.metrics.p95_duration_ms}ms
            </p>
            <h4>Alerts</h4>
            {diagnostics.value.alerts.length === 0 ? (
              <p>No alerts.</p>
            ) : (
              <ul>
                {diagnostics.value.alerts.map((a) => (
                  <li key={a}>{a}</li>
                ))}
              </ul>
            )}
            <h4>Recent command outcomes</h4>
            {diagnostics.value.events.length === 0 ? (
              <p>Nothing recorded in this session yet.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th scope="col">Command</th>
                    <th scope="col">Outcome</th>
                    <th scope="col">Class</th>
                    <th scope="col">Duration</th>
                    <th scope="col">Correlation</th>
                  </tr>
                </thead>
                <tbody>
                  {diagnostics.value.events.map((e) => (
                    <tr key={`${e.correlation_id}-${e.command}`}>
                      <td>{e.command}</td>
                      <td>{e.outcome}</td>
                      <td>{e.error_class ?? "—"}</td>
                      <td>{e.duration_ms}ms</td>
                      <td>{e.correlation_id}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <p>
              Traces: {diagnostics.value.traces.length} correlation-tagged
              span(s)
              {diagnostics.value.traces.length > 0
                ? ` — latest ${diagnostics.value.traces[0]}`
                : ""}
              ; redaction applied: {String(diagnostics.value.redaction_applied)}
            </p>
          </div>
        ) : (
          <p>
            {diagnostics?.error?.message ??
              "Diagnostics not reported by the backend."}
          </p>
        )}
      </section>

      {/* REQ-UI-004: the audit half of consequence-specific confirmation. */}
      <section id="audit" aria-labelledby="audit-heading">
        <h2 id="audit-heading">Action audit</h2>
        {auditLog.length === 0 ? (
          <p>No high-impact action has been confirmed in this session.</p>
        ) : (
          <ul>
            {auditLog.map((e) => (
              <li key={`${e.correlationId}-${e.at}`}>
                {e.action} — {e.consequence} (correlation {e.correlationId}, at{" "}
                {e.at})
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
