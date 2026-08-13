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
          const syncResult: AdapterResult = { adapterId: "profile.sync", label: "Сохранение профиля", state: "success", message: "Считанные настройки сохранены в этом профиле." };
          finalResults = [...completed, syncResult];
          if (mounted) setResults((old) => [...old, syncResult]);
        } catch (error) {
          const syncResult: AdapterResult = { adapterId: "profile.sync", label: "Сохранение профиля", state: "error", message: error instanceof Error ? error.message : "Не удалось сохранить считанные настройки.", retryable: true };
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
  const title = !done ? (operation === "capture" ? `Считываем ${gameName}, экран, NVIDIA и мышь…` : operation === "restore" ? "Восстанавливаем прежние настройки…" : "Применяем ваш Game Passport…")
    : operation === "capture" ? (hasError ? "НАСТРОЙКИ НЕ СОХРАНЕНЫ" : !capturePersisted ? "СОХРАНЕНИЕ НЕДОСТУПНО" : hasWarning ? "СОХРАНЕНО С ПРЕДУПРЕЖДЕНИЯМИ" : "НАСТРОЙКИ СОХРАНЕНЫ")
      : operation === "restore" ? (hasError ? "ВОССТАНОВЛЕНИЕ НЕ ЗАВЕРШЕНО" : !hasRestoredResult ? "РЕЗЕРВНАЯ КОПИЯ НЕ ВОССТАНОВЛЕНА" : hasWarning ? "ВОССТАНОВЛЕНО С ПРЕДУПРЕЖДЕНИЯМИ" : "НАСТРОЙКИ ВОССТАНОВЛЕНЫ")
        : hasError || !hasAppliedResult ? "НАСТРОЙКИ НЕ ПРИМЕНЕНЫ" : hasWarning ? "ГОТОВО С ПРЕДУПРЕЖДЕНИЯМИ" : "КОМПЬЮТЕР ГОТОВ";
  const lead = !done ? (operation === "capture" ? `Считываем реальные настройки ${gameName}, экрана Windows, NVAPI и физической мыши.` : operation === "restore" ? "Используем последние локальные резервные копии, созданные до изменений Game Passport." : `Проверяем, что ${gameName} закрыта, затем применяем настройки экрана, NVIDIA, мыши и ${gameName}.`)
    : operation === "capture" ? (hasError || !capturePersisted ? "В профиль ничего не записано." : "Реальные файлы, считанные Windows-адаптером, добавлены в этот профиль.")
      : operation === "restore" ? (hasRestoredResult ? "Доступные локальные настройки восстановлены. Проверьте предупреждения." : "Ни один компонент не удалось восстановить.")
        : hasError || !hasAppliedResult ? "Операция не была завершена, поэтому Game Passport не сообщает об успехе." : `Настройки экрана, драйвера и мыши применены до файлов ${gameName}. Перед запуском игры проверьте предупреждения.`;
  const baseRows = adapters.map((adapter) => ({ id: adapter.id, label: adapter.label }));
  const extraRows = results.filter((result) => !baseRows.some((row) => row.id === result.adapterId)).map((result) => ({ id: result.adapterId, label: result.label }));
  const rows = [...baseRows, ...extraRows, ...(operation === "capture" && results.some((result) => result.adapterId === "profile.sync") && !extraRows.some((row) => row.id === "profile.sync") ? [{ id: "profile.sync", label: "Сохранение профиля" }] : [])];

  return <main className="process-shell">
    <div className="process-card">
      <button className="back-button" onClick={onClose}><ArrowLeft size={18} /> Назад к профилям</button>
      <div className={`passport-orb ${done ? hasError || operation === "capture" && !capturePersisted || operation === "apply" && !hasAppliedResult || operation === "restore" && !hasRestoredResult ? "complete" : "success" : ""}`}><div><span>{done && (hasError || operation === "capture" && !capturePersisted || operation === "apply" && !hasAppliedResult || operation === "restore" && !hasRestoredResult) ? "!" : "GP"}</span></div></div>
      <p className="eyebrow">ПРОФИЛЬ: {profile.name.toUpperCase()}</p>
      <h1>{title}</h1>
      <p className="process-lead">{lead}</p>
      <div className="process-list">{rows.map((row) => {
        const result = results.find((item) => item.adapterId === row.id);
        return <div className={`process-row ${result?.state ?? "pending"}`} key={row.id}>
          <span>{row.label}<small>{result?.message ?? "Ожидание…"}</small>{result?.details && result.details.length > 0 && <details className="technical-details"><summary>Технические подробности</summary>{result.details.map((detail) => <em key={detail}>{detail}</em>)}</details>}</span>{stateIcon(result?.state)}
        </div>;
      })}</div>
      {done && hasWarning && <div className="unsupported-summary"><ShieldAlert size={19} /><span><strong>Нужна проверка</strong>Некоторые настройки не удалось считать или полностью проверить.</span></div>}
      {done && retryable && <button className="button secondary wide" onClick={() => setAttempt((value) => value + 1)}><RotateCw size={17} /> Повторить</button>}
      {done && <button className="button primary wide" onClick={onClose}>Вернуться к профилям</button>}
    </div>
  </main>;
}
