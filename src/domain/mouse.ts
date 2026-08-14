export interface DpiCapabilities {
  minimum: number;
  maximum: number;
  step: number;
  values: number[];
}

export interface MouseCapabilities {
  canReadDpi: boolean;
  canApplyDpi: boolean;
  canReadPollingRate: boolean;
  canApplyPollingRate: boolean;
  canVerify: boolean;
  dpi?: DpiCapabilities;
  pollingRatesHz: number[];
  reason?: string;
}

export interface MousePayload {
  schemaVersion: 1;
  capturedAt: string;
  dpi: number;
  pollingRateHz?: number;
}

export interface MouseDeviceDiagnostics {
  instanceId: string;
  vendorId: string;
  productId: string;
  manufacturer: string;
  model: string;
  connection: string;
  hidUsage: string;
  selectedAdapter: string;
  selected: boolean;
  capabilities: MouseCapabilities;
}

export interface MouseDiagnostics {
  devices: MouseDeviceDiagnostics[];
  probeErrors: string[];
  selectedInstanceId?: string;
  selectionAmbiguous: boolean;
  currentDpi?: number;
  requestedDpi?: number;
  appliedDpi?: number;
  currentPollingRateHz?: number;
  requestedPollingRateHz?: number;
  appliedPollingRateHz?: number;
  verificationResult?: string;
  backupResult?: string;
  restoreResult?: string;
}

export function createManualMousePayload(dpi: number, pollingRateHz?: number): MousePayload | null {
  if (!Number.isInteger(dpi) || dpi < 50 || dpi > 100_000) return null;
  if (pollingRateHz !== undefined && (!Number.isInteger(pollingRateHz) || pollingRateHz < 125 || pollingRateHz > 8_000)) return null;
  return {
    schemaVersion: 1,
    capturedAt: new Date().toISOString(),
    dpi,
    ...(pollingRateHz !== undefined && { pollingRateHz })
  };
}

export interface MouseCommandResponse {
  state: "success" | "warning" | "unsupported" | "error";
  message: string;
  details: string[];
  retryable: boolean;
  payload?: MousePayload;
  diagnostics?: MouseDiagnostics;
  backupToken?: string;
}

export function getMousePayload(settings: Record<string, unknown>): MousePayload | null {
  const adapters = settings.adapters;
  if (!adapters || typeof adapters !== "object" || Array.isArray(adapters)) return null;
  const value = (adapters as Record<string, unknown>).mouse;
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const candidate = value as Partial<MousePayload>;
  if (candidate.schemaVersion !== 1 || typeof candidate.dpi !== "number") return null;
  if (candidate.pollingRateHz !== undefined && typeof candidate.pollingRateHz !== "number") return null;
  return candidate as MousePayload;
}

export function normalizeDesiredDpi(desired: number, capabilities: DpiCapabilities): number | null {
  if (capabilities.values.length) {
    return [...capabilities.values].sort((a, b) => Math.abs(a - desired) - Math.abs(b - desired) || a - b)[0] ?? null;
  }
  if (capabilities.minimum > capabilities.maximum || capabilities.step <= 0) return null;
  const clamped = Math.min(capabilities.maximum, Math.max(capabilities.minimum, desired));
  const lower = capabilities.minimum + Math.floor((clamped - capabilities.minimum) / capabilities.step) * capabilities.step;
  const upper = Math.min(capabilities.maximum, lower + capabilities.step);
  return Math.abs(lower - clamped) <= Math.abs(upper - clamped) ? lower : upper;
}

export function normalizePollingRate(desired: number, supported: number[]): number | null {
  const rates = [...new Set(supported.filter((value) => value > 0))].sort((a, b) => a - b);
  return rates.filter((value) => value <= desired).at(-1) ?? rates[0] ?? null;
}
