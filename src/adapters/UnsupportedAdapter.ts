import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";

export class UnsupportedAdapter implements SettingsAdapter {
  readonly status = "not_implemented" as const;
  constructor(readonly id: string, readonly label: string, private readonly game?: GameProfile["game"]) {}
  supports(profile: GameProfile) { return !this.game || profile.game === this.game; }
  private result(): AdapterResult {
    return { adapterId: this.id, label: this.label, state: "unsupported", message: "Not implemented in this version" };
  }
  async capture() { return this.result(); }
  async apply() { return this.result(); }
}
