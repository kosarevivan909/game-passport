import { invoke } from "@tauri-apps/api/core";
import { getMousePayload, type MouseCommandResponse, type MousePayload } from "../domain/mouse";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

function mapResponse(response: MouseCommandResponse, settingsPatch?: Record<string, unknown>): AdapterResult {
  return {
    adapterId: "mouse",
    label: "Mouse Passport",
    state: response.state,
    message: response.message,
    details: response.details,
    retryable: response.retryable,
    ...(settingsPatch && { settingsPatch })
  };
}

export class MouseAdapter implements SettingsAdapter {
  readonly id = "mouse";
  readonly label = "Mouse Passport";
  readonly status = "implemented" as const;
  private backupToken?: string;

  supports(profile: GameProfile) { return profile.game === "cs2" || profile.game === "pubg"; }

  async capture(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    try {
      const response = await invoke<MouseCommandResponse>("capture_mouse_settings");
      return mapResponse(response, response.payload ? { adapters: { mouse: response.payload } } : undefined);
    } catch (error) { return this.runtimeError(error); }
  }

  async apply(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const payload = getMousePayload(profile.settings);
    if (!payload) return { adapterId: this.id, label: this.label, state: "warning", message: "This profile has no saved mouse preferences.", details: ["Capture the profile on a readable supported mouse first."], retryable: false };
    try {
      const response = await invoke<MouseCommandResponse>("apply_mouse_settings", { payload: payload as MousePayload });
      this.backupToken = response.backupToken;
      return mapResponse(response);
    } catch (error) { return this.runtimeError(error); }
  }

  async rollback(): Promise<AdapterResult> {
    if (!isTauriDesktop() || !this.backupToken) return { adapterId: `${this.id}.rollback`, label: "Mouse rollback", state: "unsupported", message: "No readable Mouse backup is available." };
    const response = await invoke<MouseCommandResponse>("restore_mouse_settings", { backupToken: this.backupToken });
    return { ...mapResponse(response), adapterId: `${this.id}.rollback`, label: "Mouse rollback" };
  }

  async restore(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    try { return mapResponse(await invoke<MouseCommandResponse>("restore_mouse_settings", { backupToken: null })); }
    catch (error) { return this.runtimeError(error); }
  }

  private desktopRequired(): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "Mouse hardware functions are unsupported on this platform.", retryable: false };
  }

  private runtimeError(error: unknown): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "error", message: "The Windows Mouse service did not complete the operation.", details: [error instanceof Error ? error.message : String(error)], retryable: true };
  }
}
