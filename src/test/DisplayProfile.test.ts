import { describe, expect, it } from "vitest";
import { displayRequestFromCs2 } from "../domain/display";

function settingsWithVideo(text: string) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return {
    adapters: {
      "game.cs2": {
        schemaVersion: 1,
        capturedAt: "now",
        files: [{ scope: "userdata", relativePath: "cs2_video.txt", contentBase64: btoa(binary), sha256: "x", size: bytes.length }],
        totalBytes: bytes.length,
        coreFilesFound: [],
        optionalFilesMissing: []
      }
    }
  };
}

describe("Display profile normalization", () => {
  it("reads the desired resolution and fullscreen mode from CS2", () => {
    const settings = settingsWithVideo('"setting.defaultres" "1280"\n"setting.defaultresheight" "960"\n"setting.fullscreen" "1"\n');
    expect(displayRequestFromCs2(settings)).toEqual({ width: 1280, height: 960, displayMode: "fullscreen" });
  });

  it("does not invent a resolution when CS2 video data is incomplete", () => {
    expect(displayRequestFromCs2(settingsWithVideo('"setting.defaultres" "1280"\n'))).toBeNull();
  });
});
