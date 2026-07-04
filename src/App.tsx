import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import fm from "front-matter";
import { useCallback, useEffect, useState } from "react";
import "./App.css";

// ── Types ─────────────────────────────────────────────────────────────────────

type Tab = "hoje" | "capturar" | "vault";
type NoteType = "rotina" | "tarefa" | "estudo" | "projeto" | "reflexao" | "ideia";

interface Frontmatter {
  tipo?: string;
  horario?: string | number;
  status?: string;
  tags?: string[];
  aliases?: string[];
  date?: string;
}

interface RawObsidianFile {
  name: string;
  path: string;
  content: string;
}

interface ParsedNote {
  name: string;
  path: string;
  attrs: Frontmatter;
  body: string;
}

// ── Note type config ──────────────────────────────────────────────────────────

const NOTE_TYPES: { id: NoteType; label: string; folder: string; color: string }[] = [
  { id: "rotina", label: "Rotina", folder: "05 - Routines", color: "#818cf8" },
  { id: "tarefa", label: "Tarefa", folder: "01 - Personal/Plans", color: "#fbbf24" },
  { id: "estudo", label: "Estudo", folder: "02 - Knowledge", color: "#60a5fa" },
  { id: "projeto", label: "Projeto", folder: "03 - Projects", color: "#34d399" },
  { id: "reflexao", label: "Reflexão", folder: "01 - Personal/Reflections", color: "#a78bfa" },
  { id: "ideia", label: "Ideia", folder: "01 - Personal/Fleeting", color: "#fb923c" },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

function parseHorario(horario: string | number): Date | null {
  let mins: number;
  if (typeof horario === "number") {
    mins = horario;
  } else {
    const [h, m] = horario.split(":").map(Number);
    if (Number.isNaN(h) || Number.isNaN(m)) return null;
    mins = h * 60 + m;
  }
  const d = new Date();
  d.setHours(Math.floor(mins / 60), mins % 60, 0, 0);
  return d;
}

function displayTime(horario: string | number): string {
  const d = parseHorario(horario);
  if (!d) return "--:--";
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

function todayISO(): string {
  return new Date().toISOString().split("T")[0];
}

function todayLabel(): string {
  return new Date().toLocaleDateString("pt-BR", {
    weekday: "short",
    day: "numeric",
    month: "short",
  });
}

async function ensurePermission(): Promise<boolean> {
  if (await isPermissionGranted()) return true;
  return (await requestPermission()) === "granted";
}

function scheduleNotifications(notes: ParsedNote[], permitted: boolean) {
  if (!permitted) return;
  for (const n of notes) {
    const { horario, status, tipo } = n.attrs;
    if (!horario || status !== "pendente") continue;
    const fireAt = parseHorario(horario);
    if (!fireAt) continue;
    const delay = fireAt.getTime() - Date.now();
    if (delay <= 0) continue;
    setTimeout(() => {
      sendNotification({ title: "MyLife", body: `${n.name.replace(".md", "")} — ${tipo ?? "rotina"}` });
    }, delay);
  }
}

// ── App ───────────────────────────────────────────────────────────────────────

export default function App() {
  const [vaultPath, setVaultPath] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  const [tab, setTab] = useState<Tab>("hoje");
  const [notes, setNotes] = useState<ParsedNote[]>([]);
  const [error, setError] = useState("");

  // Capture form
  const [noteType, setNoteType] = useState<NoteType>("rotina");
  const [title, setTitle] = useState("");
  const [horario, setHorario] = useState("");
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [subfolders, setSubfolders] = useState<string[]>([]);
  const [selectedSubfolder, setSelectedSubfolder] = useState("");
  const [showSettings, setShowSettings] = useState(false);

  // Load persisted vault path
  useEffect(() => {
    invoke<string | null>("get_vault_path").then((p) => {
      setVaultPath(p);
      setInitialized(true);
    });
  }, []);

  // Load subfolders for current note type
  useEffect(() => {
    if (!vaultPath) return;
    const folder = NOTE_TYPES.find((t) => t.id === noteType)!.folder;
    invoke<string[]>("list_subfolders", { vaultPath, folder }).then(setSubfolders);
    setSelectedSubfolder("");
  }, [noteType, vaultPath]);

  const loadNotes = useCallback(async () => {
    if (!vaultPath) return;
    try {
      setError("");
      const raw = await invoke<RawObsidianFile[]>("read_obsidian_file", { path: vaultPath });
      const parsed = raw.map((f) => {
        const { attributes, body } = fm<Frontmatter>(f.content);
        return { name: f.name, path: f.path, attrs: attributes, body };
      });
      const permitted = await ensurePermission();
      setNotes(parsed);
      scheduleNotifications(parsed, permitted);
    } catch (e) {
      setError(String(e));
    }
  }, [vaultPath]);

  useEffect(() => {
    if (!vaultPath) return;
    loadNotes();
    invoke("start_watching", { path: vaultPath });
    const unlisten = listen("vault-changed", loadNotes);
    return () => { unlisten.then((fn) => fn()); };
  }, [vaultPath, loadNotes]);

  // Onboarding
  async function handleOnboard(isNew: boolean) {
    try {
      setError("");
      const picked = await invoke<string | null>("pick_directory");
      if (!picked) return;
      await invoke("initialize_vault", { basePath: picked, isNew });
      const finalPath = isNew ? `${picked}/MyLife Vault` : picked;
      await invoke("set_vault_path", { path: finalPath });
      setVaultPath(finalPath);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleChangeVault() {
    try {
      setError("");
      const picked = await invoke<string | null>("pick_directory");
      if (!picked) return;
      await invoke("set_vault_path", { path: picked });
      setVaultPath(picked);
      setShowSettings(false);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleQuit() {
    await invoke("quit_app");
  }

  // Create note
  async function handleCapture(e: React.FormEvent) {
    e.preventDefault();
    if (!title.trim() || !vaultPath) return;
    if (noteType === "rotina" && !horario) return;
    setSaving(true);
    try {
      await invoke("create_note", {
        vaultPath,
        noteType,
        title: title.trim(),
        horario: noteType === "rotina" ? horario : null,
        subfolder: selectedSubfolder || null,
        date: todayISO(),
      });
      setSaved(true);
      setTitle("");
      setHorario("");
      setTimeout(() => setSaved(false), 2000);
      await loadNotes();
      if (noteType === "rotina") setTab("hoje");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  // Complete routine
  async function handleComplete(path: string) {
    if (!vaultPath) return;
    try {
      await invoke("complete_routine", { vaultPath, fileRelPath: path });
      await loadNotes();
    } catch (e) {
      setError(String(e));
    }
  }

  // Open in Obsidian
  async function openObsidian(fileRel?: string) {
    if (!vaultPath) return;
    await invoke("open_in_obsidian", { vaultPath, fileRel: fileRel ?? null });
  }

  // ── Derived data ───────────────────────────────────────────────────────────

  const routines = notes
    .filter((n) => n.path.startsWith("05 - Routines") && n.attrs.horario)
    .sort((a, b) => {
      const ta = parseHorario(a.attrs.horario!)?.getTime() ?? 0;
      const tb = parseHorario(b.attrs.horario!)?.getTime() ?? 0;
      return ta - tb;
    });

  const pendingRoutines = routines.filter((n) => n.attrs.status === "pendente");
  const doneRoutines = routines.filter((n) => n.attrs.status !== "pendente");
  const now = Date.now();

  const noteTypeInfo = NOTE_TYPES.find((t) => t.id === noteType)!;

  // ── Folder stats for Vault tab ─────────────────────────────────────────────

  const folderCounts: Record<string, number> = {};
  for (const n of notes) {
    const key = n.path.split("/").slice(0, n.path.startsWith("0") ? 2 : 1).join("/");
    folderCounts[key] = (folderCounts[key] ?? 0) + 1;
  }

  const VAULT_FOLDERS = [
    { key: "00 - Dashboard", label: "Dashboard", icon: "⌂" },
    { key: "01 - Personal", label: "Pessoal", icon: "◎" },
    { key: "02 - Knowledge", label: "Conhecimento", icon: "◈" },
    { key: "03 - Projects", label: "Projetos", icon: "◇" },
    { key: "04 - References", label: "Referências", icon: "◉" },
    { key: "05 - Routines", label: "Rotinas", icon: "↻" },
    { key: "99 - Meta", label: "Meta", icon: "◆" },
  ];

  // ── Render ─────────────────────────────────────────────────────────────────

  if (!initialized) return null;

  if (!vaultPath) {
    return (
      <div className="popover">
        <header className="header" data-tauri-drag-region>
          <span className="app-name">MyLife</span>
        </header>
        <div className="onboarding">
          <div className="onboard-icon">◎</div>
          <p className="onboard-title">Bem-vindo ao MyLife</p>
          <p className="onboard-body">
            Gerencie suas rotinas com seu vault Obsidian — crie um novo ou conecte um existente.
          </p>
          <button type="button" className="btn-primary" onClick={() => handleOnboard(true)}>
            Criar novo vault
          </button>
          <button type="button" className="btn-ghost" onClick={() => handleOnboard(false)}>
            Usar vault existente
          </button>
          {error && <p className="error-text">{error}</p>}
        </div>
      </div>
    );
  }

  return (
    <div className="popover">
      {/* Header */}
      <header className="header" data-tauri-drag-region>
        <span className="app-name">MyLife</span>
        <div className="header-actions">
          <span className="date-chip">{todayLabel()}</span>
          <button
            type="button"
            className={`settings-btn ${showSettings ? "settings-btn-active" : ""}`}
            onClick={() => setShowSettings((s) => !s)}
            title="Configurações"
          >
            ⚙
          </button>
        </div>
      </header>

      {/* Tab bar */}
      {!showSettings && (
        <nav className="tab-bar">
          {(["hoje", "capturar", "vault"] as Tab[]).map((t) => (
            <button
              key={t}
              type="button"
              className={`tab ${tab === t ? "tab-active" : ""}`}
              onClick={() => setTab(t)}
            >
              {t === "hoje" ? "Hoje" : t === "capturar" ? "Inserir" : "Vault"}
              {t === "hoje" && pendingRoutines.length > 0 && (
                <span className="tab-badge">{pendingRoutines.length}</span>
              )}
            </button>
          ))}
        </nav>
      )}

      {/* Settings pane */}
      {showSettings && (
        <main className="content">
          <div className="tab-pane">
            <div className="group">
              <p className="group-label">VAULT</p>
              <div className="settings-row">
                <span className="settings-label">Caminho</span>
                <span className="settings-value" title={vaultPath ?? ""}>
                  {vaultPath ? vaultPath.split("/").slice(-2).join("/") : "—"}
                </span>
              </div>
              <button type="button" className="btn-ghost" onClick={handleChangeVault}>
                Trocar vault
              </button>
            </div>
            <div className="group">
              <p className="group-label">APP</p>
              <button type="button" className="btn-danger" onClick={handleQuit}>
                Fechar app
              </button>
            </div>
          </div>
        </main>
      )}

      {/* Content */}
      {!showSettings && <main className="content">

        {/* ── Hoje ── */}
        {tab === "hoje" && (
          <div className="tab-pane">
            {pendingRoutines.length === 0 && doneRoutines.length === 0 ? (
              <div className="empty-state">
                <p className="empty-icon">↻</p>
                <p className="empty-text">Nenhuma rotina configurada.</p>
                <button type="button" className="btn-link" onClick={() => setTab("capturar")}>
                  Criar rotina →
                </button>
              </div>
            ) : (
              <>
                {pendingRoutines.length > 0 && (
                  <div className="group">
                    <p className="group-label">PENDENTE</p>
                    {pendingRoutines.map((n) => {
                      const fireAt = parseHorario(n.attrs.horario!);
                      const isPast = fireAt ? fireAt.getTime() < now : false;
                      return (
                        <div key={n.path} className={`routine-row ${isPast ? "past" : ""}`}>
                          <span className="routine-time">{displayTime(n.attrs.horario!)}</span>
                          <span className="routine-name">{n.name.replace(".md", "")}</span>
                          <button
                            type="button"
                            className="done-btn"
                            title="Marcar como concluído"
                            onClick={() => handleComplete(n.path)}
                          >
                            ○
                          </button>
                        </div>
                      );
                    })}
                  </div>
                )}
                {doneRoutines.length > 0 && (
                  <div className="group">
                    <p className="group-label">CONCLUÍDO</p>
                    {doneRoutines.map((n) => (
                      <div key={n.path} className="routine-row done">
                        <span className="routine-time">{displayTime(n.attrs.horario!)}</span>
                        <span className="routine-name">{n.name.replace(".md", "")}</span>
                        <span className="done-check">✓</span>
                      </div>
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {/* ── Capturar ── */}
        {tab === "capturar" && (
          <div className="tab-pane">
            {/* Type grid */}
            <div className="type-grid">
              {NOTE_TYPES.map((t) => (
                <button
                  key={t.id}
                  type="button"
                  className={`type-chip ${noteType === t.id ? "type-chip-active" : ""}`}
                  style={noteType === t.id ? { borderColor: t.color, color: t.color } : {}}
                  onClick={() => setNoteType(t.id)}
                >
                  {t.label}
                </button>
              ))}
            </div>

            {/* Subfolder selector */}
            {subfolders.length > 0 && (
              <div className="type-grid">
                {["", ...subfolders].map((sub) => (
                  <button
                    key={sub || "__root__"}
                    type="button"
                    className={`type-chip ${selectedSubfolder === sub ? "type-chip-active" : ""}`}
                    style={selectedSubfolder === sub ? { borderColor: noteTypeInfo.color, color: noteTypeInfo.color } : {}}
                    onClick={() => setSelectedSubfolder(sub)}
                  >
                    {sub || "Raiz"}
                  </button>
                ))}
              </div>
            )}

            {/* Folder hint */}
            <p className="folder-hint">→ {noteTypeInfo.folder}{selectedSubfolder ? `/${selectedSubfolder}` : ""}</p>

            {/* Form */}
            <form onSubmit={handleCapture} className="capture-form">
              <input
                className="capture-input"
                type="text"
                placeholder={`Nome ${noteType === "rotina" ? "da rotina" : noteType === "tarefa" ? "da tarefa" : noteType === "estudo" ? "do estudo" : noteType === "projeto" ? "do projeto" : noteType === "reflexao" ? "da reflexão" : "da ideia"}...`}
                value={title}
                onChange={(e) => setTitle(e.target.value)}
              />
              {noteType === "rotina" && (
                <input
                  className="capture-time"
                  type="time"
                  value={horario}
                  onChange={(e) => setHorario(e.target.value)}
                />
              )}
              <button
                type="submit"
                className="btn-primary"
                disabled={saving || !title.trim() || (noteType === "rotina" && !horario)}
                style={saving || !title.trim() ? {} : { background: noteTypeInfo.color }}
              >
                {saved ? "✓ Criado!" : saving ? "Criando..." : "Criar nota"}
              </button>
            </form>
          </div>
        )}

        {/* ── Vault ── */}
        {tab === "vault" && (
          <div className="tab-pane">
            <button type='button' className="obsidian-btn" onClick={() => openObsidian()}>
              <span>Abrir no Obsidian</span>
              <span className="obsidian-arrow">↗</span>
            </button>

            <div className="group">
              <p className="group-label">PASTAS</p>
              {VAULT_FOLDERS.map((f) => {
                const count = Object.entries(folderCounts)
                  .filter(([k]) => k.startsWith(f.key))
                  .reduce((acc, [, v]) => acc + v, 0);
                return (
                  <button
                    key={f.key}
                    type='button'
                    className="folder-row"
                    onClick={() => openObsidian(f.key)}
                  >
                    <span className="folder-icon">{f.icon}</span>
                    <span className="folder-name">{f.label}</span>
                    <span className="folder-count">{count}</span>
                    <span className="folder-arrow">›</span>
                  </button>
                );
              })}
            </div>

            <div className="vault-stats">
              <div className="stat">
                <span className="stat-value">{notes.length}</span>
                <span className="stat-label">notas</span>
              </div>
              <div className="stat">
                <span className="stat-value">{pendingRoutines.length}</span>
                <span className="stat-label">pendentes</span>
              </div>
              <div className="stat">
                <span className="stat-value">{doneRoutines.length}</span>
                <span className="stat-label">concluídas</span>
              </div>
            </div>
          </div>
        )}
      </main>}

      {error && <p className="error-text" style={{ padding: "0 16px 12px" }}>{error}</p>}
    </div>
  );
}
