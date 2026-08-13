import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Check, Circle, CircleX, Play, RefreshCw } from "lucide-react";
import type { FieldStatus, FieldValidationEntry } from "../domain/release";
import type { GameProfile } from "../domain/types";
import { fieldValidation } from "../services/fieldValidation";
import type { AppMode } from "../infrastructure/createServices";

interface Props {
  profiles: GameProfile[];
  mode: AppMode;
  onRun: (profile: GameProfile, operation: "capture" | "apply") => void;
}

const LABELS: Record<FieldValidationEntry["stage"], { title: string; detail: string; software: boolean }> = {
  capture: { title: "1 · Save current settings", detail: "Runs the real capture pipeline and profile sync.", software: true },
  apply: { title: "2 · Apply saved settings", detail: "Runs real preflight, backups, writes and verification.", software: true },
  gameplay: { title: "3 · Gameplay check", detail: "Launch the game and confirm sensitivity, binds, HUD/audio or PUBG gameplay preferences.", software: false },
  visual: { title: "4 · Visual check", detail: "Confirm graphics, resolution, aspect ratio and window mode in the game UI.", software: false },
  display: { title: "5 · Display check", detail: "Confirm the expected resolution and best available refresh rate on this PC.", software: false },
  nvidia: { title: "6 · NVIDIA check", detail: "If supported, confirm the game profile in NVIDIA Control Panel.", software: false },
  mouse: { title: "7 · Mouse check", detail: "If supported, confirm DPI and polling rate on the physical device.", software: false }
};

export function FieldValidationWizard({ profiles, mode, onRun }: Props) {
  const [game, setGame] = useState<"cs2" | "pubg">("cs2");
  const [entries, setEntries] = useState(fieldValidation.list());
  const gameProfiles = useMemo(() => profiles.filter((profile) => profile.game === game), [profiles, game]);
  const [selectedId, setSelectedId] = useState("");
  useEffect(() => { if (!gameProfiles.some((profile) => profile.id === selectedId)) setSelectedId(gameProfiles[0]?.id ?? ""); }, [gameProfiles, selectedId]);
  useEffect(() => { const refresh = () => setEntries(fieldValidation.list()); window.addEventListener("game-passport:field-validation", refresh); return () => window.removeEventListener("game-passport:field-validation", refresh); }, []);
  const selected = gameProfiles.find((profile) => profile.id === selectedId);
  const gameEntries = entries.filter((entry) => entry.game === game);

  function mark(id: string, status: FieldStatus) { if (mode === "demo" && status === "pass") return; fieldValidation.set(id, status); setEntries(fieldValidation.list()); }
  function reset() { fieldValidation.reset(game); setEntries(fieldValidation.list()); }

  return <main className="content-view validation-wizard">
    <div className="page-heading"><div><p className="eyebrow">TEST GAME PASSPORT</p><h1>Field validation</h1><p>Software evidence and your real in-game/hardware confirmation are recorded separately.</p></div><button className="button secondary" onClick={reset}><RefreshCw size={17} /> Reset {game.toUpperCase()}</button></div>
    {mode === "demo" && <div className="demo-notice prominent"><AlertTriangle size={18} /> DEMO MODE — software stages and PASS confirmation are disabled.</div>}
    <div className="wizard-toolbar"><div className="game-tabs"><button className={game === "cs2" ? "active" : ""} onClick={() => setGame("cs2")}>CS2</button><button className={game === "pubg" ? "active" : ""} onClick={() => setGame("pubg")}>PUBG</button></div><label>Test profile<select value={selectedId} onChange={(event) => setSelectedId(event.target.value)}><option value="">No {game.toUpperCase()} profile</option>{gameProfiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select></label></div>
    {!selected && <div className="form-error page-error"><AlertTriangle size={18} /> Create a {game.toUpperCase()} profile before running software stages. Manual stages may be marked Skip.</div>}
    <section className="wizard-steps">{gameEntries.map((entry) => { const copy = LABELS[entry.stage]; return <article className={`wizard-step ${entry.status}`} key={entry.id}><span className="wizard-status">{statusIcon(entry.status)}</span><div className="wizard-copy"><small>{copy.software ? "SOFTWARE-VERIFIED" : "USER CONFIRMATION"}</small><h3>{copy.title}</h3><p>{copy.detail}</p>{entry.note && <details><summary>Last software result</summary><p>{entry.note}</p></details>}</div><div className="wizard-actions">{copy.software ? <button className="button secondary" disabled={!selected || mode === "demo"} onClick={() => selected && onRun(selected, entry.stage as "capture" | "apply")}><Play size={16} /> Run</button> : <><button disabled={mode === "demo"} title="Pass" aria-label={`${entry.id} pass`} onClick={() => mark(entry.id, "pass")}><Check /></button><button title="Fail" aria-label={`${entry.id} fail`} onClick={() => mark(entry.id, "fail")}><CircleX /></button><button title="Warning" aria-label={`${entry.id} warning`} onClick={() => mark(entry.id, "warning")}><AlertTriangle /></button><button title="Skipped" aria-label={`${entry.id} skipped`} onClick={() => mark(entry.id, "skipped")}>Skip</button></>}</div></article>; })}</section>
    <div className="field-legend"><span><Check /> PASS</span><span><CircleX /> FAIL</span><span><AlertTriangle /> WARNING</span><span><Circle /> SKIPPED/PENDING</span></div>
  </main>;
}

function statusIcon(status: FieldStatus) {
  if (status === "pass") return <Check />;
  if (status === "fail") return <CircleX />;
  if (status === "warning") return <AlertTriangle />;
  return <Circle />;
}
