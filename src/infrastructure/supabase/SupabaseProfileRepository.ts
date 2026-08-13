import type { SupabaseClient } from "@supabase/supabase-js";
import type { GameProfile } from "../../domain/types";
import type { CreateProfileInput, ProfileRepository, UpdateProfileInput } from "../../ports/ProfileRepository";

type Row = { id: string; user_id: string; name: string; game: GameProfile["game"]; settings: Record<string, unknown>; created_at: string; updated_at: string };
const map = (row: Row): GameProfile => ({ id: row.id, userId: row.user_id, name: row.name, game: row.game, settings: row.settings ?? {}, createdAt: row.created_at, updatedAt: row.updated_at });
const cacheKey = (userId: string) => `game-passport.profile-cache.v1.${userId}`;

function readCache(userId: string): GameProfile[] {
  try { return JSON.parse(localStorage.getItem(cacheKey(userId)) ?? "[]") as GameProfile[]; }
  catch { return []; }
}

function writeCache(userId: string, profiles: GameProfile[]) {
  localStorage.setItem(cacheKey(userId), JSON.stringify(profiles));
}

export class SupabaseProfileRepository implements ProfileRepository {
  constructor(private readonly client: SupabaseClient) {}
  async list(userId: string) {
    try {
      const { data, error } = await this.client.from("profiles").select("*").eq("user_id", userId).order("updated_at", { ascending: false });
      if (error) throw error;
      const profiles = (data as Row[]).map(map);
      writeCache(userId, profiles);
      return profiles;
    } catch (error) {
      const cached = readCache(userId);
      if (cached.length > 0) return cached;
      throw error;
    }
  }
  async create(userId: string, input: CreateProfileInput) {
    const { data, error } = await this.client.from("profiles").insert({ user_id: userId, name: input.name.trim(), game: input.game, settings: input.settings ?? {} }).select().single();
    if (error) throw error;
    const profile = map(data as Row);
    writeCache(userId, [profile, ...readCache(userId).filter((item) => item.id !== profile.id)]);
    return profile;
  }
  async update(userId: string, id: string, input: UpdateProfileInput) {
    const payload = { ...(input.name !== undefined && { name: input.name.trim() }), ...(input.game !== undefined && { game: input.game }), ...(input.settings !== undefined && { settings: input.settings }) };
    const { data, error } = await this.client.from("profiles").update(payload).eq("id", id).eq("user_id", userId).select().single();
    if (error) throw error;
    const profile = map(data as Row);
    writeCache(userId, readCache(userId).map((item) => item.id === profile.id ? profile : item));
    return profile;
  }
  async remove(userId: string, id: string) {
    const { error } = await this.client.from("profiles").delete().eq("id", id).eq("user_id", userId);
    if (error) throw error;
    writeCache(userId, readCache(userId).filter((item) => item.id !== id));
  }
}
