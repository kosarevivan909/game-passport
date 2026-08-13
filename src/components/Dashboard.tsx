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
  const load = useCallback(async () => { try { setProfiles(await services.profiles.list(user.id)); } catch (cause) { setError("Could not load profiles."); logger.error("profiles", "Profile loading failed", cause); } finally { setLoading(false); } }, [services.profiles, user.id]);
  useEffect(() => { void load(); }, [load]);
  const refreshCloud = useCallback(async () => { const state = await services.checkCloud(); setCloudState(state); }, [services]);
  useEffect(() => {
    void refreshCloud();
    const refresh = () => void refreshCloud();
    window.addEventListener("online", refresh); window.addEventListener("offline", refresh);
    return () => { window.removeEventListener("online", refresh); window.removeEventListener("offline", refresh); };
  }, [refreshCloud]);

  async function save(name: string, game: GameId) {
    if (modal?.profile) await services.profiles.update(user.id, modal.profile.id, { name, game });
    else await services.profiles.create(user.id, { name, game, settings: {} });
    logger.info("profiles", modal?.profile ? "Updated selected profile" : "Created a profile"); setModal(null); await load();
  }
  async function remove(profile: GameProfile) {
    if (!window.confirm(`Delete “${profile.name}”? This cannot be undone.`)) return;
    try { await services.profiles.remove(user.id, profile.id); logger.info("profiles", "Deleted selected profile"); await load(); }
    catch (cause) { setError("Could not delete profile."); logger.error("profiles", "Profile deletion failed", cause); }
  }

  const persistCapture = useCallback(async (patches: Record<string, unknown>[]) => {
    if (!operation) throw new Error("The selected profile is no longer available.");
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
      setError("DEMO MODE does not run or verify real hardware operations. Use a configured cloud release build for field testing.");
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
      <nav><button className={view === "profiles" ? "active" : ""} onClick={() => setView("profiles")}><Settings2 size={19} /> My profiles</button><button className={view === "validation" ? "active" : ""} onClick={() => setView("validation")}><ListChecks size={19} /> Test Game Passport</button><button className={view === "diagnostics" ? "active" : ""} onClick={() => setView("diagnostics")}><Activity size={19} /> Diagnostics</button><button className={view === "setup" ? "active" : ""} onClick={() => setView("setup")}><SlidersHorizontal size={19} /> Initial setup</button></nav>
      <div className="sidebar-bottom"><div className={`cloud-state ${cloudState}`}>{cloudState === "connected" ? <Cloud size={17} /> : <CloudOff size={17} />}<span><strong>{cloudLabel(cloudState)}</strong><small>{cloudDetail(cloudState)}</small></span></div><div className="account"><span>{user.email.slice(0, 1).toUpperCase()}</span><div><strong>{user.email.split("@")[0]}</strong><small>{user.email}</small></div><button className="icon-button" aria-label="Sign out" onClick={onSignOut}><LogOut size={17} /></button></div></div>
    </aside>
    {view === "setup" ? <InitialSetup mode={services.mode} cloudState={cloudState} refreshCloud={refreshCloud} onContinue={() => { localStorage.setItem("game-passport.initial-setup.v1", new Date().toISOString()); setView("profiles"); }} /> : view === "validation" ? <FieldValidationWizard profiles={profiles} mode={services.mode} onRun={(profile, type) => beginOperation(profile, type, "validation")} /> : view === "diagnostics" ? <Diagnostics mode={services.mode} cloudState={cloudState} /> : <main className="content-view">
      <div className="page-heading"><div><p className="eyebrow">YOUR SETUP LIBRARY</p><h1>Game profiles</h1><p>Your preferences, ready for the next PC.</p></div><div className="heading-actions"><button className="button primary" disabled={profiles.length >= 5} onClick={() => setModal({})}><Plus size={18} /> New profile</button></div></div>
      <div className="limit-strip"><span><strong>{profiles.length}</strong> / 5 profiles</span><div><i style={{ width: `${profiles.length * 20}%` }} /></div><small>{profiles.length >= 5 ? "Profile limit reached" : `${5 - profiles.length} slots available`}</small></div>
      {error && <div className="form-error page-error" role="alert">{error}<button onClick={() => setError("")}>Dismiss</button></div>}
      {loading ? <div className="loading-grid"><span /><span /><span /></div> : profiles.length === 0 ? <section className="empty-state"><div className="empty-icon"><Save size={28} /></div><p className="eyebrow">EMPTY PASSPORT</p><h2>Create your first game profile</h2><p>Create a CS2 or PUBG profile, close the game, then capture game, Display, NVIDIA and Mouse settings.</p><button className="button primary" onClick={() => setModal({})}><Plus size={18} /> Create profile</button></section> : <section className="profile-grid">{profiles.map((profile) => <ProfileCard key={profile.id} profile={profile} onCapture={() => beginOperation(profile, "capture")} onApply={() => beginOperation(profile, "apply")} onRestore={() => beginOperation(profile, "restore")} onEdit={() => setModal({ profile })} onDelete={() => void remove(profile)} />)}<button className="add-card" disabled={profiles.length >= 5} onClick={() => setModal({})}><Plus size={25} /><strong>Add profile</strong><span>{profiles.length >= 5 ? "Maximum reached" : "Create another loadout"}</span></button></section>}
      <section className="foundation-note"><div><Activity size={20} /><span><strong>Display + NVIDIA + Mouse + CS2 + PUBG · Windows</strong>Portable settings run in a safe pre-game pipeline with local backups.</span></div><button className="text-button" onClick={() => setView("diagnostics")}>Open Validation Mode →</button></section>
    </main>}
    {modal && <ProfileModal profile={modal.profile} onClose={() => setModal(null)} onSave={save} />}
  </div>;
}

type View = "profiles" | "diagnostics" | "validation" | "setup";

function cloudLabel(state: CloudState) {
  if (state === "connected") return "Cloud connected";
  if (state === "offline") return "Offline mode";
  if (state === "unavailable") return "Cloud unavailable";
  if (state === "demo") return "DEMO MODE";
  if (state === "field-test") return "FIELD TEST";
  return "Checking cloud";
}

function cloudDetail(state: CloudState) {
  if (state === "connected") return "Profiles are synced";
  if (state === "offline") return "Cached profiles only";
  if (state === "unavailable") return "Retry in Initial setup";
  if (state === "demo") return "No hardware success claims";
  if (state === "field-test") return "Local profiles · real adapters";
  return "Please wait…";
}
