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
      const demoResult = { state: "unsupported" as const, message: "ДЕМО-РЕЖИМ — проверка оборудования пропущена.", details: [], retryable: false };
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
    { id: "windows", label: "Windows", detail: preflight ? preflight.windowsVersion : "Проверяем версию Windows…", state: !preflight ? "checking" : preflight.windowsSupported ? "pass" : "warning", icon: ShieldCheck },
    { id: "internet", label: "Интернет", detail: navigator.onLine ? "Подключение к сети доступно." : "Нет сети. Ранее сохранённые профили останутся доступны.", state: navigator.onLine ? "pass" : "warning", icon: Cloud },
    { id: "supabase", label: "Хранилище профилей", detail: mode === "field-test" ? "Полевая сборка — профили хранятся на этом ПК; реальные Windows-адаптеры включены." : mode === "demo" ? "Демо-режим — облако и оборудование не проверяются." : cloudState === "connected" ? "Облако доступно." : cloudState === "offline" ? "Нет сети — вход и изменение профилей недоступны." : cloudState === "checking" ? "Проверяем облако…" : "Облако сейчас недоступно.", state: mode === "field-test" ? "warning" : mode === "demo" || cloudState === "offline" || cloudState === "unavailable" ? "warning" : cloudState === "connected" ? "pass" : "checking", icon: Cloud },
    { id: "steam", label: "Steam", detail: !preflight ? "Проверяем Steam…" : !preflight.steamInstalled ? "Steam не найден." : !preflight.steamUserAvailable ? "Войдите в Steam, чтобы Game Passport смог определить ваши настройки CS2." : "Steam и активный пользователь найдены.", state: !preflight ? "checking" : preflight.steamInstalled && preflight.steamUserAvailable ? "pass" : "warning", icon: Gamepad2 },
    { id: "cs2", label: "Counter-Strike 2", detail: preflight?.cs2Installed ? "Установка CS2 найдена." : "CS2 не найдена в библиотеках Steam.", state: !preflight ? "checking" : preflight.cs2Installed ? "pass" : "warning", icon: Gamepad2 },
    { id: "pubg", label: "PUBG", detail: preflight?.pubgConfigAvailable ? "Настройки PUBG найдены." : "Настройки PUBG появятся после первого запуска игры.", state: !preflight ? "checking" : preflight.pubgConfigAvailable ? "pass" : "warning", icon: Gamepad2 },
    { id: "display", label: "Мониторы", detail: display?.message ?? "Проверяем мониторы…", state: responseState(display?.state), icon: Monitor },
    { id: "nvidia", label: "NVIDIA", detail: nvidia?.message ?? "Проверяем возможности NVIDIA…", state: responseState(nvidia?.state), icon: Monitor },
    { id: "mouse", label: "Мышь", detail: mouse?.message ?? "Проверяем физическую мышь…", state: responseState(mouse?.state), icon: MousePointer2 }
  ];

  return <main className="content-view setup-view">
    <div className="page-heading"><div><p className="eyebrow">ПЕРВИЧНАЯ НАСТРОЙКА</p><h1>Проверим этот компьютер</h1><p>Предупреждения не блокируют Game Passport. Они показывают, что нужно проверить перед сохранением или применением профиля.</p></div><button className="button secondary" disabled={checking} onClick={() => void run()}><RefreshCw className={checking ? "spin" : ""} size={17} /> Повторить проверку</button></div>
    {mode === "demo" && <div className="demo-notice prominent"><AlertTriangle size={18} /> ДЕМО-РЕЖИМ — только локальные примеры. Оборудование и облако не проверяются.</div>}
    {mode === "field-test" && <div className="demo-notice prominent"><AlertTriangle size={18} /> ПОЛЕВОЙ ТЕСТ — профили хранятся на этом ПК. Реальные Windows-адаптеры и проверка включены.</div>}
    <section className="setup-grid">{items.map((item) => <div className={`setup-item ${item.state}`} key={item.id}><span className="setup-icon"><item.icon size={20} /></span><div><strong>{item.label}</strong><small>{item.detail}</small></div><span className="setup-result">{item.state === "pass" ? <Check /> : item.state === "fail" ? <CircleX /> : item.state === "warning" ? <AlertTriangle /> : <RefreshCw className="spin" />}</span></div>)}</section>
    <div className="setup-footer"><p>Можно продолжить сейчас и позднее повторить все проверки в разделе «Диагностика».</p><button className="button primary" onClick={onContinue}>Перейти к профилям <ChevronRight size={18} /></button></div>
  </main>;
}

function responseState(state?: string): CheckState {
  if (!state) return "checking";
  if (state === "success") return "pass";
  if (state === "error") return "fail";
  return "warning";
}
