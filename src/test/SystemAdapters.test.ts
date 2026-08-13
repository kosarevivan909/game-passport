import { describe, expect, it } from "vitest";
import { DisplayAdapter } from "../adapters/DisplayAdapter";
import { NvidiaAdapter } from "../adapters/NvidiaAdapter";
import { MouseAdapter } from "../adapters/MouseAdapter";
import type { GameProfile } from "../domain/types";

const profile: GameProfile = { id: "p", userId: "u", name: "CS2", game: "cs2", settings: {}, createdAt: "now", updatedAt: "now" };

describe("system adapters outside Windows desktop", () => {
  it("reports Display as unsupported without claiming success", async () => {
    expect((await new DisplayAdapter().apply(profile)).state).toBe("unsupported");
  });

  it("reports NVIDIA as unsupported and does not throw", async () => {
    expect((await new NvidiaAdapter().capture(profile)).state).toBe("unsupported");
  });

  it("reports physical mouse operations as unsupported outside Tauri desktop", async () => {
    const result = await new MouseAdapter().apply({ ...profile, settings: { adapters: { mouse: { schemaVersion: 1, capturedAt: "now", dpi: 800, pollingRateHz: 1000 } } } });
    expect(result.state).toBe("unsupported");
    expect(result.message).not.toMatch(/success/i);
  });
});
