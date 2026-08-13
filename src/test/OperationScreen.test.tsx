import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UnsupportedAdapter } from "../adapters/UnsupportedAdapter";
import { OperationScreen } from "../components/OperationScreen";
import type { GameProfile } from "../domain/types";
import { ProfileOrchestrator } from "../services/profileOrchestrator";

const profile: GameProfile = { id: "p1", userId: "u1", name: "CS2 Main", game: "cs2", settings: {}, createdAt: "2026-01-01", updatedAt: "2026-01-01" };

describe("OperationScreen", () => {
  it("never labels an unsupported capture as saved", async () => {
    const onCapture = vi.fn();
    render(<OperationScreen profile={profile} operation="capture" orchestrator={new ProfileOrchestrator([new UnsupportedAdapter("game.cs2", "CS2 Settings", "cs2")])} onCapture={onCapture} onClose={() => undefined} />);
    await waitFor(() => expect(screen.getByRole("heading", { name: "CAPTURE NOT AVAILABLE" })).toBeInTheDocument());
    expect(screen.queryByText("SETTINGS SAVED")).not.toBeInTheDocument();
    expect(onCapture).not.toHaveBeenCalled();
  });

  it("returns software evidence without converting unsupported to success", async () => {
    const onComplete = vi.fn();
    render(<OperationScreen profile={profile} operation="apply" orchestrator={new ProfileOrchestrator([new UnsupportedAdapter("game.cs2", "CS2 Settings", "cs2")])} onComplete={onComplete} onClose={() => undefined} />);
    await waitFor(() => expect(onComplete).toHaveBeenCalledOnce());
    expect(onComplete.mock.calls[0][0][0].state).toBe("unsupported");
    expect(screen.getByRole("heading", { name: "SETUP NOT APPLIED" })).toBeInTheDocument();
  });
});
