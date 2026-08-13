import { useState, type FormEvent } from "react";
import { ArrowRight, Cloud, Gamepad2, ShieldCheck } from "lucide-react";
import type { AuthProvider } from "../ports/AuthProvider";
import type { SessionUser } from "../domain/types";
import type { AppMode } from "../infrastructure/createServices";

interface Props { auth: AuthProvider; mode: AppMode; onAuthenticated: (user: SessionUser) => void }

export function AuthScreen({ auth, mode, onAuthenticated }: Props) {
  const [variant, setVariant] = useState<"signin" | "signup">("signin");
  const [email, setEmail] = useState(mode !== "cloud" ? "player@example.com" : "");
  const [password, setPassword] = useState(mode !== "cloud" ? "passport" : "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [technicalError, setTechnicalError] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError(""); setTechnicalError("");
    if (mode === "cloud" && !navigator.onLine) { setError("You are offline. Connect to the internet to sign in safely."); setBusy(false); return; }
    try {
      const user = variant === "signin" ? await auth.signIn(email, password) : await auth.signUp(email, password);
      if (user) onAuthenticated(user);
      else setError("Check your inbox to confirm the account, then sign in.");
    } catch (cause) { const detail = cause instanceof Error ? cause.message : String(cause); setError(friendlyAuthError(detail)); setTechnicalError(detail); }
    finally { setBusy(false); }
  }

  return <main className="auth-shell">
    <section className="auth-hero">
      <div className="brand"><span className="brand-mark"><Gamepad2 size={22} /></span><span>GAME PASSPORT</span></div>
      <div className="hero-copy">
        <p className="eyebrow">YOUR SETUP. EVERYWHERE.</p>
        <h1>Walk in.<br /><span>Game on.</span></h1>
        <p>Carry your perfect gaming setup between PCs and make every station feel like yours.</p>
      </div>
      <div className="trust-row"><span><Cloud size={16} /> {mode === "cloud" ? "Supabase profiles" : mode === "field-test" ? "Field Test · local profiles" : "Demo · local only"}</span><span><ShieldCheck size={16} /> No game passwords</span></div>
    </section>
    <section className="auth-panel">
      <div className="auth-card">
        <p className="eyebrow">WELCOME {variant === "signin" ? "BACK" : "ABOARD"}</p>
        <h2>{variant === "signin" ? "Sign in to your passport" : "Create your passport"}</h2>
        <p className="muted">Your profiles are ready wherever you play.</p>
        {mode === "demo" && <div className="demo-notice"><span className="status-dot" /> Demo mode — data stays on this device</div>}
        {mode === "field-test" && <div className="demo-notice prominent"><span className="status-dot" /> FIELD TEST — profiles stay on this PC; real Windows adapters are enabled</div>}
        <form onSubmit={submit}>
          <label>Email<input aria-label="Email" type="email" autoComplete="email" value={email} onChange={(event) => setEmail(event.target.value)} required /></label>
          <label>Password<input aria-label="Password" type="password" autoComplete={variant === "signin" ? "current-password" : "new-password"} minLength={6} value={password} onChange={(event) => setPassword(event.target.value)} required /></label>
          {error && <div className="form-error" role="alert">{error}{technicalError && <details className="technical-details"><summary>Technical details</summary><span>{technicalError}</span></details>}</div>}
          <button className="button primary wide" disabled={busy}>{busy ? "Please wait…" : variant === "signin" ? "Enter Game Passport" : "Create account"}<ArrowRight size={18} /></button>
        </form>
        <button className="text-button" onClick={() => { setVariant(variant === "signin" ? "signup" : "signin"); setError(""); }}>
          {variant === "signin" ? "New here? Create an account" : "Already have an account? Sign in"}
        </button>
      </div>
    </section>
  </main>;
}

function friendlyAuthError(detail: string) {
  if (/invalid login credentials/i.test(detail)) return "Email or password is incorrect.";
  if (/email not confirmed/i.test(detail)) return "Confirm your email, then try signing in again.";
  if (/fetch|network|offline/i.test(detail)) return "Game Passport could not reach the profile cloud. Check the internet connection and try again.";
  return "Game Passport could not complete sign-in. Try again or open Technical details for support.";
}
