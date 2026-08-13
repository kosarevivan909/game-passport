import type { AdapterResult, GameProfile } from "../domain/types";
import type { SettingsAdapter } from "../ports/SettingsAdapter";
import { logger } from "./logger";
import { mergeSettingsPatch } from "../domain/cs2";

export class ProfileOrchestrator {
  constructor(private readonly adapters: SettingsAdapter[]) {}

  applicable(profile: GameProfile) {
    return this.adapters.filter((item) => item.supports(profile));
  }

  async apply(profile: GameProfile, onResult?: (result: AdapterResult) => void) {
    return this.run("apply", profile, onResult);
  }

  async capture(profile: GameProfile, onResult?: (result: AdapterResult) => void) {
    return this.run("capture", profile, onResult);
  }

  async restore(profile: GameProfile, onResult?: (result: AdapterResult) => void) {
    const results: AdapterResult[] = [];
    for (const adapter of this.adapters.filter((item) => item.supports(profile))) {
      const result = adapter.restore
        ? await adapter.restore()
        : { adapterId: adapter.id, label: adapter.label, state: "unsupported" as const, message: "Restore is not supported by this adapter." };
      results.push(result);
      onResult?.(result);
      if (result.state === "error") break;
    }
    return results;
  }

  private async run(operation: "apply" | "capture", profile: GameProfile, onResult?: (result: AdapterResult) => void) {
    const results: AdapterResult[] = [];
    const rollbackStack: SettingsAdapter[] = [];
    let workingProfile = profile;
    for (const adapter of this.adapters.filter((item) => item.supports(profile))) {
      try {
        const result = await adapter[operation](workingProfile);
        results.push(result);
        onResult?.(result);
        if (result.settingsPatch) {
          workingProfile = { ...workingProfile, settings: mergeSettingsPatch(workingProfile.settings, result.settingsPatch) };
        }
        if (result.state === "unsupported" || result.state === "warning") logger.warning(`${adapter.id}.${operation}`, result.message);
        if (result.state === "error") logger.error(`${adapter.id}.${operation}`, result.message);
        if (operation === "apply" && result.state !== "error" && result.state !== "unsupported" && adapter.rollback) rollbackStack.push(adapter);
        if (result.state === "error") {
          if (operation === "apply") {
            for (const applied of rollbackStack.reverse()) {
              try {
                const rollback = await applied.rollback?.();
                if (rollback) { results.push(rollback); onResult?.(rollback); }
              } catch (error) {
                const rollback: AdapterResult = { adapterId: `${applied.id}.rollback`, label: `${applied.label} rollback`, state: "error", message: "Automatic rollback failed.", details: [String(error)] };
                results.push(rollback); onResult?.(rollback);
              }
            }
          }
          break;
        }
      } catch (error) {
        const result: AdapterResult = { adapterId: adapter.id, label: adapter.label, state: "error", message: "Adapter failed" };
        results.push(result); onResult?.(result); logger.error(`${adapter.id}.${operation}`, `${operation} failed`, error);
        break;
      }
    }
    return results;
  }
}
