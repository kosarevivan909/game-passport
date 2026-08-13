import { invoke } from "@tauri-apps/api/core";
import { getNvidiaPayload, type NvidiaCommandResponse, type NvidiaPayload } from "../domain/nvidia";
import { getDisplayPayload } from "../domain/display";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

function mapResponse(response: NvidiaCommandResponse, settingsPatch?: Record<string, unknown>): AdapterResult {
  return {
    adapterId: "nvidia",
    label: "NVIDIA",
    state: response.state,
    message: response.message,
    details: response.details,
    retryable: response.retryable,
    ...(settingsPatch && { settingsPatch })
  };
}

export class NvidiaAdapter implements SettingsAdapter {
  readonly id = "nvidia";
  readonly label = "NVIDIA";
  readonly status = "implemented" as const;
  private backupToken?: string;

  supports(profile: GameProfile) { return profile.game === "cs2" || profile.game === "pubg"; }

  async capture(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    try {
      const response = await invoke<NvidiaCommandResponse>("capture_nvidia_settings", { request: { game: profile.game } });
      const patch = response.payload ? {
        adapters: {
          [this.id]: response.payload,
          ...(getDisplayPayload(profile.settings) && { display: { scalingPreference: response.payload.scalingMode } })
        }
      } : undefined;
      return mapResponse(response, patch);
    } catch (error) { return this.runtimeError(error); }
  }

  async apply(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const payload = getNvidiaPayload(profile.settings);
    if (!payload) return { adapterId: this.id, label: this.label, state: "unsupported", message: "This profile has no portable NVIDIA snapshot.", retryable: false };
    try {
      const response = await invoke<NvidiaCommandResponse>("apply_nvidia_settings", { payload: payload as NvidiaPayload });
      this.backupToken = response.backupToken;
      return mapResponse(response);
    } catch (error) { return this.runtimeError(error); }
  }

  async rollback(): Promise<AdapterResult> {
    if (!isTauriDesktop() || !this.backupToken) return { adapterId: `${this.id}.rollback`, label: "NVIDIA rollback", state: "unsupported", message: "No NVIDIA backup token is available." };
    const response = await invoke<NvidiaCommandResponse>("restore_nvidia_settings", { backupToken: this.backupToken });
    return { ...mapResponse(response), adapterId: `${this.id}.rollback`, label: "NVIDIA rollback" };
  }

  async restore(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const response = await invoke<NvidiaCommandResponse>("restore_nvidia_settings", { backupToken: null });
    return mapResponse(response);
  }

  private desktopRequired(): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "NVIDIA settings are unsupported on this platform.", retryable: false };
  }

  private runtimeError(error: unknown): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "error", message: "The NVIDIA NVAPI service did not complete the operation.", details: [error instanceof Error ? error.message : String(error)], retryable: true };
  }
}
