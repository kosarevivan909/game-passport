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
    catch (cause) { setError(cause instanceof Error ? cause.message : "Could not save the profile."); setBusy(false); }
  }

  return <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
    <section className="modal" role="dialog" aria-modal="true" aria-labelledby="profile-title">
      <button className="icon-button modal-close" aria-label="Close" onClick={onClose}><X size={19} /></button>
      <p className="eyebrow">{profile ? "EDIT PROFILE" : "NEW LOADOUT"}</p>
      <h2 id="profile-title">{profile ? "Update your profile" : "Create a game profile"}</h2>
      <p className="muted">Choose a clear name so you can find the right setup in seconds.</p>
      <form onSubmit={submit}>
        <label>Profile name<input autoFocus maxLength={40} value={name} onChange={(event) => setName(event.target.value)} placeholder="e.g. CS2 Competitive" required /></label>
        <fieldset><legend>Game</legend><div className="game-picker">
          {RELEASE_GAMES.map((id) => <button className={`game-choice ${game === id ? "selected" : ""}`} type="button" key={id} onClick={() => setGame(id)} style={{ "--game-accent": GAME_META[id].accent } as React.CSSProperties}>
            <span>{GAME_META[id].short}</span><small>{GAME_META[id].name}</small>
          </button>)}
        </div></fieldset>
        <div className="info-box">{game === "cs2" ? "Save current settings captures real CS2, Display, supported NVIDIA and Mouse settings. Close CS2 and sign in to Steam first." : "Save current settings captures real portable PUBG, Display, supported NVIDIA and Mouse settings. Close PUBG first."}</div>
        {error && <div className="form-error" role="alert">{error}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={onClose}>Cancel</button><button className="button primary" disabled={busy || !name.trim()}>{busy ? "Saving…" : profile ? "Save changes" : "Create profile"}</button></div>
      </form>
    </section>
  </div>;
}
