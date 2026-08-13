export type Cs2FileScope = "userdata" | "install";

export interface Cs2ConfigFile {
  scope: Cs2FileScope;
  relativePath: string;
  contentBase64: string;
  sha256: string;
  size: number;
}

export interface Cs2Payload {
  schemaVersion: 1;
  capturedAt: string;
  files: Cs2ConfigFile[];
  totalBytes: number;
  coreFilesFound: string[];
  optionalFilesMissing: string[];
}

export interface Cs2CommandResponse {
  state: "success" | "warning" | "unsupported" | "error";
  message: string;
  details: string[];
  retryable: boolean;
  payload?: Cs2Payload;
}

export function getCs2Payload(settings: Record<string, unknown>): Cs2Payload | null {
  const adapters = settings.adapters;
  if (!adapters || typeof adapters !== "object" || Array.isArray(adapters)) return null;
  const value = (adapters as Record<string, unknown>)["game.cs2"];
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Cs2Payload;
}

export function mergeSettingsPatch(current: Record<string, unknown>, patch: Record<string, unknown>): Record<string, unknown> {
  const currentAdapters = current.adapters && typeof current.adapters === "object" && !Array.isArray(current.adapters) ? current.adapters as Record<string, unknown> : {};
  const patchAdapters = patch.adapters && typeof patch.adapters === "object" && !Array.isArray(patch.adapters) ? patch.adapters as Record<string, unknown> : {};
  const adapters = { ...currentAdapters };
  for (const [key, value] of Object.entries(patchAdapters)) {
    const existing = adapters[key];
    adapters[key] = existing && value && typeof existing === "object" && typeof value === "object" && !Array.isArray(existing) && !Array.isArray(value)
      ? { ...(existing as Record<string, unknown>), ...(value as Record<string, unknown>) }
      : value;
  }
  return { ...current, ...patch, adapters };
}
