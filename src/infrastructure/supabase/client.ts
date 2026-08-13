import { createClient } from "@supabase/supabase-js";
import { ConfigurationError } from "../../domain/errors";

export function createSupabaseClient() {
  const url = import.meta.env.VITE_SUPABASE_URL;
  const key = import.meta.env.VITE_SUPABASE_ANON_KEY;
  if (!url || !key || url.includes("YOUR_PROJECT")) throw new ConfigurationError("Supabase is not configured.");
  return createClient(url, key, { auth: { persistSession: true, autoRefreshToken: true, detectSessionInUrl: false } });
}
