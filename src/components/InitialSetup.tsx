import { useEffect, useState } from "react";
import { AlertTriangle, Check, ChevronRight, CircleX, Cloud, Gamepad2, Monitor, MousePointer2, RefreshCw, ShieldCheck } from "lucide-react";
import type { CloudState, ReleasePreflight } from "../domain/release";
import type { DisplayCommandResponse } from "../domain/display";
import type { NvidiaCommandResponse } from "../domain/nvidia";
import type { MouseCommandResponse } from "../domain/mouse";
import { getDisplayDiagnostics, getMouseDiagnostics, getNvidiaDiagnostics, getReleasePreflight } from "../services/platform";
import { logger } from "../services/logger";
import type { AppMode } from "../infrastructure/createServices";

interface Props {
  mode: AppMode;
  cloudState: CloudState;
  refreshCloud: () => Promise<void>;
  onContinue: () => void;
}

type CheckState = "pass" | "warning" | "fail" | "checking";
interface CheckItem { id: string; label: string; detail: string; state: CheckState; icon: typeof Cloud }

export function InitialSetup({ mode, cloudState, refreshCloud, onContinue }: Props) {
  const [preflight, setPreflight] = useState<ReleasePreflight | null>(null);
  const [display, setDisplay] = useState<DisplayCommandResponse | null>(null);
  const [nvidia, setNvidia] = useState<NvidiaCommandResponse | null>(null);
  const [mouse, setMouse] = useState<MouseCommandResponse | null>(null);
  const [checking, setChecking] = useState(false);

  async function run() {
    setChecking(true);
    if (mode === "demo") {
      const release = await getReleasePreflight().catch((error) => { logger.error("setup.preflight", "Initial Windows check failed", error); return null; });
      setPreflight(release);
      const demoResult = { state: "unsupported" as const, message: "DEMO MODE — hardware verification was skipped.", details: [], retryable: false };
      setDisplay(demoResult); setNvidia(demoResult); setMouse(demoResult);
      await refreshCloud(); setChecking(false); return;
    }
    const results = await Promise.allSettled([getReleasePreflight(), getDisplayDiagnostics(), getNvidiaDiagnostics(), getMouseDiagnostics(), refreshCloud()]);
    if (results[0].status === "fulfilled") setPreflight(results[0].value); else logger.error("setup.preflight", "Initial Windows check failed", results[0].reason);
    if (results[1].status === "fulfilled") setDisplay(results[1].value); else logger.error("setup.display", "Initial display check failed", results[1].reason);
    if (results[2].status === "fulfilled") setNvidia(results[2].value); else logger.error("setup.nvidia", "Initial NVIDIA check failed", results[2].reason);
    if (results[3].status === "fulfilled") setMouse(results[3].value); else logger.error("setup.mouse", "Initial mouse check failed", results[3].reason);
    setChecking(false);
  }

  useEffect(() => { void run(); }, []);

  const items: CheckItem[] = [
    { id: "windows", label: "Windows", detail: preflight ? preflight.windowsVersion : "Checking supported Windows version…", state: !preflight ? "checking" : preflight.windowsSupported ? "pass" : "warning", icon: ShieldCheck },
    { id: "internet", label: "Internet", detail: navigator.onLine ? "Network connection is available." : "Offline. Cached profiles remain available when present.", state: navigator.onLine ? "pass" : "warning", icon: Cloud },
    { id: "supabase", label: "Profile storage", detail: mode === "field-test" ? "Field Test build — profiles stay on this PC; real Windows adapters are enabled." : mode === "demo" ? "Demo mode — no cloud verification or hardware success is claimed." : cloudState === "connected" ? "Supabase is reachable." : cloudState === "offline" ? "Offline — sign-in and profile changes are unavailable." : cloudState === "checking" ? "Checking Supabase…" : "Supabase is not reachable right now.", state: mode === "field-test" ? "warning" : mode === "demo" || cloudState === "offline" || cloudState === "unavailable" ? "warning" : cloudState === "connected" ? "pass" : "checking", icon: Cloud },
    { id: "steam", label: "Steam", detail: !preflight ? "Checking Steam…" : !preflight.steamInstalled ? "Steam was not found." : !preflight.steamUserAvailable ? "Войдите в Steam, чтобы Game Passport смог определить ваши настройки CS2." : "Steam and its active user were detected.", state: !preflight ? "checking" : preflight.steamInstalled && preflight.steamUserAvailable ? "pass" : "warning", icon: Gamepad2 },
    { id: "cs2", label: "Counter-Strike 2", detail: preflight?.cs2Installed ? "CS2 installation was found." : "CS2 installation was not found in Steam libraries.", state: !preflight ? "checking" : preflight.cs2Installed ? "pass" : "warning", icon: Gamepad2 },
    { id: "pubg", label: "PUBG", detail: preflight?.pubgConfigAvailable ? "PUBG configuration was found." : "PUBG configuration will appear after the game has been launched once.", state: !preflight ? "checking" : preflight.pubgConfigAvailable ? "pass" : "warning", icon: Gamepad2 },
    { id: "display", label: "Monitors", detail: display?.message ?? "Checking monitor configuration…", state: responseState(display?.state), icon: Monitor },
    { id: "nvidia", label: "NVIDIA", detail: nvidia?.message ?? "Checking NVIDIA capability…", state: responseState(nvidia?.state), icon: Monitor },
    { id: "mouse", label: "Mouse", detail: mouse?.message ?? "Checking physical mouse capability…", state: responseState(mouse?.state), icon: MousePointer2 }
  ];

  return <main className="content-view setup-view">
    <div className="page-heading"><div><p className="eyebrow">INITIAL SETUP</p><h1>Let’s check this PC</h1><p>Warnings do not block Game Passport. They show what needs attention before a real capture or apply.</p></div><button className="button secondary" disabled={checking} onClick={() => void run()}><RefreshCw className={checking ? "spin" : ""} size={17} /> Повторить проверку</button></div>
    {mode === "demo" && <div className="demo-notice prominent"><AlertTriangle size={18} /> DEMO MODE — local sample data only. Hardware and cloud checks are never treated as a successful transfer.</div>}
    {mode === "field-test" && <div className="demo-notice prominent"><AlertTriangle size={18} /> FIELD TEST — profiles are local to this PC. Real Windows adapters and validation are enabled.</div>}
    <section className="setup-grid">{items.map((item) => <div className={`setup-item ${item.state}`} key={item.id}><span className="setup-icon"><item.icon size={20} /></span><div><strong>{item.label}</strong><small>{item.detail}</small></div><span className="setup-result">{item.state === "pass" ? <Check /> : item.state === "fail" ? <CircleX /> : item.state === "warning" ? <AlertTriangle /> : <RefreshCw className="spin" />}</span></div>)}</section>
    <div className="setup-footer"><p>You can continue now and repeat every check later in Diagnostics.</p><button className="button primary" onClick={onContinue}>Continue to profiles <ChevronRight size={18} /></button></div>
  </main>;
}

function responseState(state?: string): CheckState {
  if (!state) return "checking";
  if (state === "success") return "pass";
  if (state === "error") return "fail";
  return "warning";
}
