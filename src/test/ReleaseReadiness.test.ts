import type { SupabaseClient } from "@supabase/supabase-js";
import { beforeEach, describe, expect, it } from "vitest";
import type { AdapterResult, GameProfile } from "../domain/types";
import { SupabaseProfileRepository } from "../infrastructure/supabase/SupabaseProfileRepository";
import { fieldValidation, validationStatus } from "../services/fieldValidation";
import { logger } from "../services/logger";
import { operationHistory } from "../services/operationHistory";

const cachedProfile: GameProfile = { id: "p1", userId: "u1", name: "Club CS2", game: "cs2", settings: { adapters: {} }, createdAt: "2026-08-11", updatedAt: "2026-08-11" };

describe("0.6.0 RC release readiness", () => {
  beforeEach(() => localStorage.clear());

  it("uses a previously synchronized profile when Supabase is offline", async () => {
    localStorage.setItem("game-passport.profile-cache.v1.u1", JSON.stringify([cachedProfile]));
    const client = { from: () => ({ select: () => ({ eq: () => ({ order: async () => { throw new Error("offline"); } }) }) }) } as unknown as SupabaseClient;
    await expect(new SupabaseProfileRepository(client).list("u1")).resolves.toEqual([cachedProfile]);
  });

  it("does not turn warnings or failures into a field-test pass", () => {
    const warning: AdapterResult[] = [{ adapterId: "mouse", label: "Mouse", state: "unsupported", message: "Manual setup required" }];
    const failure: AdapterResult[] = [{ adapterId: "game.cs2", label: "CS2", state: "error", message: "Game is open" }];
    expect(validationStatus(warning)).toBe("warning");
    expect(validationStatus(failure)).toBe("fail");
    expect(validationStatus([])).toBe("fail");
  });

  it("stores software and user verification separately", () => {
    fieldValidation.recordSoftware("cs2", "capture", [{ adapterId: "game.cs2", label: "CS2", state: "success", message: "Captured" }]);
    fieldValidation.set("cs2.gameplay", "pass");
    const entries = fieldValidation.list();
    expect(entries.find((entry) => entry.id === "cs2.capture")?.status).toBe("pass");
    expect(entries.find((entry) => entry.id === "cs2.gameplay")?.status).toBe("pass");
    expect(entries.find((entry) => entry.id === "cs2.apply")?.status).toBe("pending");
  });

  it("keeps bounded sanitized support history without profile payloads", () => {
    operationHistory.record("pubg", "apply", [{ adapterId: "game.pubg", label: "PUBG", state: "warning", message: "Visual check required" }]);
    const record = operationHistory.list()[0];
    expect(record.state).toBe("warning");
    expect(record).not.toHaveProperty("profileId");
    expect(record).not.toHaveProperty("settings");
  });

  it("redacts sensitive-looking values from local diagnostics", () => {
    logger.error("auth.signin", "authorization: Bearer secret-token");
    expect(logger.list()[0].message).toBe("[REDACTED: sensitive-looking value]");
  });
});
