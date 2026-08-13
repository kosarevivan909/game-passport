import { invoke } from "@tauri-apps/api/core";
import type { Cs2CommandResponse, Cs2Payload } from "../domain/cs2";
import { getCs2Payload } from "../domain/cs2";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

function mapResponse(response: Cs2CommandResponse, settingsPatch?: Record<string, unknown>): AdapterResult {
  return {
    adapterId: "game.cs2",
    label: "CS2 Settings",
    state: response.state,
    message: response.message,
    details: response.details,
    retryable: response.retryable,
    ...(settingsPatch && { settingsPatch })
  };
}

export class Cs2Adapter implements SettingsAdapter {
  readonly id = "game.cs2";
  readonly label = "CS2 Settings";
  readonly status = "implemented" as const;

  supports(profile: GameProfile) { return profile.game === "cs2"; }

  async capture(_profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    try {
      const response = await invoke<Cs2CommandResponse>("capture_cs2_settings");
      const patch = response.payload ? { adapters: { [this.id]: response.payload } } : undefined;
      return mapResponse(response, patch);
    } catch (error) { return this.runtimeError(error); }
  }

  async apply(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const payload = getCs2Payload(profile.settings);
    if (!payload) return { adapterId: this.id, label: this.label, state: "warning", message: "This profile has no saved CS2 settings yet.", retryable: false };
    try {
      const response = await invoke<Cs2CommandResponse>("apply_cs2_settings", { payload: payload as Cs2Payload });
      return mapResponse(response);
    } catch (error) { return this.runtimeError(error); }
  }

  async restore(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    try {
      const response = await invoke<Cs2CommandResponse>("restore_cs2_settings");
      return mapResponse(response);
    } catch (error) { return this.runtimeError(error); }
  }

  private desktopRequired(): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "CS2 settings require the Windows desktop application.", retryable: false };
  }

  private runtimeError(error: unknown): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "error", message: "The Windows CS2 service did not complete the operation.", details: [error instanceof Error ? error.message : String(error)], retryable: true };
  }
}
