import { describe, expect, it } from "vitest";
import { UnsupportedAdapter } from "../adapters/UnsupportedAdapter";
import { ProfileOrchestrator } from "../services/profileOrchestrator";
import type { GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";

const profile: GameProfile = { id: "p1", userId: "u1", name: "Main", game: "cs2", settings: {}, createdAt: "2026-01-01", updatedAt: "2026-01-01" };

describe("ProfileOrchestrator", () => {
  it("reports unavailable adapters as unsupported", async () => {
    const orchestrator = new ProfileOrchestrator([new UnsupportedAdapter("display", "Display")]);
    expect(await orchestrator.apply(profile)).toEqual([{ adapterId: "display", label: "Display", state: "unsupported", message: "Not implemented in this version" }]);
  });

  it("runs only adapters that support the selected game", async () => {
    const orchestrator = new ProfileOrchestrator([new UnsupportedAdapter("cs2", "CS2", "cs2"), new UnsupportedAdapter("pubg", "PUBG", "pubg")]);
    expect(await orchestrator.capture(profile)).toHaveLength(1);
  });

  it("continues after an unsupported NVIDIA stage", async () => {
    const unsupported = new UnsupportedAdapter("nvidia", "NVIDIA");
    const cs2: SettingsAdapter = {
      id: "game.cs2", label: "CS2", status: "implemented", supports: () => true,
      capture: async () => ({ adapterId: "game.cs2", label: "CS2", state: "success", message: "captured" }),
      apply: async () => ({ adapterId: "game.cs2", label: "CS2", state: "success", message: "applied" })
    };
    const results = await new ProfileOrchestrator([unsupported, cs2]).apply(profile);
    expect(results.map((result) => result.state)).toEqual(["unsupported", "success"]);
  });

  it("rolls back completed stages after a critical apply failure", async () => {
    const display: SettingsAdapter = {
      id: "display", label: "Display", status: "implemented", supports: () => true,
      capture: async () => ({ adapterId: "display", label: "Display", state: "success", message: "captured" }),
      apply: async () => ({ adapterId: "display", label: "Display", state: "success", message: "applied" }),
      rollback: async () => ({ adapterId: "display.rollback", label: "Display rollback", state: "success", message: "restored" })
    };
    const failed: SettingsAdapter = {
      id: "game.cs2", label: "CS2", status: "implemented", supports: () => true,
      capture: async () => ({ adapterId: "game.cs2", label: "CS2", state: "error", message: "failed" }),
      apply: async () => ({ adapterId: "game.cs2", label: "CS2", state: "error", message: "failed" })
    };
    const results = await new ProfileOrchestrator([display, failed]).apply(profile);
    expect(results.map((result) => result.adapterId)).toEqual(["display", "game.cs2", "display.rollback"]);
  });

  it("passes captured patches to the next adapter", async () => {
    const first: SettingsAdapter = {
      id: "game.cs2", label: "CS2", status: "implemented", supports: () => true,
      capture: async () => ({ adapterId: "game.cs2", label: "CS2", state: "success", message: "captured", settingsPatch: { adapters: { "game.cs2": { resolution: "1280x960" } } } }),
      apply: async () => ({ adapterId: "game.cs2", label: "CS2", state: "success", message: "applied" })
    };
    let observed = false;
    const second: SettingsAdapter = {
      id: "display", label: "Display", status: "implemented", supports: () => true,
      capture: async (current) => {
        observed = Boolean((current.settings.adapters as Record<string, unknown>)["game.cs2"]);
        return { adapterId: "display", label: "Display", state: "success", message: "captured" };
      },
      apply: async () => ({ adapterId: "display", label: "Display", state: "success", message: "applied" })
    };
    await new ProfileOrchestrator([first, second]).capture(profile);
    expect(observed).toBe(true);
  });
});
