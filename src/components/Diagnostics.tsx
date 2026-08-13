import { useEffect, useState } from "react";
import { AlertTriangle, Bug, CheckCircle2, Copy, FileDown, RefreshCw, Trash2 } from "lucide-react";
import type { DiagnosticEntry } from "../domain/types";
import type { DisplayCommandResponse, DisplayMode } from "../domain/display";
import type { NvidiaCommandResponse } from "../domain/nvidia";
import type { MouseCommandResponse } from "../domain/mouse";
import type { PubgCommandResponse } from "../domain/pubg";
import { adapterRegistry } from "../adapters/registry";
import { logger } from "../services/logger";
import { getDisplayDiagnostics, getMouseDiagnostics, getNvidiaDiagnostics, getPlatformInfo, getPubgDiagnostics, getReleasePreflight, saveDiagnosticReport, type PlatformInfo } from "../services/platform";
import { RELEASE_LABEL, type CloudState, type FileCommandResponse, type ReleasePreflight } from "../domain/release";
import { fieldValidation } from "../services/fieldValidation";
import { operationHistory } from "../services/operationHistory";
import type { AppMode } from "../infrastructure/createServices";

interface Props { mode: AppMode; cloudState: CloudState }

export function Diagnostics({ mode, cloudState }: Props) {
  const [entries, setEntries] = useState<DiagnosticEntry[]>(logger.list());
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [copied, setCopied] = useState(false);
  const [display, setDisplay] = useState<DisplayCommandResponse | null>(null);
  const [nvidia, setNvidia] = useState<NvidiaCommandResponse | null>(null);
  const [mouse, setMouse] = useState<MouseCommandResponse | null>(null);
  const [pubg, setPubg] = useState<PubgCommandResponse | null>(null);
  const [preflight, setPreflight] = useState<ReleasePreflight | null>(null);
  const [saved, setSaved] = useState<FileCommandResponse | null>(null);
  const [validating, setValidating] = useState(false);
  async function validateHardware() {
    setValidating(true);
    if (mode === "demo") {
      const demoResult = { state: "unsupported" as const, message: "DEMO MODE — real adapter verification was skipped.", details: [], retryable: false };
      setDisplay(demoResult); setNvidia(demoResult); setMouse(demoResult); setPubg(demoResult);
      setPreflight(await getReleasePreflight().catch(() => null)); setValidating(false); return;
    }
    const [displayResult, nvidiaResult, mouseResult, pubgResult, releaseResult] = await Promise.allSettled([getDisplayDiagnostics(), getNvidiaDiagnostics(), getMouseDiagnostics(), getPubgDiagnostics(), getReleasePreflight()]);
    setDisplay(displayResult.status === "fulfilled" ? displayResult.value : { state: "error", message: String(displayResult.reason), details: [], retryable: true });
    setNvidia(nvidiaResult.status === "fulfilled" ? nvidiaResult.value : { state: "error", message: String(nvidiaResult.reason), details: [], retryable: true });
    setMouse(mouseResult.status === "fulfilled" ? mouseResult.value : { state: "error", message: String(mouseResult.reason), details: [], retryable: true });
    setPubg(pubgResult.status === "fulfilled" ? pubgResult.value : { state: "error", message: String(pubgResult.reason), details: [], retryable: true });
    if (releaseResult.status === "fulfilled") setPreflight(releaseResult.value); else logger.error("diagnostics.preflight", "Release preflight failed", releaseResult.reason);
    setValidating(false);
  }
  useEffect(() => { getPlatformInfo().then(setPlatform).catch((error) => logger.error("diagnostics", "Could not read platform information", error)); void validateHardware(); const refresh = () => setEntries(logger.list()); window.addEventListener("game-passport:diagnostics", refresh); return () => window.removeEventListener("game-passport:diagnostics", refresh); }, []);
  const report = JSON.stringify(sanitizeReport({
    generatedAt: new Date().toISOString(), appVersion: RELEASE_LABEL, build: preflight?.build ?? "unknown",
    dataMode: mode, cloudState, platform,
    preflight: preflight ? { windowsVersion: preflight.windowsVersion, windowsSupported: preflight.windowsSupported, steamInstalled: preflight.steamInstalled, steamUserAvailable: preflight.steamUserAvailable, cs2Installed: preflight.cs2Installed, pubgConfigAvailable: preflight.pubgConfigAvailable, administratorRequired: preflight.administratorRequired, updateChannel: preflight.updateChannel, logDirectory: preflight.logDirectory } : null,
    display, nvidia, mouse, pubg,
    adapters: adapterRegistry.map(({ id, label, status }) => ({ id, label, status })),
    fieldValidation: fieldValidation.list(), operations: operationHistory.list(),
    logs: entries.map((entry) => entry.scope.startsWith("profile") ? { ...entry, message: "Profile operation recorded.", details: undefined } : entry)
  }), null, 2);
  const pendingAdapters = adapterRegistry.filter((adapter) => adapter.status !== "implemented").length;
  async function copy() { await navigator.clipboard.writeText(report); setCopied(true); setTimeout(() => setCopied(false), 1600); }
  async function save() { const response = await saveDiagnosticReport(report); setSaved(response); if (response.state === "success") logger.info("diagnostics.export", "Sanitized diagnostic report saved"); else logger.warning("diagnostics.export", response.message); }

  return <section className="content-view diagnostics">
    <div className="page-heading"><div><p className="eyebrow">WINDOWS VALIDATION MODE · {RELEASE_LABEL}</p><h1>Diagnostics</h1><p>Live Display, NVIDIA, Mouse, CS2 and PUBG evidence for field testing.</p></div><div className="heading-actions"><button className="button secondary" disabled={validating} onClick={() => void validateHardware()}><RefreshCw size={17} className={validating ? "spin" : ""} /> Refresh hardware</button><button className="button secondary" onClick={copy}><Copy size={17} /> {copied ? "Copied" : "Copy report"}</button><button className="button secondary" onClick={() => void save()}><FileDown size={17} /> Save report</button></div></div>
    {saved && <div className={`export-result ${saved.state}`}><strong>{saved.state === "success" ? "Report saved" : "Report not saved"}</strong><span>{saved.path ?? saved.message}</span></div>}
    <div className="diagnostic-grid">
      <div className="diagnostic-card"><span className={`diagnostic-icon ${mode !== "demo" && platform?.desktopRuntime ? "ok" : "warn"}`}><CheckCircle2 /></span><div><small>RELEASE</small><h3>{RELEASE_LABEL}</h3><p>{mode === "demo" ? "DEMO MODE — no real hardware success." : mode === "field-test" ? `FIELD TEST · ${preflight?.build ? `Build ${preflight.build}` : "local profiles"}` : preflight?.build ? `Build ${preflight.build}` : "Reading build identity…"}</p></div></div>
      <div className="diagnostic-card"><span className="diagnostic-icon warn"><AlertTriangle /></span><div><small>ADAPTERS</small><h3>{pendingAdapters} pending</h3><p>Display, NVIDIA, Mouse, CS2 and PUBG are implemented for Windows.</p></div></div>
      <div className="diagnostic-card"><span className="diagnostic-icon"><Bug /></span><div><small>RUNTIME</small><h3>{platform?.desktopRuntime ? "Tauri desktop" : "Browser preview"}</h3><p>{platform ? `${platform.os} · ${platform.architecture}` : "Reading platform…"}</p></div></div>
    </div>
    <div className="panel release-panel"><header><div><h2>Release readiness</h2><p>Permissions, offline behavior, update policy and local support paths.</p></div><span className={`status-pill ${preflight?.administratorRequired ? "unsupported" : "implemented"}`}>{preflight ? preflight.administratorRequired ? "Admin required" : "Standard user" : "checking"}</span></header><div className="metric-grid"><Metric label="Cloud" value={cloudState} /><Metric label="Windows" value={preflight?.windowsSupported ? "Supported" : "Not verified"} /><Metric label="Steam user" value={preflight?.steamUserAvailable ? "Detected" : "Sign in required"} /><Metric label="CS2" value={preflight?.cs2Installed ? "Installed" : "Not found"} /><Metric label="PUBG config" value={preflight?.pubgConfigAvailable ? "Found" : "Not found"} /><Metric label="Updater" value="Manual signed releases" /><Metric label="Production logs" value={preflight?.logDirectory ?? "Windows build only"} /><Metric label="Privileges" value={preflight?.administratorRequired ? "Elevation required" : "No permanent admin"} /></div></div>
    <div className="validation-grid">
      <div className="panel validation-panel"><header><div><h2>Display</h2><p>Primary gaming display and modes reported by Win32.</p></div><span className={`status-pill ${display?.state === "success" ? "implemented" : "unsupported"}`}>{display?.state ?? "checking"}</span></header>
        {display?.diagnostics ? <><div className="metric-grid"><Metric label="Monitor" value={display.diagnostics.primaryMonitor ?? "Unknown"} /><Metric label="Displays" value={String(display.diagnostics.monitorCount)} /><Metric label="Current" value={formatMode(display.diagnostics.currentMode)} /><Metric label="Modes" value={String(display.diagnostics.supportedModes.length)} /></div><div className="mode-list">{groupModes(display.diagnostics.supportedModes).map((entry) => <div key={entry.resolution}><strong>{entry.resolution}</strong><span>{entry.rates.join(" · ")} Hz</span></div>)}</div></> : <p className="validation-message">{display?.message ?? "Reading Windows display configuration…"}</p>}
      </div>
      <div className="panel validation-panel"><header><div><h2>NVIDIA</h2><p>Public NVAPI/DRS capability check.</p></div><span className={`status-pill ${nvidia?.state === "success" ? "implemented" : "unsupported"}`}>{nvidia?.state ?? "checking"}</span></header>
        {nvidia?.diagnostics ? <div className="metric-grid"><Metric label="GPU" value={nvidia.diagnostics.gpuName ?? (nvidia.diagnostics.gpuDetected ? "Detected" : "Not detected")} /><Metric label="NVAPI" value={nvidia.diagnostics.nvapiInitialized ? "Initialized" : "Unavailable"} /><Metric label="Driver" value={nvidia.diagnostics.driverVersion ? `${nvidia.diagnostics.driverVersion} ${nvidia.diagnostics.driverBranch ?? ""}` : "Unavailable"} /><Metric label="CS2 profile" value={nvidia.diagnostics.cs2ProfileFound ? "Found" : "Not found"} /><Metric label="Portable settings" value={String(nvidia.diagnostics.settingsRead)} /><Metric label="Scaling" value={nvidia.diagnostics.scalingSupported ? nvidia.diagnostics.scalingMode ?? "Supported" : "Unsupported"} /></div> : <p className="validation-message">{nvidia?.message ?? "Initializing NVIDIA NVAPI…"}</p>}
      </div>
      <div className="panel validation-panel mouse-validation"><header><div><h2>Mouse Passport</h2><p>Physical Windows HID devices and per-device capabilities.</p></div><span className={`status-pill ${mouse?.state === "success" ? "implemented" : "unsupported"}`}>{mouse?.state ?? "checking"}</span></header>
        {mouse?.diagnostics ? <><div className="metric-grid"><Metric label="Detected HID mice" value={String(mouse.diagnostics.devices.length)} /><Metric label="Selection" value={mouse.diagnostics.selectionAmbiguous ? "Ambiguous — no writes" : mouse.diagnostics.selectedInstanceId ? "One physical mouse" : "No controllable mouse"} /><Metric label="Current DPI" value={formatOptional(mouse.diagnostics.currentDpi, " DPI")} /><Metric label="Requested DPI" value={formatOptional(mouse.diagnostics.requestedDpi, " DPI")} /><Metric label="Applied DPI" value={formatOptional(mouse.diagnostics.appliedDpi, " DPI")} /><Metric label="Current polling" value={formatOptional(mouse.diagnostics.currentPollingRateHz, " Hz")} /><Metric label="Requested polling" value={formatOptional(mouse.diagnostics.requestedPollingRateHz, " Hz")} /><Metric label="Applied polling" value={formatOptional(mouse.diagnostics.appliedPollingRateHz, " Hz")} /><Metric label="Backup" value={mouse.diagnostics.backupResult ?? "Not created"} /><Metric label="Restore" value={mouse.diagnostics.restoreResult ?? "Not attempted"} /></div>
          <div className="mode-list">{mouse.diagnostics.devices.map((device) => <div key={`${device.instanceId}-${device.hidUsage}`}><strong>{device.selected ? "✓ " : ""}{device.manufacturer} {device.model}</strong><span>VID {device.vendorId} · PID {device.productId} · {device.connection}</span><span>{device.selectedAdapter} · DPI {device.capabilities.canApplyDpi ? "read/write" : "unsupported"} · Polling {device.capabilities.canApplyPollingRate ? device.capabilities.pollingRatesHz.map((rate) => `${rate} Hz`).join(" / ") : "unsupported"}</span>{device.capabilities.reason && <span>{device.capabilities.reason}</span>}</div>)}</div>
          <p className="validation-message">{mouse.diagnostics.verificationResult ?? mouse.message}</p></> : <p className="validation-message">{mouse?.message ?? "Enumerating physical Windows HID devices…"}</p>}
      </div>
      <div className="panel validation-panel"><header><div><h2>PUBG</h2><p>Portable config discovery and last real file operation.</p></div><span className={`status-pill ${pubg?.state === "success" ? "implemented" : "unsupported"}`}>{pubg?.state ?? "checking"}</span></header>
        {pubg?.diagnostics ? <><div className="metric-grid"><Metric label="Detected" value={pubg.diagnostics.pubgDetected ? "Yes" : "No"} /><Metric label="Process" value={pubg.diagnostics.processRunning ? "Running — blocked" : "Closed"} /><Metric label="Config files" value={String(pubg.diagnostics.configFilesFound.length)} /><Metric label="Capture" value={pubg.diagnostics.captureResult ?? "Not attempted"} /><Metric label="Apply" value={pubg.diagnostics.applyResult ?? "Not attempted"} /><Metric label="Backup" value={pubg.diagnostics.backupResult ?? "Not created"} /><Metric label="Restore" value={pubg.diagnostics.restoreResult ?? "Not attempted"} /></div><p className="validation-message">{pubg.diagnostics.configDirectory ?? "PUBG config directory was not found."}</p>{[...pubg.diagnostics.parseErrors, ...pubg.diagnostics.writeErrors].map((error) => <p className="validation-message" key={error}>{error}</p>)}</> : <p className="validation-message">{pubg?.message ?? "Discovering PUBG configuration…"}</p>}
      </div>
    </div>
    <div className="panel field-tests"><header><div><h2>PUBG field tests A–E</h2><p>Run with PUBG closed; use Retry after closing it.</p></div></header><ol><li><strong>A · Gameplay</strong><span>Capture a distinctive sensitivity, change it, Apply, then verify in PUBG.</span></li><li><strong>B · Graphics</strong><span>Capture distinctive quality settings, change them, Apply, then verify.</span></li><li><strong>C · Keybinds</strong><span>Capture changed bindings, overwrite them, Apply, then verify the saved bindings.</span></li><li><strong>D · Cross-PC</strong><span>Capture on PC 1, sign in on PC 2, Apply, and verify the same portable settings.</span></li><li><strong>E · Full Passport</strong><span>Verify PUBG + exact resolution + MAX_AVAILABLE Hz + NVIDIA profile + supported mouse DPI/polling.</span></li></ol></div>
    <div className="panel field-tests"><header><div><h2>Mouse field tests A–E</h2><p>Run on target Windows hardware. Refresh Diagnostics after each USB/device change.</p></div></header><ol><li><strong>A · Detection</strong><span>Connect Logitech, then replace it with Razer/Lamzu; verify model, VID/PID and adapter update.</span></li><li><strong>B · DPI</strong><span>Set hardware to 400 DPI, apply a saved 800 DPI profile, then confirm current/applied/verified all show 800.</span></li><li><strong>C · Cross-brand</strong><span>Capture 800 DPI on Logitech and apply the same profile to supported Razer, then Lamzu only when its adapter reports write capability.</span></li><li><strong>D · Polling rate</strong><span>Apply 1000/2000/4000 Hz; exact rates must verify, lower-capability hardware must show the explicit fallback.</span></li><li><strong>E · Unsupported</strong><span>Connect an unknown mouse and confirm Warning plus manual DPI/Hz instruction—never Success.</span></li></ol></div>
    <div className="adapter-table panel"><header><div><h2>Adapter status</h2><p>Capabilities available in this build.</p></div></header>{adapterRegistry.map((adapter) => <div className="table-row" key={adapter.id}><span><strong>{adapter.label}</strong><small>{adapter.id}</small></span><span className={`status-pill ${adapter.status === "implemented" ? "implemented" : "unsupported"}`}>{adapter.status === "implemented" ? "Implemented · Windows" : "Not implemented"}</span></div>)}</div>
    <div className="logs panel"><header><div><h2>Diagnostic log</h2><p>Stored locally on this computer. No passwords are logged.</p></div><button className="icon-button" aria-label="Clear logs" onClick={() => logger.clear()}><Trash2 size={17} /></button></header>
      {entries.length === 0 ? <div className="empty-log">No events recorded yet.</div> : entries.map((entry) => <div className={`log-row ${entry.level}`} key={entry.id}><time>{new Date(entry.timestamp).toLocaleTimeString()}</time><span><strong>{entry.scope}</strong>{entry.message}</span></div>)}
    </div>
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><small>{label}</small><strong>{value}</strong></div>;
}

function formatMode(mode?: DisplayMode) {
  return mode ? `${mode.width}×${mode.height} · ${mode.refreshHz} Hz` : "Unavailable";
}

function groupModes(modes: DisplayMode[]) {
  const groups = new Map<string, Set<number>>();
  for (const mode of modes.filter((candidate) => !candidate.interlaced)) {
    const key = `${mode.width}×${mode.height}`;
    const rates = groups.get(key) ?? new Set<number>();
    rates.add(mode.refreshHz); groups.set(key, rates);
  }
  return [...groups.entries()].map(([resolution, rates]) => ({ resolution, rates: [...rates].sort((a, b) => a - b) }));
}

function formatOptional(value: number | undefined, suffix: string) {
  return value === undefined ? "Unavailable" : `${value}${suffix}`;
}

function sanitizeReport(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sanitizeReport);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).filter(([key]) => !/password|token|secret|cookie|authorization|email|userid/i.test(key)).map(([key, child]) => [key, sanitizeReport(child)]));
  }
  if (typeof value === "string") {
    if (/authorization:|bearer\s|access_token|refresh_token|password=|cookie:|steamloginsecure/i.test(value)) return "[REDACTED]";
    return value.replace(/eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g, "[REDACTED]").replace(/[A-Z]:\\Users\\[^\\]+/gi, "%USERPROFILE%");
  }
  return value;
}
