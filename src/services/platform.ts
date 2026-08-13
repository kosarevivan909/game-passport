import { invoke } from "@tauri-apps/api/core";
import type { DisplayCommandResponse } from "../domain/display";
import type { NvidiaCommandResponse } from "../domain/nvidia";
import type { MouseCommandResponse } from "../domain/mouse";
import type { PubgCommandResponse } from "../domain/pubg";
import type { FileCommandResponse, ReleasePreflight } from "../domain/release";

export interface PlatformInfo { os: string; version: string; architecture: string; desktopRuntime: boolean }

export function isTauriDesktop() {
  return Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

export async function getPlatformInfo(): Promise<PlatformInfo> {
  if (isTauriDesktop()) {
    return invoke<PlatformInfo>("get_platform_info");
  }
  return { os: navigator.platform || "Browser", version: navigator.userAgent, architecture: "unknown", desktopRuntime: false };
}

export async function getDisplayDiagnostics(): Promise<DisplayCommandResponse> {
  if (!isTauriDesktop()) return { state: "unsupported", message: "Unsupported on this platform.", details: [], retryable: false };
  return invoke<DisplayCommandResponse>("get_display_diagnostics");
}

export async function getNvidiaDiagnostics(): Promise<NvidiaCommandResponse> {
  if (!isTauriDesktop()) return { state: "unsupported", message: "Unsupported on this platform.", details: [], retryable: false };
  return invoke<NvidiaCommandResponse>("get_nvidia_diagnostics");
}

export async function getMouseDiagnostics(): Promise<MouseCommandResponse> {
  if (!isTauriDesktop()) return { state: "unsupported", message: "Mouse hardware functions are unsupported on this platform.", details: [], retryable: false };
  return invoke<MouseCommandResponse>("get_mouse_diagnostics");
}

export async function getPubgDiagnostics(): Promise<PubgCommandResponse> {
  if (!isTauriDesktop()) return { state: "unsupported", message: "PUBG diagnostics require Windows desktop.", details: [], retryable: false };
  return invoke<PubgCommandResponse>("get_pubg_diagnostics");
}

export async function getReleasePreflight(): Promise<ReleasePreflight> {
  if (!isTauriDesktop()) return {
    appVersion: "0.6.0", build: "browser-preview", windowsVersion: navigator.platform || "Browser",
    windowsSupported: false, steamInstalled: false, steamUserAvailable: false, steamPath: null, cs2Installed: false,
    pubgConfigAvailable: false, logDirectory: null, administratorRequired: false,
    updateChannel: "Manual signed releases (updater not activated)"
  };
  return invoke<ReleasePreflight>("get_release_preflight");
}

export async function saveDiagnosticReport(contents: string): Promise<FileCommandResponse> {
  if (!isTauriDesktop()) return { state: "unsupported", message: "Saving a report file requires the Windows desktop build." };
  return invoke<FileCommandResponse>("save_diagnostic_report", { contents });
}
