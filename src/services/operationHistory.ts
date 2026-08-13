import type { AdapterResult, GameId } from "../domain/types";
import type { OperationRecord } from "../domain/release";

const STORAGE_KEY = "game-passport.operation-history.v1";
const MAX_RECORDS = 40;

function read(): OperationRecord[] {
  try { return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as OperationRecord[]; }
  catch { return []; }
}

export const operationHistory = {
  list: read,
  record(game: GameId, operation: OperationRecord["operation"], results: AdapterResult[]) {
    const state: AdapterResult["state"] = results.some((result) => result.state === "error") ? "error"
      : results.some((result) => result.state === "warning" || result.state === "unsupported") ? "warning" : "success";
    const record: OperationRecord = {
      id: crypto.randomUUID(), timestamp: new Date().toISOString(), game, operation, state,
      adapters: results.map(({ adapterId, state: adapterState, message }) => ({ adapterId, state: adapterState, message }))
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify([record, ...read()].slice(0, MAX_RECORDS)));
  }
};
