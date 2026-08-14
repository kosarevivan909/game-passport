import { useEffect, useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { GAME_META, type GameId, type GameProfile } from "../domain/types";
import { createManualMousePayload, getMousePayload, type MousePayload } from "../domain/mouse";

const RELEASE_GAMES = ["cs2", "pubg"] as const;

interface Props { profile?: GameProfile | null; onClose: () => void; onSave: (name: string, game: GameId, manualMouse?: MousePayload) => Promise<void> }

export function ProfileModal({ profile, onClose, onSave }: Props) {
  const [name, setName] = useState(profile?.name ?? "");
  const [game, setGame] = useState<GameId>(profile?.game ?? "cs2");
  const existingMouse = profile ? getMousePayload(profile.settings) : null;
  const [dpi, setDpi] = useState(existingMouse ? String(existingMouse.dpi) : "");
  const [polling, setPolling] = useState(existingMouse?.pollingRateHz ? String(existingMouse.pollingRateHz) : "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => { const listener = (event: KeyboardEvent) => event.key === "Escape" && onClose(); window.addEventListener("keydown", listener); return () => window.removeEventListener("keydown", listener); }, [onClose]);

  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError("");
    try {
      const manualMouse = dpi ? createManualMousePayload(Number(dpi), polling ? Number(polling) : undefined) : undefined;
      if (dpi && !manualMouse) throw new Error("Проверьте DPI и частоту опроса мыши.");
      await onSave(name, game, manualMouse ?? undefined);
    }
    catch (cause) { setError(cause instanceof Error ? cause.message : "Не удалось сохранить профиль."); setBusy(false); }
  }

  return <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
    <section className="modal" role="dialog" aria-modal="true" aria-labelledby="profile-title">
      <button className="icon-button modal-close" aria-label="Закрыть" onClick={onClose}><X size={19} /></button>
      <p className="eyebrow">{profile ? "ИЗМЕНИТЬ ПРОФИЛЬ" : "НОВЫЙ НАБОР"}</p>
      <h2 id="profile-title">{profile ? "Изменить профиль" : "Создать игровой профиль"}</h2>
      <p className="muted">Выберите понятное имя, чтобы быстро найти нужные настройки.</p>
      <form onSubmit={submit}>
        <label>Название профиля<input autoFocus maxLength={40} value={name} onChange={(event) => setName(event.target.value)} placeholder="например, CS2 Соревновательный" required /></label>
        <fieldset><legend>Игра</legend><div className="game-picker">
          {RELEASE_GAMES.map((id) => <button className={`game-choice ${game === id ? "selected" : ""}`} type="button" key={id} onClick={() => setGame(id)} style={{ "--game-accent": GAME_META[id].accent } as React.CSSProperties}>
            <span>{GAME_META[id].short}</span><small>{GAME_META[id].name}</small>
          </button>)}
        </div></fieldset>
        <fieldset className="mouse-fallback"><legend>Мышь · необязательно</legend><p className="muted">Если Logitech не считывается автоматически, впишите значения из G HUB. Они сохранятся в профиле; на другом ПК приложение применит их автоматически либо покажет точную ручную инструкцию.</p><div className="mouse-fields"><label>DPI<input inputMode="numeric" min={50} max={100000} type="number" value={dpi} onChange={(event) => setDpi(event.target.value)} placeholder="например, 800" /></label><label>Частота опроса<select value={polling} onChange={(event) => setPolling(event.target.value)}><option value="">Не указана</option>{[125, 250, 500, 1000, 2000, 4000, 8000].map((rate) => <option key={rate} value={rate}>{rate} Гц</option>)}</select></label></div></fieldset>
        <div className="info-box">{game === "cs2" ? "Сохранение считывает реальные настройки CS2, экрана, NVIDIA и мыши. Сначала закройте CS2 и войдите в Steam." : "Сохранение считывает переносимые настройки PUBG, экрана, NVIDIA и мыши. Сначала закройте PUBG."}</div>
        {error && <div className="form-error" role="alert">{error}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={onClose}>Отмена</button><button className="button primary" disabled={busy || !name.trim()}>{busy ? "Сохраняем…" : profile ? "Сохранить изменения" : "Создать профиль"}</button></div>
      </form>
    </section>
  </div>;
}
