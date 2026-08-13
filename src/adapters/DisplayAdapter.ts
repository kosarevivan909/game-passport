import { invoke } from "@tauri-apps/api/core";
import { displayRequestFromProfile, getDisplayPayload, type DisplayCommandResponse, type DisplayPayload } from "../domain/display";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

function mapResponse(response: DisplayCommandResponse, settingsPatch?: Record<string, unknown>): AdapterResult {
  return {
    adapterId: "display",
    label: "Display",
    state: response.state,
    message: response.message,
    details: response.details,
    retryable: response.retryable,
    ...(settingsPatch && { settingsPatch })
  };
}

export class DisplayAdapter implements SettingsAdapter {
  readonly id = "display";
  readonly label = "Display";
  readonly status = "implemented" as const;
  private backupToken?: string;

  supports(profile: GameProfile) { return profile.game === "cs2" || profile.game === "pubg"; }

  async capture(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const request = displayRequestFromProfile(profile.game, profile.settings);
    if (!request) return {
      adapterId: this.id,
      label: this.label,
      state: "warning",
      message: `${profile.game === "pubg" ? "PUBG" : "CS2"} video resolution could not be read, so no Display snapshot was saved.`,
      details: [profile.game === "pubg" ? "GameUserSettings.ini must contain ResolutionSizeX and ResolutionSizeY." : "cs2_video.txt must contain setting.defaultres and setting.defaultresheight."],
      retryable: true
    };
    try {
      const response = await invoke<DisplayCommandResponse>("capture_display_settings", { request });
      const patch = response.payload ? { adapters: { [this.id]: response.payload } } : undefined;
      return mapResponse(response, patch);
    } catch (error) { return this.runtimeError(error); }
  }

  async apply(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const payload = getDisplayPayload(profile.settings);
    if (!payload) return { adapterId: this.id, label: this.label, state: "warning", message: "This profile has no saved Display policy.", retryable: false };
    try {
      const response = await invoke<DisplayCommandResponse>("apply_display_settings", { payload: payload as DisplayPayload });
      this.backupToken = response.backupToken;
      return mapResponse(response);
    } catch (error) { return this.runtimeError(error); }
  }

  async rollback(): Promise<AdapterResult> {
    if (!isTauriDesktop() || !this.backupToken) return { adapterId: `${this.id}.rollback`, label: "Display rollback", state: "unsupported", message: "No Display backup token is available." };
    const response = await invoke<DisplayCommandResponse>("restore_display_settings", { backupToken: this.backupToken });
    return { ...mapResponse(response), adapterId: `${this.id}.rollback`, label: "Display rollback" };
  }

  async restore(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.desktopRequired();
    const response = await invoke<DisplayCommandResponse>("restore_display_settings", { backupToken: null });
    return mapResponse(response);
  }

  private desktopRequired(): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "Display settings are unsupported on this platform.", retryable: false };
  }

  private runtimeError(error: unknown): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "error", message: "The Windows Display service did not complete the operation.", details: [error instanceof Error ? error.message : String(error)], retryable: true };
  }
}
