export interface PubgIniEntry {
  section: string;
  key: string;
  operator: string;
  value: string;
  categories: Array<"gameplay" | "keybinds" | "graphics" | "audio">;
}

export interface PubgConfigFile { relativeId: "GameUserSettings.ini" | "Input.ini" | "Scalability.ini"; entries: PubgIniEntry[]; sha256: string }
export interface PubgCategorySummary { gameplay: number; keybinds: number; graphics: number; audio: number }
export interface PubgNormalizedSettings { width?: number; height?: number; displayMode: "fullscreen" | "borderless" | "windowed" | "unknown" }
export interface PubgPayload {
  schemaVersion: 1;
  capturedAt: string;
  game: "pubg";
  files: PubgConfigFile[];
  normalized: PubgNormalizedSettings;
  capturedCategories: PubgCategorySummary;
  unsupportedCategories: string[];
  warnings: string[];
}
export interface PubgDiagnostics {
  pubgDetected: boolean;
  installPath?: string;
  configDirectory?: string;
  configFilesFound: string[];
  processRunning: boolean;
  captureResult?: string;
  applyResult?: string;
  backupResult?: string;
  restoreResult?: string;
  categories: PubgCategorySummary;
  unsupportedSettings: string[];
  parseErrors: string[];
  writeErrors: string[];
}
export interface PubgCommandResponse { state: "success" | "warning" | "unsupported" | "error"; message: string; details: string[]; retryable: boolean; payload?: PubgPayload; diagnostics?: PubgDiagnostics; backupToken?: string }

export function getPubgPayload(settings: Record<string, unknown>): PubgPayload | null {
  const adapters = settings.adapters;
  if (!adapters || typeof adapters !== "object" || Array.isArray(adapters)) return null;
  const value = (adapters as Record<string, unknown>).pubg;
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const payload = value as Partial<PubgPayload>;
  return payload.schemaVersion === 1 && payload.game === "pubg" && Array.isArray(payload.files) ? payload as PubgPayload : null;
}
