import { describe, expect, it } from "vitest";
import { getCs2Payload, mergeSettingsPatch } from "../domain/cs2";

describe("CS2 profile payload", () => {
  it("merges adapter data without deleting other settings", () => {
    const payload = { schemaVersion: 1, capturedAt: "now", files: [], totalBytes: 0, coreFilesFound: [], optionalFilesMissing: [] } as const;
    const merged = mergeSettingsPatch(
      { theme: "dark", adapters: { display: { width: 1920 } } },
      { adapters: { "game.cs2": payload } }
    );
    expect(merged.theme).toBe("dark");
    expect((merged.adapters as Record<string, unknown>).display).toEqual({ width: 1920 });
    expect(getCs2Payload(merged)?.schemaVersion).toBe(1);
  });

  it("merges normalized adapter fields without replacing its snapshot", () => {
    const merged = mergeSettingsPatch(
      { adapters: { display: { width: 1280, height: 960, refreshRatePolicy: "MAX_AVAILABLE" } } },
      { adapters: { display: { scalingPreference: "stretched" } } }
    );
    expect((merged.adapters as Record<string, Record<string, unknown>>).display).toEqual({ width: 1280, height: 960, refreshRatePolicy: "MAX_AVAILABLE", scalingPreference: "stretched" });
  });

  it("returns null for profiles without captured CS2 settings", () => {
    expect(getCs2Payload({})).toBeNull();
  });
});
