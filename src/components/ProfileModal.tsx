import { useEffect, useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { GAME_META, type GameId, type GameProfile } from "../domain/types";

const RELEASE_GAMES = ["cs2", "pubg"] as const;

interface Props { profile?: GameProfile | null; onClose: () => void; onSave: (name: string, game: GameId) => Promise<void> }

export function ProfileModal({ profile, onClose, onSave }: Props) {
  const [name, setName] = useState(profile?.name ?? "");
  const [game, setGame] = useState<GameId>(profile?.game ?? "cs2");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => { const listener = (event: KeyboardEvent) => event.key === "Escape" && onClose(); window.addEventListener("keydown", listener); return () => window.removeEventListener("keydown", listener); }, [onClose]);

  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError("");
    try { await onSave(name, game); }
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
        <div className="info-box">{game === "cs2" ? "Сохранение считывает реальные настройки CS2, экрана, NVIDIA и мыши. Сначала закройте CS2 и войдите в Steam." : "Сохранение считывает переносимые настройки PUBG, экрана, NVIDIA и мыши. Сначала закройте PUBG."}</div>
        {error && <div className="form-error" role="alert">{error}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={onClose}>Отмена</button><button className="button primary" disabled={busy || !name.trim()}>{busy ? "Сохраняем…" : profile ? "Сохранить изменения" : "Создать профиль"}</button></div>
      </form>
    </section>
  </div>;
}
