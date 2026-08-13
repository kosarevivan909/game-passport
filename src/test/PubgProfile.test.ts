import { describe, expect, it } from "vitest";
import { DisplayAdapter } from "../adapters/DisplayAdapter";
import { MouseAdapter } from "../adapters/MouseAdapter";
import { NvidiaAdapter } from "../adapters/NvidiaAdapter";
import { PubgAdapter } from "../adapters/PubgAdapter";
import { displayRequestFromProfile } from "../domain/display";
import { getPubgPayload, type PubgPayload } from "../domain/pubg";
import type { GameProfile } from "../domain/types";

const payload: PubgPayload = { schemaVersion: 1, capturedAt: "2026-01-01T00:00:00Z", game: "pubg", files: [{ relativeId: "GameUserSettings.ini", sha256: "abc", entries: [] }], normalized: { width: 1728, height: 1080, displayMode: "fullscreen" }, capturedCategories: { gameplay: 3, keybinds: 2, graphics: 4, audio: 1 }, unsupportedCategories: [], warnings: [] };
const profile: GameProfile = { id: "p", userId: "u", name: "PUBG", game: "pubg", settings: { adapters: { pubg: payload } }, createdAt: "now", updatedAt: "now" };

describe("PUBG profile integration", () => {
  it("keeps the versioned payload in the generic Supabase settings shape", () => expect(getPubgPayload(profile.settings)).toEqual(payload));
  it("feeds exact resolution to the existing Display Adapter policy", () => expect(displayRequestFromProfile("pubg", profile.settings)).toEqual({ width: 1728, height: 1080, displayMode: "fullscreen" }));
  it("uses shared Display, NVIDIA and Mouse adapters", () => { expect(new DisplayAdapter().supports(profile)).toBe(true); expect(new NvidiaAdapter().supports(profile)).toBe(true); expect(new MouseAdapter().supports(profile)).toBe(true); });
  it("does not claim browser mock success", async () => { const result = await new PubgAdapter().capture(); expect(result.state).toBe("unsupported"); expect(result.message).not.toMatch(/captured|success/i); });
});
