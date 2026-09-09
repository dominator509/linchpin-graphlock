export interface JobEnvelope {
  jobId: string;
  payload: Record<string, unknown>;
}

export interface SystemHealth {
  status: string;
  version: string;
  storage_ok: boolean;
}
