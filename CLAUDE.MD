## Project: MyLife — Obsidian Automation Engine

Tauri v2 + React + Rust desktop app. Invisible tray daemon that reads local Obsidian `.md` files, fires native notifications, and writes new notes back to the vault via a tray popover. **No external DB** — Obsidian vault is the only data store.

## Vision

Global, invisible routine assistant. User plans in Obsidian; MyLife watches, notifies, and enables quick capture — all without opening a window.

## Tech Stack

- **Frontend:** React 19 + TypeScript + Vite 7
- **Backend:** Rust (Tauri v2)
- **Package Manager:** Bun (not npm/yarn)
- **Key libs:** `front-matter` (TS), `serde`/`serde_json` + `notify` (Rust), `tauri-plugin-notification`, `tauri-plugin-store`, `tauri-plugin-dialog`, `tauri-plugin-positioner`

## Architecture Rules

- Rust owns ALL filesystem I/O — reads and writes. Never touch files from TS/React directly
- React is view-only; all data goes through `invoke()` IPC
- Vault path is never hardcoded — persisted via `tauri-plugin-store` (`settings.json`, key `vault_path`), set during onboarding (`initialize_vault` / `set_vault_path`) or changed later from settings
- Filename = task ID (React key + record identifier), slugified from title (`safe_filename` in `src-tauri/src/main.rs`)
- App is tray-only daemon — no persistent main window (`visible: false`, `skipTaskbar: true`, `alwaysOnTop: true` in `tauri.conf.json`)
- `src-tauri/src/main.rs` is the real entry point (`fn main()` registers all commands + tray). `src-tauri/src/lib.rs` is unused Tauri-init boilerplate (`greet` command, no tray) — don't add commands there, it isn't wired into the binary

## Vault Structure

```
MyLife Vault/
├── 00 - Dashboard/HOME.md
├── 01 - Personal/Daily|Fleeting|Health|Plans|Reflections/
├── 02 - Knowledge/Computer|Health|History|Literature|Philosophy/
├── 03 - Projects/
├── 04 - References/Articles|Books|Courses|Tutorials/
├── 05 - Routines/        ← only folder read for "Hoje" tab / notifications
├── 99 - Meta/System/ (guides), Templates/ (TEMPLATE) *.md
└── .obsidian/
```

Scaffolded by `initialize_vault` (Rust) from files embedded at compile time via `include_str!("../vault-template/...")`. Source of truth for that content lives in `src-tauri/vault-template/` (a working copy) — there is also a standalone clone at repo root, `obsidian-vault-template/` (own git remote: `voidashi/obsidian-vault-template`), used as the upstream reference. Keep the two in sync manually; only `src-tauri/vault-template/` is actually compiled in.

## Task Frontmatter Schema

```yaml
---
tipo: rotina          # rotina | tarefa | estudo | projeto | reflexao | ideia
horario: "14:30"      # ALWAYS quoted — unquoted parses as int minutes (routines only)
status: pendente      # pendente | concluido
---
```

Other note types (`tarefa`, `estudo`, `projeto`, `reflexao`, `ideia`) use a different frontmatter shape (`aliases`, `tags`, `date`, `last_updated`, no `tipo`/`status`) — see `create_note` in `src-tauri/src/main.rs` for the exact fields per type.

## Critical Gotcha: YAML Time Values

`horario: 08:30` (unquoted) → parsed as integer `510` (minutes since midnight).
`parseHorario()` in `App.tsx` handles both `string` and `number`. `create_note` always writes it quoted, so this only bites hand-edited files.

## Key Commands

| Command | Description |
|---|---|
| `bun run dev` | Vite dev server only |
| `bun run tauri dev` | Full Tauri dev (Rust + React) |
| `bun run build` | `tsc && vite build` |
| `cargo build` (in `src-tauri/`) | Rust only |

## Tauri IPC Commands (src-tauri/src/main.rs)

| Command | Purpose |
|---|---|
| `get_vault_path` / `set_vault_path` | Persisted vault path via store plugin |
| `pick_directory` | Native folder picker |
| `initialize_vault` | Scaffold new vault or ensure `05 - Routines/` on existing one |
| `read_obsidian_file` | Recursive `.md` scan, returns `Vec<ObsidianFile>` (skips dotfolders) |
| `start_watching` | Spawns thread with `notify` watcher, emits `vault-changed` on any `.md` event |
| `list_subfolders` | Subfolders under a note type's base folder, for the capture form picker |
| `create_note` | Writes a new `.md` from type-specific frontmatter + template |
| `complete_routine` | String-replaces `status: pendente` → `status: concluido` |
| `open_in_obsidian` | Builds `obsidian://open?vault=…&file=…` deep link |
| `quit_app` | `app.exit(0)` |

## Implemented Features

1. Onboarding — new vault scaffold or attach existing, no hardcoded path
2. Vault reader — Rust scans dir, returns `Vec<ObsidianFile>` via IPC
3. YAML parser — React parses frontmatter; handles `string | number` horario
4. Live file watcher — `notify` crate emits `vault-changed` event; React auto-reloads + reschedules
5. Native notifications — permission request + `setTimeout` scheduler per pending routine
6. Capture form — 6 note types with subfolder picker and template-aware bodies
7. Complete routine toggle, Open in Obsidian deep link, Settings pane (change vault / quit)
8. Menu Bar app — `ActivationPolicy::Accessory`, tray icon toggles + positions popover

## Next Steps / Known Gaps

1. **Windows System Tray parity** — positioner branches per-OS in `main.rs`, untested on Windows
2. **Stale vault path handling** — no check that the stored path still exists on disk
3. **Notification survival across sleep/hibernate** — `setTimeout` delays don't survive; consider recompute-on-wake or OS-native scheduled notifications
4. **Recurring routines** — `status` never auto-resets daily; a routine completed today stays `concluido` tomorrow unless hand-edited

## Anti-Patterns

- No direct file I/O from React — always Rust IPC
- No business logic in components — hooks or utils only
- No hardcoded vault path in production — use `tauri-plugin-store`
- No polling — file watcher is push-based; never use `setInterval` for vault reads
- Don't add commands to `src-tauri/src/lib.rs` — it's unused boilerplate, not part of the running binary

## Repo Status

Not yet a git repository (project moved from another machine, never pushed). See `GIT_SETUP.md` at repo root before running `git init` — flags a nested repo (`obsidian-vault-template/.git`) and the ~19 GB `src-tauri/target/` build cache.
