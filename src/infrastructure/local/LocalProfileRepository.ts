import type { GameProfile } from "../../domain/types";
import { ProfileLimitError } from "../../domain/errors";
import type { CreateProfileInput, ProfileRepository, UpdateProfileInput } from "../../ports/ProfileRepository";

const STORAGE_KEY = "game-passport.demo-profiles.v1";

export class LocalProfileRepository implements ProfileRepository {
  private read(): GameProfile[] { try { return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]"); } catch { return []; } }
  private write(items: GameProfile[]) { localStorage.setItem(STORAGE_KEY, JSON.stringify(items)); }
  async list(userId: string) { return this.read().filter((item) => item.userId === userId).sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)); }
  async create(userId: string, input: CreateProfileInput) {
    const items = this.read();
    if (items.filter((item) => item.userId === userId).length >= 5) throw new ProfileLimitError();
    const now = new Date().toISOString();
    const profile: GameProfile = { id: crypto.randomUUID(), userId, name: input.name.trim(), game: input.game, settings: input.settings ?? {}, createdAt: now, updatedAt: now };
    this.write([...items, profile]); return profile;
  }
  async update(userId: string, id: string, input: UpdateProfileInput) {
    const items = this.read(); const index = items.findIndex((item) => item.id === id && item.userId === userId);
    if (index < 0) throw new Error("Profile not found.");
    const updated = { ...items[index], ...input, name: input.name?.trim() ?? items[index].name, updatedAt: new Date().toISOString() };
    items[index] = updated; this.write(items); return updated;
  }
  async remove(userId: string, id: string) { this.write(this.read().filter((item) => !(item.id === id && item.userId === userId))); }
}
