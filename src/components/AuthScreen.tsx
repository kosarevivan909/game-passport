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
    if (mode === "cloud" && !navigator.onLine) { setError("Нет подключения к интернету. Подключитесь к сети, чтобы войти."); setBusy(false); return; }
    try {
      const user = variant === "signin" ? await auth.signIn(email, password) : await auth.signUp(email, password);
      if (user) onAuthenticated(user);
      else setError("Подтвердите аккаунт по ссылке в письме, затем войдите.");
    } catch (cause) { const detail = cause instanceof Error ? cause.message : String(cause); setError(friendlyAuthError(detail)); setTechnicalError(detail); }
    finally { setBusy(false); }
  }

  return <main className="auth-shell">
    <section className="auth-hero">
      <div className="brand"><span className="brand-mark"><Gamepad2 size={22} /></span><span>GAME PASSPORT</span></div>
      <div className="hero-copy">
        <p className="eyebrow">ВАШИ НАСТРОЙКИ. НА ЛЮБОМ ПК.</p>
        <h1>Садитесь.<br /><span>Играйте.</span></h1>
        <p>Переносите игровые настройки между компьютерами — любое место станет вашим.</p>
      </div>
      <div className="trust-row"><span><Cloud size={16} /> {mode === "cloud" ? "Облачные профили" : mode === "field-test" ? "Полевой тест · локальные профили" : "Демо · только на этом ПК"}</span><span><ShieldCheck size={16} /> Без игровых паролей</span></div>
    </section>
    <section className="auth-panel">
      <div className="auth-card">
        <p className="eyebrow">{variant === "signin" ? "С ВОЗВРАЩЕНИЕМ" : "ДОБРО ПОЖАЛОВАТЬ"}</p>
        <h2>{variant === "signin" ? "Войти в Game Passport" : "Создать Game Passport"}</h2>
        <p className="muted">Ваши профили готовы к игре.</p>
        {mode === "demo" && <div className="demo-notice"><span className="status-dot" /> Демо-режим — данные остаются на этом устройстве</div>}
        {mode === "field-test" && <div className="demo-notice prominent"><span className="status-dot" /> ПОЛЕВОЙ ТЕСТ — профили хранятся на этом ПК; реальные Windows-адаптеры включены</div>}
        <form onSubmit={submit}>
          <label>Email<input aria-label="Email" type="email" autoComplete="email" value={email} onChange={(event) => setEmail(event.target.value)} required /></label>
          <label>Пароль<input aria-label="Пароль" type="password" autoComplete={variant === "signin" ? "current-password" : "new-password"} minLength={6} value={password} onChange={(event) => setPassword(event.target.value)} required /></label>
          {error && <div className="form-error" role="alert">{error}{technicalError && <details className="technical-details"><summary>Технические подробности</summary><span>{technicalError}</span></details>}</div>}
          <button className="button primary wide" disabled={busy}>{busy ? "Подождите…" : variant === "signin" ? "Войти" : "Создать аккаунт"}<ArrowRight size={18} /></button>
        </form>
        <button className="text-button" onClick={() => { setVariant(variant === "signin" ? "signup" : "signin"); setError(""); }}>
          {variant === "signin" ? "Впервые здесь? Создать аккаунт" : "Уже есть аккаунт? Войти"}
        </button>
      </div>
    </section>
  </main>;
}

function friendlyAuthError(detail: string) {
  if (/invalid login credentials/i.test(detail)) return "Неверный email или пароль.";
  if (/email not confirmed/i.test(detail)) return "Подтвердите email и попробуйте войти снова.";
  if (/fetch|network|offline/i.test(detail)) return "Не удалось подключиться к облаку профилей. Проверьте интернет и повторите попытку.";
  return "Не удалось войти. Повторите попытку или откройте технические подробности.";
}
