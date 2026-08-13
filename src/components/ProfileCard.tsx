import { MoreHorizontal, Pencil, Play, RotateCcw, Save, Trash2 } from "lucide-react";
import { useState } from "react";
import { GAME_META, type GameProfile } from "../domain/types";

interface Props { profile: GameProfile; onApply: () => void; onCapture: () => void; onRestore: () => void; onEdit: () => void; onDelete: () => void }

export function ProfileCard({ profile, onApply, onCapture, onRestore, onEdit, onDelete }: Props) {
  const [menu, setMenu] = useState(false); const meta = GAME_META[profile.game];
  return <article className="profile-card" style={{ "--game-accent": meta.accent } as React.CSSProperties}>
    <div className="card-glow" />
    <header><span className="game-badge">{meta.short}</span><div className="card-menu-wrap"><button className="icon-button" aria-label={`Параметры профиля ${profile.name}`} onClick={() => setMenu(!menu)}><MoreHorizontal size={20} /></button>
      {menu && <div className="card-menu"><button onClick={() => { setMenu(false); onEdit(); }}><Pencil size={15} /> Изменить</button><button onClick={() => { setMenu(false); onRestore(); }}><RotateCcw size={15} /> Восстановить прежние настройки</button><button className="danger" onClick={() => { setMenu(false); onDelete(); }}><Trash2 size={15} /> Удалить</button></div>}
    </div></header>
    <div className="profile-card-copy"><p>{meta.name}</p><h3>{profile.name}</h3><span>Обновлён {new Date(profile.updatedAt).toLocaleDateString("ru-RU", { month: "short", day: "numeric" })}</span></div>
    <div className="card-actions"><button className="button secondary" onClick={onCapture}><Save size={16} /> Сохранить текущие</button><button className="button apply" onClick={onApply}><Play size={17} fill="currentColor" /> Настроить этот ПК</button></div>
  </article>;
}
