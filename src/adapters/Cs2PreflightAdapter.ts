import { invoke } from "@tauri-apps/api/core";
import type { Cs2CommandResponse } from "../domain/cs2";
import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { isTauriDesktop } from "../services/platform";

export class Cs2PreflightAdapter implements SettingsAdapter {
  readonly id = "preflight.cs2";
  readonly label = "CS2 closed";
  readonly status = "implemented" as const;

  supports(profile: GameProfile) { return profile.game === "cs2"; }

  async capture(): Promise<AdapterResult> {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "Preflight is apply-only." };
  }

  async apply(): Promise<AdapterResult> {
    if (!isTauriDesktop()) return { adapterId: this.id, label: this.label, state: "unsupported", message: "CS2 preflight requires Windows desktop." };
    try {
      const response = await invoke<Cs2CommandResponse>("check_cs2_closed");
      return { adapterId: this.id, label: this.label, state: response.state, message: response.message, details: response.details, retryable: response.retryable };
    } catch (error) {
      return { adapterId: this.id, label: this.label, state: "error", message: "Could not verify that CS2 is closed.", details: [String(error)], retryable: true };
    }
  }

  async restore(): Promise<AdapterResult> {
    return this.apply();
  }
}
