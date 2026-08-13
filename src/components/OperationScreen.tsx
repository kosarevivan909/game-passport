import { AlertTriangle, ArrowLeft, Check, CircleX, LoaderCircle, RotateCw, ShieldAlert } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { GAME_META, type AdapterResult, type GameProfile, type ProfileSettings } from "../domain/types";
import { ProfileOrchestrator } from "../services/profileOrchestrator";

interface Props {
  profile: GameProfile;
  operation: "capture" | "apply" | "restore";
  orchestrator: ProfileOrchestrator;
  onCapture?: (patches: ProfileSettings[]) => Promise<void>;
  onComplete?: (results: AdapterResult[]) => void;
  onClose: () => void;
}

const stateIcon = (state?: AdapterResult["state"]) => {
  if (!state) return <LoaderCircle className="spin" size={19} />;
  if (state === "success") return <Check size={18} />;
  if (state === "error") return <CircleX size={18} />;
  if (state === "warning") return <AlertTriangle size={18} />;
  return <ShieldAlert size={18} />;
};

export function OperationScreen({ profile, operation, orchestrator, onCapture, onComplete, onClose }: Props) {
  const [results, setResults] = useState<AdapterResult[]>([]);
  const [done, setDone] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const adapters = useMemo(() => orchestrator.applicable(profile), [orchestrator, profile]);

  useEffect(() => {
    let mounted = true;
    setResults([]); setDone(false);
    async function run() {
      const completed = await orchestrator[operation](profile, (result) => mounted && setResults((old) => [...old, result]));
      const patches = completed.flatMap((result) => result.settingsPatch ? [result.settingsPatch] : []);
      let finalResults = completed;
      if (operation === "capture" && patches.length > 0 && onCapture) {
        try {
          await onCapture(patches);
          const syncResult: AdapterResult = { adapterId: "profile.sync", label: "Profile sync", state: "success", message: "Captured settings were saved to this profile." };
          finalResults = [...completed, syncResult];
          if (mounted) setResults((old) => [...old, syncResult]);
        } catch (error) {
          const syncResult: AdapterResult = { adapterId: "profile.sync", label: "Profile sync", state: "error", message: error instanceof Error ? error.message : "Captured settings could not be saved.", retryable: true };
          finalResults = [...completed, syncResult];
          if (mounted) setResults((old) => [...old, syncResult]);
        }
      }
      if (mounted) { setDone(true); onComplete?.(finalResults); }
    }
    void run();
    return () => { mounted = false; };
  }, [attempt, operation, orchestrator, profile, onCapture, onComplete]);

  const hasError = results.some((result) => result.state === "error");
  const hasWarning = results.some((result) => result.state === "warning" || result.state === "unsupported");
  const gameAdapterId = `game.${profile.game}`;
  const gameName = GAME_META[profile.game].short;
  const hasAppliedResult = results.some((result) => result.adapterId === gameAdapterId && (result.state === "success" || result.state === "warning"));
  const hasRestoredResult = results.some((result) => !result.adapterId.endsWith(".rollback") && (result.state === "success" || result.state === "warning"));
  const capturePersisted = results.some((result) => result.adapterId === "profile.sync" && result.state === "success");
  const retryable = results.some((result) => result.retryable);
  const title = !done ? (operation === "capture" ? `Reading ${gameName}, Display, NVIDIA and Mouse…` : operation === "restore" ? "Restoring pre-Game Passport settings…" : "Applying your Game Passport…")
    : operation === "capture" ? (hasError ? "SETTINGS NOT SAVED" : !capturePersisted ? "CAPTURE NOT AVAILABLE" : hasWarning ? "SETTINGS SAVED WITH WARNINGS" : "SETTINGS SAVED")
      : operation === "restore" ? (hasError ? "RESTORE INCOMPLETE" : !hasRestoredResult ? "NO BACKUP RESTORED" : hasWarning ? "RESTORED WITH WARNINGS" : "SETTINGS RESTORED")
        : hasError || !hasAppliedResult ? "SETUP NOT APPLIED" : hasWarning ? "SETUP COMPLETED WITH WARNINGS" : "YOUR PC IS READY";
  const lead = !done ? (operation === "capture" ? `Reading real ${gameName} configuration, Windows display, NVAPI and physical mouse settings.` : operation === "restore" ? "Using the latest local backups created before Game Passport changed this PC." : `Checking that ${gameName} is closed, then applying Display, NVIDIA, Mouse and ${gameName} in order.`)
    : operation === "capture" ? (hasError || !capturePersisted ? "No settings were written to your profile." : "The real files captured by the Windows adapter are now part of this profile.")
      : operation === "restore" ? (hasRestoredResult ? "Restorable local state was applied. Review warnings for best-effort components." : "No restorable adapter completed successfully.")
        : hasError || !hasAppliedResult ? "Game Passport did not report success because the requested operation was not completed." : `Display, driver and Mouse stages ran before ${gameName} files. Review warnings before launching the game.`;
  const baseRows = adapters.map((adapter) => ({ id: adapter.id, label: adapter.label }));
  const extraRows = results.filter((result) => !baseRows.some((row) => row.id === result.adapterId)).map((result) => ({ id: result.adapterId, label: result.label }));
  const rows = [...baseRows, ...extraRows, ...(operation === "capture" && results.some((result) => result.adapterId === "profile.sync") && !extraRows.some((row) => row.id === "profile.sync") ? [{ id: "profile.sync", label: "Profile sync" }] : [])];

  return <main className="process-shell">
    <div className="process-card">
      <button className="back-button" onClick={onClose}><ArrowLeft size={18} /> Back to profiles</button>
      <div className={`passport-orb ${done ? hasError || operation === "capture" && !capturePersisted || operation === "apply" && !hasAppliedResult || operation === "restore" && !hasRestoredResult ? "complete" : "success" : ""}`}><div><span>{done && (hasError || operation === "capture" && !capturePersisted || operation === "apply" && !hasAppliedResult || operation === "restore" && !hasRestoredResult) ? "!" : "GP"}</span></div></div>
      <p className="eyebrow">PROFILE: {profile.name.toUpperCase()}</p>
      <h1>{title}</h1>
      <p className="process-lead">{lead}</p>
      <div className="process-list">{rows.map((row) => {
        const result = results.find((item) => item.adapterId === row.id);
        return <div className={`process-row ${result?.state ?? "pending"}`} key={row.id}>
          <span>{row.label}<small>{result?.message ?? "Waiting…"}</small>{result?.details && result.details.length > 0 && <details className="technical-details"><summary>Technical details</summary>{result.details.map((detail) => <em key={detail}>{detail}</em>)}</details>}</span>{stateIcon(result?.state)}
        </div>;
      })}</div>
      {done && hasWarning && <div className="unsupported-summary"><ShieldAlert size={19} /><span><strong>Review required</strong>Warnings describe settings that could not be captured or fully verified.</span></div>}
      {done && retryable && <button className="button secondary wide" onClick={() => setAttempt((value) => value + 1)}><RotateCw size={17} /> Повторить проверку / Retry</button>}
      {done && <button className="button primary wide" onClick={onClose}>Return to dashboard</button>}
    </div>
  </main>;
}
