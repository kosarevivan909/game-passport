import { getCs2Payload } from "./cs2";
import { getPubgPayload } from "./pubg";
import type { GameId } from "./types";

export interface DisplayMode {
  width: number;
  height: number;
  refreshHz: number;
  bitsPerPixel: number;
  interlaced: boolean;
}

export interface DisplayPayload {
  schemaVersion: 1;
  capturedAt: string;
  width: number;
  height: number;
  aspectRatio: string;
  displayMode: "fullscreen" | "borderless" | "windowed" | "unknown";
  scalingPreference: string;
  refreshRatePolicy: "MAX_AVAILABLE";
}

export interface DisplayDiagnostics {
  monitorDetected: boolean;
  primaryMonitor?: string;
  primaryDevice?: string;
  monitorCount: number;
  currentMode?: DisplayMode;
  supportedModes: DisplayMode[];
  selectedMode?: DisplayMode;
  lastChangeResult?: string;
}

export interface DisplayCommandResponse {
  state: "success" | "warning" | "unsupported" | "error";
  message: string;
  details: string[];
  retryable: boolean;
  payload?: DisplayPayload;
  diagnostics?: DisplayDiagnostics;
  backupToken?: string;
}

export interface DisplayCaptureRequest {
  width: number;
  height: number;
  displayMode: DisplayPayload["displayMode"];
}

export function getDisplayPayload(settings: Record<string, unknown>): DisplayPayload | null {
  const adapters = settings.adapters;
  if (!adapters || typeof adapters !== "object" || Array.isArray(adapters)) return null;
  const value = (adapters as Record<string, unknown>).display;
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as DisplayPayload;
}

export function displayRequestFromCs2(settings: Record<string, unknown>): DisplayCaptureRequest | null {
  const payload = getCs2Payload(settings);
  const file = payload?.files.find((candidate) => candidate.relativePath.toLowerCase() === "cs2_video.txt");
  if (!file) return null;
  try {
    const binary = atob(file.contentBase64);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const text = new TextDecoder().decode(bytes);
    const width = numericSetting(text, "setting.defaultres");
    const height = numericSetting(text, "setting.defaultresheight");
    if (!width || !height) return null;
    const fullscreen = numericSetting(text, "setting.fullscreen");
    const borderless = numericSetting(text, "setting.nowindowborder");
    const displayMode: DisplayCaptureRequest["displayMode"] = fullscreen === 1
      ? "fullscreen"
      : borderless === 1 || fullscreen === 2 ? "borderless" : fullscreen === 0 ? "windowed" : "unknown";
    return { width, height, displayMode };
  } catch {
    return null;
  }
}

export function displayRequestFromProfile(game: GameId, settings: Record<string, unknown>): DisplayCaptureRequest | null {
  if (game === "cs2") return displayRequestFromCs2(settings);
  if (game !== "pubg") return null;
  const value = getPubgPayload(settings)?.normalized;
  return value?.width && value.height ? { width: value.width, height: value.height, displayMode: value.displayMode } : null;
}

function numericSetting(text: string, key: string) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = text.match(new RegExp(`["']?${escaped}["']?\\s*(?:=|:)?\\s*["']?(\\d+)`, "i"));
  return match ? Number(match[1]) : null;
}
