import { useEffect, useMemo, useState } from "react";
import type { SessionUser } from "./domain/types";
import { createServices } from "./infrastructure/createServices";
import { AuthScreen } from "./components/AuthScreen";
import { Dashboard } from "./components/Dashboard";
import { logger } from "./services/logger";

export function App() {
  const services = useMemo(createServices, []);
  const [user, setUser] = useState<SessionUser | null>(null);
  const [ready, setReady] = useState(false);
  useEffect(() => { services.auth.getCurrentUser().then(setUser).catch((error) => logger.error("auth", "Session restore failed", error)).finally(() => setReady(true)); return services.auth.onAuthChange(setUser); }, [services]);
  async function signOut() { try { await services.auth.signOut(); setUser(null); } catch (error) { logger.error("auth", "Sign out failed", error); } }
  if (!ready) return <main className="splash"><div className="passport-loader">GP</div><span>Загружаем ваш паспорт…</span></main>;
  return user ? <Dashboard user={user} services={services} onSignOut={() => void signOut()} /> : <AuthScreen auth={services.auth} mode={services.mode} onAuthenticated={setUser} />;
}
