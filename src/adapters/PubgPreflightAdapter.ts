import { invoke } from "@tauri-apps/api/core";
import type { PubgCommandResponse } from "../domain/pubg";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

export class PubgPreflightAdapter implements SettingsAdapter {
  readonly id = "game.pubg.preflight";
  readonly label = "PUBG preflight";
  readonly status = "implemented" as const;
  supports(profile: GameProfile) { return profile.game === "pubg"; }
  async capture(): Promise<AdapterResult> { return this.run(); }
  async apply(): Promise<AdapterResult> { return this.run(); }
  async restore(): Promise<AdapterResult> { return this.run(); }
  private async run(): Promise<AdapterResult> { if (!isTauriDesktop()) return { adapterId: this.id, label: this.label, state: "unsupported", message: "PUBG preflight requires Windows desktop." }; try { const response = await invoke<PubgCommandResponse>("check_pubg_closed"); return { adapterId: this.id, label: this.label, state: response.state, message: response.message, details: response.details, retryable: response.retryable }; } catch (error) { return { adapterId: this.id, label: this.label, state: "error", message: "PUBG preflight failed.", details: [String(error)], retryable: true }; } }
}
