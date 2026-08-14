import { useCallback, useEffect, useMemo, useState } from "react";
import { Activity, Cloud, CloudOff, Gamepad2, ListChecks, LogOut, Plus, Save, Settings2, SlidersHorizontal } from "lucide-react";
import type { GameId, GameProfile, SessionUser } from "../domain/types";
import type { Services } from "../infrastructure/createServices";
import { applyAdapterRegistry, captureAdapterRegistry, restoreAdapterRegistry } from "../adapters/registry";
import { ProfileOrchestrator } from "../services/profileOrchestrator";
import { mergeSettingsPatch } from "../domain/cs2";
import { logger } from "../services/logger";
import { ProfileCard } from "./ProfileCard";
import { ProfileModal } from "./ProfileModal";
import { OperationScreen } from "./OperationScreen";
import { Diagnostics } from "./Diagnostics";
import { InitialSetup } from "./InitialSetup";
import { FieldValidationWizard } from "./FieldValidationWizard";
import type { CloudState } from "../domain/release";
import type { MousePayload } from "../domain/mouse";
import { fieldValidation } from "../services/fieldValidation";
import { operationHistory } from "../services/operationHistory";

interface Props { user: SessionUser; services: Services; onSignOut: () => void }

export function Dashboard({ user, services, onSignOut }: Props) {
  const [profiles, setProfiles] = useState<GameProfile[]>([]);
  const [loading, setLoading] = useState(true);
  const [modal, setModal] = useState<{ profile?: GameProfile } | null>(null);
  const [operation, setOperation] = useState<{ profile: GameProfile; type: "capture" | "apply" | "restore"; returnView: View } | null>(null);
  const [view, setView] = useState<View>(() => localStorage.getItem("game-passport.initial-setup.v1") ? "profiles" : "setup");
  const [cloudState, setCloudState] = useState<CloudState>(services.mode === "demo" ? "demo" : services.mode === "field-test" ? "field-test" : "checking");
  const [error, setError] = useState("");
  const applyOrchestrator = useMemo(() => new ProfileOrchestrator(applyAdapterRegistry), []);
  const captureOrchestrator = useMemo(() => new ProfileOrchestrator(captureAdapterRegistry), []);
  const restoreOrchestrator = useMemo(() => new ProfileOrchestrator(restoreAdapterRegistry), []);
  const load = useCallback(async () => { try { setProfiles(await services.profiles.list(user.id)); } catch (cause) { setError("Не удалось загрузить профили."); logger.error("profiles", "Profile loading failed", cause); } finally { setLoading(false); } }, [services.profiles, user.id]);
  useEffect(() => { void load(); }, [load]);
  const refreshCloud = useCallback(async () => { const state = await services.checkCloud(); setCloudState(state); }, [services]);
  useEffect(() => {
    void refreshCloud();
    const refresh = () => void refreshCloud();
    window.addEventListener("online", refresh); window.addEventListener("offline", refresh);
    return () => { window.removeEventListener("online", refresh); window.removeEventListener("offline", refresh); };
  }, [refreshCloud]);

  async function save(name: string, game: GameId, manualMouse?: MousePayload) {
    const mousePatch = manualMouse ? { adapters: { mouse: manualMouse } } : undefined;
    if (modal?.profile) {
      const settings = mousePatch ? mergeSettingsPatch(modal.profile.settings, mousePatch) : modal.profile.settings;
      await services.profiles.update(user.id, modal.profile.id, { name, game, settings });
    } else {
      await services.profiles.create(user.id, { name, game, settings: mousePatch ?? {} });
    }
    logger.info("profiles", modal?.profile ? "Updated selected profile" : "Created a profile"); setModal(null); await load();
  }
  async function remove(profile: GameProfile) {
    if (!window.confirm(`Удалить профиль «${profile.name}»? Это действие нельзя отменить.`)) return;
    try { await services.profiles.remove(user.id, profile.id); logger.info("profiles", "Deleted selected profile"); await load(); }
    catch (cause) { setError("Не удалось удалить профиль."); logger.error("profiles", "Profile deletion failed", cause); }
  }

  const persistCapture = useCallback(async (patches: Record<string, unknown>[]) => {
    if (!operation) throw new Error("Выбранный профиль больше недоступен.");
    let settings = operation.profile.settings;
    for (const patch of patches) settings = mergeSettingsPatch(settings, patch);
    await services.profiles.update(user.id, operation.profile.id, { settings });
    logger.info("profile.capture", `Saved captured ${operation.profile.game.toUpperCase()}, Display, NVIDIA and Mouse settings to the selected profile`);
  }, [operation, services.profiles, user.id]);

  const completeOperation = useCallback((results: import("../domain/types").AdapterResult[]) => {
    if (!operation) return;
    operationHistory.record(operation.profile.game, operation.type, results);
    if ((operation.profile.game === "cs2" || operation.profile.game === "pubg") && (operation.type === "capture" || operation.type === "apply")) {
      fieldValidation.recordSoftware(operation.profile.game, operation.type, results);
    }
  }, [operation]);

  function beginOperation(profile: GameProfile, type: "capture" | "apply" | "restore", returnView: View = view) {
    if (services.mode === "demo") {
      setError("Демо-режим не выполняет операции с реальным оборудованием. Для проверки используйте полевую сборку.");
      logger.warning("demo.operation", "Real adapter operation skipped in demo mode");
      setView("profiles");
      return;
    }
    setOperation({ profile, type, returnView });
  }

  if (operation) return <OperationScreen profile={operation.profile} operation={operation.type} orchestrator={operation.type === "capture" ? captureOrchestrator : operation.type === "restore" ? restoreOrchestrator : applyOrchestrator} onCapture={operation.type === "capture" ? persistCapture : undefined} onComplete={completeOperation} onClose={() => { setView(operation.returnView); setOperation(null); void load(); void refreshCloud(); }} />;
  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><Gamepad2 size={21} /></span><span>GAME PASSPORT</span></div>
      <nav><button className={view === "profiles" ? "active" : ""} onClick={() => setView("profiles")}><Settings2 size={19} /> Мои профили</button><button className={view === "validation" ? "active" : ""} onClick={() => setView("validation")}><ListChecks size={19} /> Проверка Game Passport</button><button className={view === "diagnostics" ? "active" : ""} onClick={() => setView("diagnostics")}><Activity size={19} /> Диагностика</button><button className={view === "setup" ? "active" : ""} onClick={() => setView("setup")}><SlidersHorizontal size={19} /> Первичная настройка</button></nav>
      <div className="sidebar-bottom"><div className={`cloud-state ${cloudState}`}>{cloudState === "connected" ? <Cloud size={17} /> : <CloudOff size={17} />}<span><strong>{cloudLabel(cloudState)}</strong><small>{cloudDetail(cloudState)}</small></span></div><div className="account"><span>{user.email.slice(0, 1).toUpperCase()}</span><div><strong>{user.email.split("@")[0]}</strong><small>{user.email}</small></div><button className="icon-button" aria-label="Выйти" onClick={onSignOut}><LogOut size={17} /></button></div></div>
    </aside>
    {view === "setup" ? <InitialSetup mode={services.mode} cloudState={cloudState} refreshCloud={refreshCloud} onContinue={() => { localStorage.setItem("game-passport.initial-setup.v1", new Date().toISOString()); setView("profiles"); }} /> : view === "validation" ? <FieldValidationWizard profiles={profiles} mode={services.mode} onRun={(profile, type) => beginOperation(profile, type, "validation")} /> : view === "diagnostics" ? <Diagnostics mode={services.mode} cloudState={cloudState} /> : <main className="content-view">
      <div className="page-heading"><div><p className="eyebrow">БИБЛИОТЕКА НАСТРОЕК</p><h1>Игровые профили</h1><p>Ваши настройки готовы для следующего компьютера.</p></div><div className="heading-actions"><button className="button primary" disabled={profiles.length >= 5} onClick={() => setModal({})}><Plus size={18} /> Новый профиль</button></div></div>
      <div className="limit-strip"><span><strong>{profiles.length}</strong> из 5 профилей</span><div><i style={{ width: `${profiles.length * 20}%` }} /></div><small>{profiles.length >= 5 ? "Достигнут лимит профилей" : `Свободно мест: ${5 - profiles.length}`}</small></div>
      {error && <div className="form-error page-error" role="alert">{error}<button onClick={() => setError("")}>Закрыть</button></div>}
      {loading ? <div className="loading-grid"><span /><span /><span /></div> : profiles.length === 0 ? <section className="empty-state"><div className="empty-icon"><Save size={28} /></div><p className="eyebrow">ПАСПОРТ ПУСТ</p><h2>Создайте первый игровой профиль</h2><p>Создайте профиль CS2 или PUBG, закройте игру, затем сохраните настройки игры, экрана, NVIDIA и мыши.</p><button className="button primary" onClick={() => setModal({})}><Plus size={18} /> Создать профиль</button></section> : <section className="profile-grid">{profiles.map((profile) => <ProfileCard key={profile.id} profile={profile} onCapture={() => beginOperation(profile, "capture")} onApply={() => beginOperation(profile, "apply")} onRestore={() => beginOperation(profile, "restore")} onEdit={() => setModal({ profile })} onDelete={() => void remove(profile)} />)}<button className="add-card" disabled={profiles.length >= 5} onClick={() => setModal({})}><Plus size={25} /><strong>Добавить профиль</strong><span>{profiles.length >= 5 ? "Достигнут максимум" : "Создать ещё один набор"}</span></button></section>}
      <section className="foundation-note"><div><Activity size={20} /><span><strong>Экран + NVIDIA + мышь + CS2 + PUBG · Windows</strong>Настройки применяются безопасно до запуска игры, с локальными резервными копиями.</span></div><button className="text-button" onClick={() => setView("diagnostics")}>Открыть диагностику →</button></section>
    </main>}
    {modal && <ProfileModal profile={modal.profile} onClose={() => setModal(null)} onSave={save} />}
  </div>;
}

type View = "profiles" | "diagnostics" | "validation" | "setup";

function cloudLabel(state: CloudState) {
  if (state === "connected") return "Облако подключено";
  if (state === "offline") return "Автономный режим";
  if (state === "unavailable") return "Облако недоступно";
  if (state === "demo") return "ДЕМО-РЕЖИМ";
  if (state === "field-test") return "ПОЛЕВОЙ ТЕСТ";
  return "Проверяем облако";
}

function cloudDetail(state: CloudState) {
  if (state === "connected") return "Профили синхронизированы";
  if (state === "offline") return "Только сохранённые профили";
  if (state === "unavailable") return "Повторите в настройке";
  if (state === "demo") return "Без проверки оборудования";
  if (state === "field-test") return "Локально · реальные адаптеры";
  return "Подождите…";
}
