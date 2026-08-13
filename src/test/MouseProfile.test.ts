import { describe, expect, it } from "vitest";
import { getMousePayload, normalizeDesiredDpi, normalizePollingRate } from "../domain/mouse";

describe("Mouse Passport profile", () => {
  it("stores normalized preferences without physical identity", () => {
    const payload = getMousePayload({ adapters: { mouse: { schemaVersion: 1, capturedAt: "2026-08-11", dpi: 800, pollingRateHz: 2000 } } });
    expect(payload).toEqual({ schemaVersion: 1, capturedAt: "2026-08-11", dpi: 800, pollingRateHz: 2000 });
    expect(payload).not.toHaveProperty("vendorId");
    expect(payload).not.toHaveProperty("devicePath");
    expect(payload).not.toHaveProperty("serialNumber");
  });

  it("supports exact and rounded DPI", () => {
    const caps = { minimum: 100, maximum: 3200, step: 50, values: [] };
    expect(normalizeDesiredDpi(800, caps)).toBe(800);
    expect(normalizeDesiredDpi(805, caps)).toBe(800);
    expect(normalizeDesiredDpi(99_000, caps)).toBe(3200);
  });

  it("falls back to the greatest polling rate not above the request", () => {
    expect(normalizePollingRate(2000, [125, 500, 1000])).toBe(1000);
    expect(normalizePollingRate(2000, [125, 1000, 2000, 4000])).toBe(2000);
  });

  it("rejects corrupted profile shapes", () => {
    expect(getMousePayload({ adapters: { mouse: { schemaVersion: 2, dpi: 800 } } })).toBeNull();
    expect(getMousePayload({ adapters: { mouse: { schemaVersion: 1, dpi: "800" } } })).toBeNull();
  });
});
