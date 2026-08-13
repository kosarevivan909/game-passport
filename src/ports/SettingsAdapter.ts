import type { AdapterResult, GameProfile } from "../domain/types";

export interface SettingsAdapter {
  readonly id: string;
  readonly label: string;
  readonly status: "implemented" | "not_implemented" | "unsupported";
  supports(profile: GameProfile): boolean;
  capture(profile: GameProfile): Promise<AdapterResult>;
  apply(profile: GameProfile): Promise<AdapterResult>;
  rollback?(): Promise<AdapterResult>;
  restore?(): Promise<AdapterResult>;
}
