import type { AuthProvider } from "../../ports/AuthProvider";
import type { SessionUser } from "../../domain/types";

const SESSION_KEY = "game-passport.demo-session.v1";

export class LocalAuthProvider implements AuthProvider {
  private listeners = new Set<(user: SessionUser | null) => void>();
  async getCurrentUser() { return this.read(); }
  async signIn(email: string, password: string) {
    if (!email.includes("@") || password.length < 6) throw new Error("Enter a valid email and a password of at least 6 characters.");
    return this.save(email);
  }
  async signUp(email: string, password: string) { return this.signIn(email, password); }
  async signOut() { localStorage.removeItem(SESSION_KEY); this.emit(null); }
  onAuthChange(callback: (user: SessionUser | null) => void) { this.listeners.add(callback); return () => this.listeners.delete(callback); }
  private read(): SessionUser | null { try { return JSON.parse(localStorage.getItem(SESSION_KEY) ?? "null"); } catch { return null; } }
  private save(email: string) {
    const user = { id: `demo-${email.toLowerCase()}`, email: email.toLowerCase() };
    localStorage.setItem(SESSION_KEY, JSON.stringify(user)); this.emit(user); return user;
  }
  private emit(user: SessionUser | null) { this.listeners.forEach((listener) => listener(user)); }
}
