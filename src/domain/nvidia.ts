export interface NvidiaSetting {
  id: number;
  key: string;
  value: number;
}

export interface NvidiaPayload {
  schemaVersion: 1;
  capturedAt: string;
  profileExecutable: "cs2.exe" | "TslGame.exe";
  settings: NvidiaSetting[];
  scalingMode: string;
  scalingValue: number;
  scalingSupported: boolean;
}

export interface NvidiaDiagnostics {
  gpuDetected: boolean;
  gpuName?: string;
  driverAvailable: boolean;
  driverVersion?: number;
  driverBranch?: string;
  nvapiInitialized: boolean;
  cs2ProfileFound: boolean;
  cs2ProfileCreated: boolean;
  profileName?: string;
  settingsRead: number;
  settingsApplied: number;
  settingsSkipped: number;
  settingsUnsupported: number;
  scalingSupported: boolean;
  scalingMode?: string;
  scalingResult?: string;
}

export interface NvidiaCommandResponse {
  state: "success" | "warning" | "unsupported" | "error";
  message: string;
  details: string[];
  retryable: boolean;
  payload?: NvidiaPayload;
  diagnostics?: NvidiaDiagnostics;
  backupToken?: string;
}

export function getNvidiaPayload(settings: Record<string, unknown>): NvidiaPayload | null {
  const adapters = settings.adapters;
  if (!adapters || typeof adapters !== "object" || Array.isArray(adapters)) return null;
  const value = (adapters as Record<string, unknown>).nvidia;
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as NvidiaPayload;
}
