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
  capture: { title: "1 · Сохранение текущих настроек", detail: "Запускает реальное считывание настроек и сохраняет их в профиль.", software: true },
  apply: { title: "2 · Применение сохранённых настроек", detail: "Проверяет игру, создаёт резервные копии, записывает и проверяет настройки.", software: true },
  gameplay: { title: "3 · Проверка управления", detail: "Запустите игру и проверьте чувствительность, клавиши, HUD, звук и игровые предпочтения.", software: false },
  visual: { title: "4 · Проверка изображения", detail: "Проверьте графику, разрешение, соотношение сторон и оконный режим.", software: false },
  display: { title: "5 · Проверка монитора", detail: "Проверьте нужное разрешение и максимальную доступную частоту на этом ПК.", software: false },
  nvidia: { title: "6 · Проверка NVIDIA", detail: "Если поддерживается, проверьте профиль игры в панели управления NVIDIA.", software: false },
  mouse: { title: "7 · Проверка мыши", detail: "Если поддерживается, проверьте DPI и частоту опроса физической мыши.", software: false }
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
    <div className="page-heading"><div><p className="eyebrow">ПРОВЕРКА GAME PASSPORT</p><h1>Полевое тестирование</h1><p>Автоматические результаты и ваши проверки в игре записываются отдельно.</p></div><button className="button secondary" onClick={reset}><RefreshCw size={17} /> Сбросить {game.toUpperCase()}</button></div>
    {mode === "demo" && <div className="demo-notice prominent"><AlertTriangle size={18} /> ДЕМО-РЕЖИМ — автоматические этапы и подтверждение «Пройдено» отключены.</div>}
    <div className="wizard-toolbar"><div className="game-tabs"><button className={game === "cs2" ? "active" : ""} onClick={() => setGame("cs2")}>CS2</button><button className={game === "pubg" ? "active" : ""} onClick={() => setGame("pubg")}>PUBG</button></div><label>Тестовый профиль<select value={selectedId} onChange={(event) => setSelectedId(event.target.value)}><option value="">Нет профиля {game.toUpperCase()}</option>{gameProfiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}</select></label></div>
    {!selected && <div className="form-error page-error"><AlertTriangle size={18} /> Создайте профиль {game.toUpperCase()} перед автоматической проверкой. Ручные этапы можно пропустить.</div>}
    <section className="wizard-steps">{gameEntries.map((entry) => { const copy = LABELS[entry.stage]; return <article className={`wizard-step ${entry.status}`} key={entry.id}><span className="wizard-status">{statusIcon(entry.status)}</span><div className="wizard-copy"><small>{copy.software ? "ПРОВЕРЯЕТСЯ ПРОГРАММОЙ" : "ПОДТВЕРЖДАЕТ ПОЛЬЗОВАТЕЛЬ"}</small><h3>{copy.title}</h3><p>{copy.detail}</p>{entry.note && <details><summary>Последний результат</summary><p>{entry.note}</p></details>}</div><div className="wizard-actions">{copy.software ? <button className="button secondary" disabled={!selected || mode === "demo"} onClick={() => selected && onRun(selected, entry.stage as "capture" | "apply")}><Play size={16} /> Запустить</button> : <><button disabled={mode === "demo"} title="Пройдено" aria-label={`${entry.id} pass`} onClick={() => mark(entry.id, "pass")}><Check /></button><button title="Ошибка" aria-label={`${entry.id} fail`} onClick={() => mark(entry.id, "fail")}><CircleX /></button><button title="Предупреждение" aria-label={`${entry.id} warning`} onClick={() => mark(entry.id, "warning")}><AlertTriangle /></button><button title="Пропущено" aria-label={`${entry.id} skipped`} onClick={() => mark(entry.id, "skipped")}>Пропустить</button></>}</div></article>; })}</section>
    <div className="field-legend"><span><Check /> ПРОЙДЕНО</span><span><CircleX /> ОШИБКА</span><span><AlertTriangle /> ПРЕДУПРЕЖДЕНИЕ</span><span><Circle /> ПРОПУЩЕНО/ОЖИДАЕТ</span></div>
  </main>;
}

function statusIcon(status: FieldStatus) {
  if (status === "pass") return <Check />;
  if (status === "fail") return <CircleX />;
  if (status === "warning") return <AlertTriangle />;
  return <Circle />;
}
