import { beforeEach, describe, expect, it } from "vitest";
import { LocalProfileRepository } from "../infrastructure/local/LocalProfileRepository";
import { ProfileLimitError } from "../domain/errors";

describe("LocalProfileRepository", () => {
  beforeEach(() => localStorage.clear());

  it("creates, updates, lists and removes a profile", async () => {
    const repository = new LocalProfileRepository();
    const created = await repository.create("u1", { name: "CS2 Main", game: "cs2" });
    expect(await repository.list("u1")).toHaveLength(1);
    expect((await repository.update("u1", created.id, { name: "CS2 Competitive" })).name).toBe("CS2 Competitive");
    await repository.remove("u1", created.id);
    expect(await repository.list("u1")).toEqual([]);
  });

  it("enforces the five-profile limit per user", async () => {
    const repository = new LocalProfileRepository();
    for (let index = 0; index < 5; index++) await repository.create("u1", { name: `Profile ${index}`, game: "cs2" });
    await expect(repository.create("u1", { name: "Sixth", game: "pubg" })).rejects.toBeInstanceOf(ProfileLimitError);
    await expect(repository.create("u2", { name: "Other user", game: "dota2" })).resolves.toBeDefined();
  });
});
