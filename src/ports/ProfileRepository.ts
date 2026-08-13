import type { GameId, GameProfile, ProfileSettings } from "../domain/types";

export interface CreateProfileInput { name: string; game: GameId; settings?: ProfileSettings }
export interface UpdateProfileInput { name?: string; game?: GameId; settings?: ProfileSettings }

export interface ProfileRepository {
  list(userId: string): Promise<GameProfile[]>;
  create(userId: string, input: CreateProfileInput): Promise<GameProfile>;
  update(userId: string, id: string, input: UpdateProfileInput): Promise<GameProfile>;
  remove(userId: string, id: string): Promise<void>;
}
