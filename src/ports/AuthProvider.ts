import type { SessionUser } from "../domain/types";

export interface AuthProvider {
  getCurrentUser(): Promise<SessionUser | null>;
  signIn(email: string, password: string): Promise<SessionUser>;
  signUp(email: string, password: string): Promise<SessionUser | null>;
  signOut(): Promise<void>;
  onAuthChange(callback: (user: SessionUser | null) => void): () => void;
}
