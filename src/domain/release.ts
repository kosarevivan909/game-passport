import type { AdapterResult, GameId } from "./types";

export const RELEASE_LABEL = "0.6.0 RC";

export interface ReleasePreflight {
  appVersion: string;
  build: string;
  windowsVersion: string;
  windowsSupported: boolean;
  steamInstalled: boolean;
  steamUserAvailable: boolean;
  steamPath?: string | null;
  cs2Installed: boolean;
  pubgConfigAvailable: boolean;
  logDirectory?: string | null;
  administratorRequired: boolean;
  updateChannel: string;
}

export interface FileCommandResponse {
  state: "success" | "warning" | "unsupported" | "error";
  message: string;
  path?: string | null;
}

export type CloudState = "checking" | "connected" | "offline" | "unavailable" | "demo" | "field-test";
export type FieldStatus = "pending" | "pass" | "fail" | "warning" | "skipped";

export interface FieldValidationEntry {
  id: string;
  game: Extract<GameId, "cs2" | "pubg">;
  stage: "capture" | "apply" | "gameplay" | "visual" | "display" | "nvidia" | "mouse";
  status: FieldStatus;
  updatedAt?: string;
  note?: string;
}

export interface OperationRecord {
  id: string;
  timestamp: string;
  game: GameId;
  operation: "capture" | "apply" | "restore";
  state: AdapterResult["state"];
  adapters: Array<Pick<AdapterResult, "adapterId" | "state" | "message">>;
}
