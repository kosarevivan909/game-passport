import { invoke } from "@tauri-apps/api/core";
import { getPubgPayload, type PubgCommandResponse, type PubgPayload } from "../domain/pubg";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

function map(response: PubgCommandResponse, patch?: Record<string, unknown>): AdapterResult {
  return { adapterId: "game.pubg", label: "PUBG Settings", state: response.state, message: response.message, details: response.details, retryable: response.retryable, ...(patch && { settingsPatch: patch }) };
}

export class PubgAdapter implements SettingsAdapter {
  readonly id = "game.pubg";
  readonly label = "PUBG Settings";
  readonly status = "implemented" as const;
  private backupToken?: string;
  supports(profile: GameProfile) { return profile.game === "pubg"; }
  async capture(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.unsupported();
    try { const response = await invoke<PubgCommandResponse>("capture_pubg_settings"); return map(response, response.payload ? { adapters: { pubg: response.payload } } : undefined); }
    catch (error) { return this.error(error); }
  }
  async apply(profile: GameProfile): Promise<AdapterResult> {
    if (!isTauriDesktop()) return this.unsupported();
    const payload = getPubgPayload(profile.settings);
    if (!payload) return { adapterId: this.id, label: this.label, state: "error", message: "This profile has no valid PUBG snapshot.", retryable: false };
    try { const response = await invoke<PubgCommandResponse>("apply_pubg_settings", { payload: payload as PubgPayload }); this.backupToken = response.backupToken; return map(response); }
    catch (error) { return this.error(error); }
  }
  async rollback(): Promise<AdapterResult> { if (!isTauriDesktop() || !this.backupToken) return { adapterId: `${this.id}.rollback`, label: "PUBG rollback", state: "unsupported", message: "No PUBG backup token is available." }; const response = await invoke<PubgCommandResponse>("restore_pubg_settings", { backupToken: this.backupToken }); return { ...map(response), adapterId: `${this.id}.rollback`, label: "PUBG rollback" }; }
  async restore(): Promise<AdapterResult> { if (!isTauriDesktop()) return this.unsupported(); try { return map(await invoke<PubgCommandResponse>("restore_pubg_settings", { backupToken: null })); } catch (error) { return this.error(error); } }
  private unsupported(): AdapterResult { return { adapterId: this.id, label: this.label, state: "unsupported", message: "PUBG settings are supported only in the Windows desktop application.", retryable: false }; }
  private error(error: unknown): AdapterResult { return { adapterId: this.id, label: this.label, state: "error", message: "The Windows PUBG service did not complete the operation.", details: [error instanceof Error ? error.message : String(error)], retryable: true }; }
}
