# MyLife — Obsidian Automation Engine

Invisible background assistant that lives in the macOS Menu Bar / Windows System Tray. Uses a local Obsidian vault as its database. Reads routines, fires native notifications, and lets you create notes without ever opening Obsidian.

## The Idea

MyLife is a global, invisible routine manager. The user plans their life in Obsidian (local `.md` files). MyLife watches those files, proactively pushes native notifications at the right time, and lets you capture new notes from a tray popover — turning passive time-blocking into a live life assistant.

Key differentiator: **no open window needed**. The app lives in the tray, reacts to vault changes in real time, and exposes a compact popover for quick note creation that writes directly back to the vault.

## Architecture

```
Obsidian Vault (.md files)
        │
        ▼
Rust (notify crate) ── File Watcher ──► IPC Event ──► React re-renders
        │
        ▼
Rust (read_dir) ── IPC ──► React parses YAML frontmatter ──► schedules notifications
        │
        ▼
Rust (write .md) ◄── IPC ◄── React tray popover (capture form)
```

**Why local-first:** Zero latency, no API limits, full privacy. Sync is user's choice (iCloud, Git, Obsidian Sync).
**Why Rust for I/O:** Push-based file watching (OS notifies on save) vs. React polling — saves CPU and battery.
**Why tray-only:** No window pollution. App is a daemon; window is `visible: false` / `alwaysOnTop` / `skipTaskbar`, toggled from the tray icon.

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 19 + TypeScript + Vite 7 |
| Backend (native) | Rust (Tauri v2) |
| Package Manager | Bun (not npm/yarn) |
| `front-matter` | TS YAML parser for `.md` frontmatter |
| `serde` / `serde_json` | Rust struct ↔ JSON for IPC |
| `notify` | Rust FS event watcher (push-based) |
| `tauri-plugin-notification` | Native OS notification bridge |
| `tauri-plugin-store` | Persists chosen vault path (`settings.json`) |
| `tauri-plugin-dialog` | Native folder picker for onboarding |
| `tauri-plugin-positioner` | Docks the popover under the tray icon |

## Onboarding

No vault path is hardcoded. On first launch (`get_vault_path` returns `None`) the app shows a welcome screen with two choices:

- **Criar novo vault** — picks a base directory, then `initialize_vault` scaffolds a full `MyLife Vault/` folder tree, copies `.obsidian/*` config and note templates (embedded at compile time via `include_str!` from `src-tauri/vault-template/`), and writes index notes per folder.
- **Usar vault existente** — picks any directory and only ensures a `05 - Routines/` folder exists inside it.

The resolved path is persisted via `tauri-plugin-store` (`set_vault_path` / `get_vault_path`) so onboarding only runs once. It can be changed later from the settings pane (⚙).

## Vault Structure

```
MyLife Vault/
├── 00 - Dashboard/
│   └── HOME.md
├── 01 - Personal/
│   ├── Daily/ Fleeting/ Health/ Plans/ Reflections/
│   └── Personal.md          ← index note
├── 02 - Knowledge/
│   ├── Computer/ Health/ History/ Literature/ Philosophy/
│   └── Knowledge.md
├── 03 - Projects/
│   └── Projects.md
├── 04 - References/
│   ├── Articles/ Books/ Courses/ Tutorials/
│   └── References.md
├── 05 - Routines/           ← notification-eligible notes live here
│   └── Routines.md
├── 99 - Meta/
│   ├── System/               (usage guides)
│   ├── Templates/             (TEMPLATE) Daily/Idea/Project/Reflection/Review/Studies/Task.md
│   └── Meta.md
└── .obsidian/                (plugin/workspace config)
```

Only `05 - Routines/*.md` files are read for the "Hoje" tab and notification scheduling; the reader (`read_obsidian_file`) still indexes every `.md` in the vault for stats/search.

## Note Types (Capture Tab)

Each type maps to a target folder, optional subfolder, tag set, and template (loaded from `99 - Meta/Templates/`, `{{title}}`/`{{date}}` substituted; falls back to a hardcoded body if the template file is missing):

| Type | Folder | Needs `horario` | Template |
|---|---|---|---|
| `rotina` | `05 - Routines` | ✅ | — (inline body) |
| `tarefa` | `01 - Personal/Plans` | — | Task |
| `estudo` | `02 - Knowledge` | — | Studies |
| `projeto` | `03 - Projects` | — | Project |
| `reflexao` | `01 - Personal/Reflections` | — | Reflection |
| `ideia` | `01 - Personal/Fleeting` | — | Idea |

Filenames are slugified from the title (`safe_filename`); collisions get a `-1`, `-2`, … suffix.

Task notes (`rotina`) use YAML frontmatter:
```yaml
---
tipo: rotina
horario: "14:30"   # ALWAYS quote — unquoted HH:MM parses as int (minutes since midnight)
status: pendente
---
```

## Implemented Features

- **Onboarding flow** — create new vault (full scaffold) or attach an existing one; no hardcoded path
- **Vault reader** — Rust recursively scans dir (skips dotfolders), returns `Vec<ObsidianFile>` via IPC
- **YAML parser** — React (`front-matter`) parses frontmatter; handles `horario` as `string | number`
- **Live File Watcher** — Rust `notify` crate emits `vault-changed` Tauri event on any `.md` change; React auto-reloads and reschedules
- **Native notifications** — `requestPermission()` on first run; `setTimeout` scheduler per pending routine
- **Capture form** — 6 note types, per-type subfolder picker (`list_subfolders`), template-aware body
- **Complete routine** — toggles `status: pendente` → `status: concluido` in place (`complete_routine`)
- **Open in Obsidian** — deep-links via `obsidian://open?vault=…&file=…`, works for a specific note or a folder (opens first `.md` inside it) or the vault root
- **Settings pane** — shows current vault path, "Trocar vault", "Fechar app" (`quit_app`)
- **Menu Bar app** — `ActivationPolicy::Accessory` hides dock icon (macOS); tray icon click toggles/positions the popover (`tauri-plugin-positioner`)

## Roadmap

- [ ] **Windows System Tray** — parity check; positioner logic currently branches per-OS but untested on Windows
- [ ] **Vault path validation** — surface a clear error if the stored path no longer exists (moved/deleted vault)
- [ ] **Notification rescheduling on wake** — `setTimeout` delays don't survive sleep/hibernate; recompute on wake or use OS-scheduled notifications
- [ ] **Recurring routines** — currently one-shot per note; no daily reset of `status`

## Quick Start

```bash
bun install
bun run tauri dev
```

## Known Gotcha

**YAML time values:** `horario: 08:30` (unquoted) → parsed as integer `510` (minutes since midnight).
Always write `horario: "08:30"` in Obsidian. Parser (`parseHorario` in `App.tsx`) handles both, but quoted is safer and is what `create_note` always writes.

## Repo / Git Status

This project was not yet initialized as a git repository (moved here from another machine). See [GIT_SETUP.md](./GIT_SETUP.md) before running `git init` — there's a nested repo under `obsidian-vault-template/` and a ~19 GB `src-tauri/target/` build cache that need to be handled correctly.
