import type { SupabaseClient } from "@supabase/supabase-js";
import type { AuthProvider } from "../../ports/AuthProvider";
import type { SessionUser } from "../../domain/types";

const mapUser = (user: { id: string; email?: string } | null): SessionUser | null => user ? { id: user.id, email: user.email ?? "Unknown email" } : null;

export class SupabaseAuthProvider implements AuthProvider {
  constructor(private readonly client: SupabaseClient) {}
  async getCurrentUser() {
    if (navigator.onLine) {
      const { data, error } = await this.client.auth.getUser();
      if (!error) return mapUser(data.user);
    }
    const { data: cached } = await this.client.auth.getSession();
    const session = cached.session;
    if (!session || !session.expires_at || session.expires_at <= Math.floor(Date.now() / 1000)) return null;
    return mapUser(session.user);
  }
  async signIn(email: string, password: string) { const { data, error } = await this.client.auth.signInWithPassword({ email, password }); if (error) throw error; return mapUser(data.user)!; }
  async signUp(email: string, password: string) { const { data, error } = await this.client.auth.signUp({ email, password }); if (error) throw error; return data.session ? mapUser(data.user) : null; }
  async signOut() { const { error } = await this.client.auth.signOut(); if (error) throw error; }
  onAuthChange(callback: (user: SessionUser | null) => void) { const { data } = this.client.auth.onAuthStateChange((_event, session) => callback(mapUser(session?.user ?? null))); return () => data.subscription.unsubscribe(); }
}
