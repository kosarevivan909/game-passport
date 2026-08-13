import type { DiagnosticEntry } from "../domain/types";
import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "./platform";

const STORAGE_KEY = "game-passport.diagnostics.v1";
const MAX_ENTRIES = 250;

function read(): DiagnosticEntry[] {
  try { return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as DiagnosticEntry[]; }
  catch { return []; }
}

function write(entries: DiagnosticEntry[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(entries.slice(0, MAX_ENTRIES)));
  window.dispatchEvent(new Event("game-passport:diagnostics"));
}

function sanitized(value: string | undefined) {
  if (!value) return value;
  if (/authorization:|bearer\s|access_token|refresh_token|password=|cookie:|steamloginsecure/i.test(value)) return "[REDACTED: sensitive-looking value]";
  return value.replace(/eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g, "[REDACTED]").slice(0, 4000);
}

function classify(scope: string) {
  const parts = scope.split(".");
  const adapterParts = parts[0] === "game" && parts.length > 1 ? parts.splice(0, 2) : parts.splice(0, 1);
  return { adapter: adapterParts.join(".") || "application", operation: parts.join(".") || "event" };
}

export const logger = {
  list: read,
  clear: () => write([]),
  log(level: DiagnosticEntry["level"], scope: string, message: string, error?: unknown) {
    const rawDetails = error instanceof Error ? `${error.name}: ${error.message}\n${error.stack ?? ""}` : error ? String(error) : undefined;
    const details = sanitized(rawDetails);
    const safeMessage = sanitized(message) ?? "Event";
    const { adapter, operation } = classify(scope);
    const code = error instanceof Error ? error.name : undefined;
    const entry = { id: crypto.randomUUID(), timestamp: new Date().toISOString(), level, scope, operation, code, message: safeMessage, details };
    write([entry, ...read()]);
    if (isTauriDesktop()) void invoke("append_production_log", { entry: {
      timestamp: entry.timestamp, severity: level, adapter, operation, code,
      message: safeMessage, technicalDetails: details
    }}).catch(() => undefined);
  },
  info(scope: string, message: string) { this.log("info", scope, message); },
  warning(scope: string, message: string, error?: unknown) { this.log("warning", scope, message, error); },
  error(scope: string, message: string, error?: unknown) { this.log("error", scope, message, error); }
};
