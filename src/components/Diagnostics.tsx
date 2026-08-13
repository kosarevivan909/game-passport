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
      const demoResult = { state: "unsupported" as const, message: "ДЕМО-РЕЖИМ — проверка реальных адаптеров пропущена.", details: [], retryable: false };
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
    <div className="page-heading"><div><p className="eyebrow">ПРОВЕРКА WINDOWS · {RELEASE_LABEL}</p><h1>Диагностика</h1><p>Данные экрана, NVIDIA, мыши, CS2 и PUBG для полевого тестирования.</p></div><div className="heading-actions"><button className="button secondary" disabled={validating} onClick={() => void validateHardware()}><RefreshCw size={17} className={validating ? "spin" : ""} /> Обновить</button><button className="button secondary" onClick={copy}><Copy size={17} /> {copied ? "Скопировано" : "Копировать отчёт"}</button><button className="button secondary" onClick={() => void save()}><FileDown size={17} /> Сохранить отчёт</button></div></div>
    {saved && <div className={`export-result ${saved.state}`}><strong>{saved.state === "success" ? "Отчёт сохранён" : "Отчёт не сохранён"}</strong><span>{saved.path ?? saved.message}</span></div>}
    <div className="diagnostic-grid">
      <div className="diagnostic-card"><span className={`diagnostic-icon ${mode !== "demo" && platform?.desktopRuntime ? "ok" : "warn"}`}><CheckCircle2 /></span><div><small>ВЕРСИЯ</small><h3>{RELEASE_LABEL}</h3><p>{mode === "demo" ? "ДЕМО-РЕЖИМ — без проверки реального оборудования." : mode === "field-test" ? `ПОЛЕВОЙ ТЕСТ · ${preflight?.build ? `Сборка ${preflight.build}` : "локальные профили"}` : preflight?.build ? `Сборка ${preflight.build}` : "Определяем сборку…"}</p></div></div>
      <div className="diagnostic-card"><span className="diagnostic-icon warn"><AlertTriangle /></span><div><small>АДАПТЕРЫ</small><h3>Ожидают: {pendingAdapters}</h3><p>Экран, NVIDIA, мышь, CS2 и PUBG реализованы для Windows.</p></div></div>
      <div className="diagnostic-card"><span className="diagnostic-icon"><Bug /></span><div><small>СРЕДА</small><h3>{platform?.desktopRuntime ? "Приложение Windows" : "Просмотр в браузере"}</h3><p>{platform ? `${platform.os} · ${platform.architecture}` : "Определяем платформу…"}</p></div></div>
    </div>
    <div className="panel release-panel"><header><div><h2>Готовность сборки</h2><p>Права, автономная работа, обновления и локальные пути поддержки.</p></div><span className={`status-pill ${preflight?.administratorRequired ? "unsupported" : "implemented"}`}>{preflight ? preflight.administratorRequired ? "Нужен администратор" : "Обычный пользователь" : "проверка"}</span></header><div className="metric-grid"><Metric label="Облако" value={cloudState} /><Metric label="Windows" value={preflight?.windowsSupported ? "Поддерживается" : "Не проверена"} /><Metric label="Пользователь Steam" value={preflight?.steamUserAvailable ? "Найден" : "Нужно войти"} /><Metric label="CS2" value={preflight?.cs2Installed ? "Установлена" : "Не найдена"} /><Metric label="Настройки PUBG" value={preflight?.pubgConfigAvailable ? "Найдены" : "Не найдены"} /><Metric label="Обновления" value="Вручную" /><Metric label="Журналы" value={preflight?.logDirectory ?? "Только в Windows-сборке"} /><Metric label="Права" value={preflight?.administratorRequired ? "Нужно повышение прав" : "Постоянные права не нужны"} /></div></div>
    <div className="validation-grid">
      <div className="panel validation-panel"><header><div><h2>Экран</h2><p>Основной игровой монитор и режимы, обнаруженные Windows.</p></div><span className={`status-pill ${display?.state === "success" ? "implemented" : "unsupported"}`}>{stateLabel(display?.state)}</span></header>
        {display?.diagnostics ? <><div className="metric-grid"><Metric label="Монитор" value={display.diagnostics.primaryMonitor ?? "Неизвестно"} /><Metric label="Экранов" value={String(display.diagnostics.monitorCount)} /><Metric label="Текущий режим" value={formatMode(display.diagnostics.currentMode)} /><Metric label="Режимов" value={String(display.diagnostics.supportedModes.length)} /></div><div className="mode-list">{groupModes(display.diagnostics.supportedModes).map((entry) => <div key={entry.resolution}><strong>{entry.resolution}</strong><span>{entry.rates.join(" · ")} Гц</span></div>)}</div></> : <p className="validation-message">{display?.message ?? "Считываем настройки экрана Windows…"}</p>}
      </div>
      <div className="panel validation-panel"><header><div><h2>NVIDIA</h2><p>Проверка возможностей NVAPI/DRS.</p></div><span className={`status-pill ${nvidia?.state === "success" ? "implemented" : "unsupported"}`}>{stateLabel(nvidia?.state)}</span></header>
        {nvidia?.diagnostics ? <div className="metric-grid"><Metric label="GPU" value={nvidia.diagnostics.gpuName ?? (nvidia.diagnostics.gpuDetected ? "Detected" : "Not detected")} /><Metric label="NVAPI" value={nvidia.diagnostics.nvapiInitialized ? "Initialized" : "Unavailable"} /><Metric label="Driver" value={nvidia.diagnostics.driverVersion ? `${nvidia.diagnostics.driverVersion} ${nvidia.diagnostics.driverBranch ?? ""}` : "Unavailable"} /><Metric label="CS2 profile" value={nvidia.diagnostics.cs2ProfileFound ? "Found" : "Not found"} /><Metric label="Portable settings" value={String(nvidia.diagnostics.settingsRead)} /><Metric label="Scaling" value={nvidia.diagnostics.scalingSupported ? nvidia.diagnostics.scalingMode ?? "Supported" : "Unsupported"} /></div> : <p className="validation-message">{nvidia?.message ?? "Initializing NVIDIA NVAPI…"}</p>}
      </div>
      <div className="panel validation-panel mouse-validation"><header><div><h2>Паспорт мыши</h2><p>Физические HID-устройства Windows и их возможности.</p></div><span className={`status-pill ${mouse?.state === "success" ? "implemented" : "unsupported"}`}>{stateLabel(mouse?.state)}</span></header>
        {mouse?.diagnostics ? <><div className="metric-grid"><Metric label="Detected HID mice" value={String(mouse.diagnostics.devices.length)} /><Metric label="Selection" value={mouse.diagnostics.selectionAmbiguous ? "Ambiguous — no writes" : mouse.diagnostics.selectedInstanceId ? "One physical mouse" : "No controllable mouse"} /><Metric label="Current DPI" value={formatOptional(mouse.diagnostics.currentDpi, " DPI")} /><Metric label="Requested DPI" value={formatOptional(mouse.diagnostics.requestedDpi, " DPI")} /><Metric label="Applied DPI" value={formatOptional(mouse.diagnostics.appliedDpi, " DPI")} /><Metric label="Current polling" value={formatOptional(mouse.diagnostics.currentPollingRateHz, " Hz")} /><Metric label="Requested polling" value={formatOptional(mouse.diagnostics.requestedPollingRateHz, " Hz")} /><Metric label="Applied polling" value={formatOptional(mouse.diagnostics.appliedPollingRateHz, " Hz")} /><Metric label="Backup" value={mouse.diagnostics.backupResult ?? "Not created"} /><Metric label="Restore" value={mouse.diagnostics.restoreResult ?? "Not attempted"} /></div>
          <div className="mode-list">{mouse.diagnostics.devices.map((device) => <div key={`${device.instanceId}-${device.hidUsage}`}><strong>{device.selected ? "✓ " : ""}{device.manufacturer} {device.model}</strong><span>VID {device.vendorId} · PID {device.productId} · {device.connection}</span><span>{device.selectedAdapter} · DPI {device.capabilities.canApplyDpi ? "read/write" : "unsupported"} · Polling {device.capabilities.canApplyPollingRate ? device.capabilities.pollingRatesHz.map((rate) => `${rate} Hz`).join(" / ") : "unsupported"}</span>{device.capabilities.reason && <span>{device.capabilities.reason}</span>}</div>)}</div>
          <p className="validation-message">{mouse.diagnostics.verificationResult ?? mouse.message}</p></> : <p className="validation-message">{mouse?.message ?? "Enumerating physical Windows HID devices…"}</p>}
      </div>
      <div className="panel validation-panel"><header><div><h2>PUBG</h2><p>Поиск переносимых настроек и последняя операция с файлами.</p></div><span className={`status-pill ${pubg?.state === "success" ? "implemented" : "unsupported"}`}>{stateLabel(pubg?.state)}</span></header>
        {pubg?.diagnostics ? <><div className="metric-grid"><Metric label="Detected" value={pubg.diagnostics.pubgDetected ? "Yes" : "No"} /><Metric label="Process" value={pubg.diagnostics.processRunning ? "Running — blocked" : "Closed"} /><Metric label="Config files" value={String(pubg.diagnostics.configFilesFound.length)} /><Metric label="Capture" value={pubg.diagnostics.captureResult ?? "Not attempted"} /><Metric label="Apply" value={pubg.diagnostics.applyResult ?? "Not attempted"} /><Metric label="Backup" value={pubg.diagnostics.backupResult ?? "Not created"} /><Metric label="Restore" value={pubg.diagnostics.restoreResult ?? "Not attempted"} /></div><p className="validation-message">{pubg.diagnostics.configDirectory ?? "PUBG config directory was not found."}</p>{[...pubg.diagnostics.parseErrors, ...pubg.diagnostics.writeErrors].map((error) => <p className="validation-message" key={error}>{error}</p>)}</> : <p className="validation-message">{pubg?.message ?? "Discovering PUBG configuration…"}</p>}
      </div>
    </div>
    <div className="panel field-tests"><header><div><h2>Полевые тесты PUBG A–E</h2><p>Запускайте при закрытой PUBG; после закрытия используйте «Повторить».</p></div></header><ol><li><strong>A · Управление</strong><span>Сохраните заметную чувствительность, измените её, примените профиль и проверьте в PUBG.</span></li><li><strong>B · Графика</strong><span>Сохраните заметные настройки качества, измените их, примените и проверьте.</span></li><li><strong>C · Клавиши</strong><span>Сохраните изменённые клавиши, перезапишите их, примените профиль и проверьте.</span></li><li><strong>D · Другой ПК</strong><span>Сохраните профиль на ПК 1, откройте его на ПК 2, примените и проверьте переносимые настройки.</span></li><li><strong>E · Полный паспорт</strong><span>Проверьте PUBG, разрешение, максимальную частоту, профиль NVIDIA, DPI и частоту мыши.</span></li></ol></div>
    <div className="panel field-tests"><header><div><h2>Полевые тесты мыши A–E</h2><p>Проводите на целевом Windows-ПК. Обновляйте диагностику после каждой смены USB-устройства.</p></div></header><ol><li><strong>A · Обнаружение</strong><span>Подключите Logitech, затем Razer/Lamzu; проверьте модель, VID/PID и обновление адаптера.</span></li><li><strong>B · DPI</strong><span>Установите 400 DPI, примените профиль 800 DPI и убедитесь, что текущее, применённое и проверенное значения равны 800.</span></li><li><strong>C · Между брендами</strong><span>Сохраните 800 DPI на Logitech и примените к поддерживаемой Razer; Lamzu — только если адаптер разрешает запись.</span></li><li><strong>D · Частота опроса</strong><span>Примените 1000/2000/4000 Гц; точные значения должны проверяться, а ограничения устройства — показываться явно.</span></li><li><strong>E · Неподдерживаемая мышь</strong><span>Подключите неизвестную мышь: должно появиться предупреждение и ручная инструкция, но не сообщение об успехе.</span></li></ol></div>
    <div className="adapter-table panel"><header><div><h2>Состояние адаптеров</h2><p>Возможности, доступные в этой сборке.</p></div></header>{adapterRegistry.map((adapter) => <div className="table-row" key={adapter.id}><span><strong>{adapter.label}</strong><small>{adapter.id}</small></span><span className={`status-pill ${adapter.status === "implemented" ? "implemented" : "unsupported"}`}>{adapter.status === "implemented" ? "Реализовано · Windows" : "Не реализовано"}</span></div>)}</div>
    <div className="logs panel"><header><div><h2>Журнал диагностики</h2><p>Хранится локально на этом компьютере. Пароли не записываются.</p></div><button className="icon-button" aria-label="Очистить журнал" onClick={() => logger.clear()}><Trash2 size={17} /></button></header>
      {entries.length === 0 ? <div className="empty-log">Событий пока нет.</div> : entries.map((entry) => <div className={`log-row ${entry.level}`} key={entry.id}><time>{new Date(entry.timestamp).toLocaleTimeString("ru-RU")}</time><span><strong>{entry.scope}</strong>{entry.message}</span></div>)}
    </div>
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><small>{label}</small><strong>{value}</strong></div>;
}

function stateLabel(state?: string) {
  if (!state) return "проверка";
  if (state === "success") return "готово";
  if (state === "warning") return "предупреждение";
  if (state === "error") return "ошибка";
  return "не поддерживается";
}

function formatMode(mode?: DisplayMode) {
  return mode ? `${mode.width}×${mode.height} · ${mode.refreshHz} Гц` : "Недоступно";
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
  return value === undefined ? "Недоступно" : `${value}${suffix}`;
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
