import type { AdapterResult } from "../domain/types";
import type { FieldStatus, FieldValidationEntry } from "../domain/release";

const STORAGE_KEY = "game-passport.field-validation.v1";
const GAMES = ["cs2", "pubg"] as const;
const STAGES = ["capture", "apply", "gameplay", "visual", "display", "nvidia", "mouse"] as const;

function defaults(): FieldValidationEntry[] {
  return GAMES.flatMap((game) => STAGES.map((stage) => ({ id: `${game}.${stage}`, game, stage, status: "pending" as const })));
}

function read(): FieldValidationEntry[] {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as FieldValidationEntry[];
    const byId = new Map(stored.map((entry) => [entry.id, entry]));
    return defaults().map((entry) => byId.get(entry.id) ?? entry);
  } catch { return defaults(); }
}

function write(entries: FieldValidationEntry[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  window.dispatchEvent(new Event("game-passport:field-validation"));
}

export function validationStatus(results: AdapterResult[]): FieldStatus {
  if (results.length === 0) return "fail";
  if (results.some((result) => result.state === "error")) return "fail";
  if (results.some((result) => result.state === "warning" || result.state === "unsupported")) return "warning";
  return "pass";
}

export const fieldValidation = {
  list: read,
  set(id: string, status: FieldStatus, note?: string) {
    write(read().map((entry) => entry.id === id ? { ...entry, status, note, updatedAt: new Date().toISOString() } : entry));
  },
  recordSoftware(game: "cs2" | "pubg", stage: "capture" | "apply", results: AdapterResult[]) {
    this.set(`${game}.${stage}`, validationStatus(results), results.map((result) => `${result.label}: ${result.message}`).join(" | "));
  },
  reset(game?: "cs2" | "pubg") {
    write(game ? read().map((entry) => entry.game === game ? { ...entry, status: "pending", note: undefined, updatedAt: undefined } : entry) : defaults());
  }
};
