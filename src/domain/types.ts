export const GAMES = ["cs2", "dota2", "pubg"] as const;
export type GameId = (typeof GAMES)[number];

export const GAME_META: Record<GameId, { name: string; short: string; accent: string }> = {
  cs2: { name: "Counter-Strike 2", short: "CS2", accent: "#ff9945" },
  dota2: { name: "Dota 2", short: "DOTA", accent: "#f04f4f" },
  pubg: { name: "PUBG: Battlegrounds", short: "PUBG", accent: "#f4c443" }
};

export type ProfileSettings = Record<string, unknown>;

export interface GameProfile {
  id: string;
  userId: string;
  name: string;
  game: GameId;
  settings: ProfileSettings;
  createdAt: string;
  updatedAt: string;
}

export interface SessionUser { id: string; email: string }

export type OperationState = "success" | "warning" | "unsupported" | "error";

export interface AdapterResult {
  adapterId: string;
  label: string;
  state: OperationState;
  message: string;
  details?: string[];
  retryable?: boolean;
  settingsPatch?: ProfileSettings;
}

export interface DiagnosticEntry {
  id: string;
  timestamp: string;
  level: "info" | "warning" | "error";
  scope: string;
  operation?: string;
  code?: string;
  message: string;
  details?: string;
}
