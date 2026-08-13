import type { AuthProvider } from "../ports/AuthProvider";
import type { ProfileRepository } from "../ports/ProfileRepository";
import { LocalAuthProvider } from "./local/LocalAuthProvider";
import { LocalProfileRepository } from "./local/LocalProfileRepository";
import { createSupabaseClient } from "./supabase/client";
import { SupabaseAuthProvider } from "./supabase/SupabaseAuthProvider";
import { SupabaseProfileRepository } from "./supabase/SupabaseProfileRepository";
import type { CloudState } from "../domain/release";

export type AppMode = "demo" | "field-test" | "cloud";

export interface Services { auth: AuthProvider; profiles: ProfileRepository; mode: AppMode; checkCloud: () => Promise<CloudState> }

export function createServices(): Services {
  const useFieldTest = import.meta.env.VITE_FIELD_TEST_MODE === "true";
  const useDemo = import.meta.env.VITE_DEMO_MODE === "true" || !import.meta.env.VITE_SUPABASE_URL;
  if (useFieldTest) return { auth: new LocalAuthProvider(), profiles: new LocalProfileRepository(), mode: "field-test", checkCloud: async () => "field-test" };
  if (useDemo) return { auth: new LocalAuthProvider(), profiles: new LocalProfileRepository(), mode: "demo", checkCloud: async () => "demo" };
  const client = createSupabaseClient();
  return {
    auth: new SupabaseAuthProvider(client), profiles: new SupabaseProfileRepository(client), mode: "cloud",
    checkCloud: async () => {
      if (!navigator.onLine) return "offline";
      try {
        const { error } = await client.from("profiles").select("id").limit(1);
        return error ? "unavailable" : "connected";
      } catch { return "unavailable"; }
    }
  };
}
