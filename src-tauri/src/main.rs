// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::path::Path;
use tauri::{Emitter, Manager};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_plugin_store::StoreExt;

// ── Vault store ──────────────────────────────────────────────────────────────

const STORE_FILE: &str = "settings.json";
const VAULT_KEY: &str = "vault_path";

#[tauri::command]
fn get_vault_path(app: tauri::AppHandle) -> Option<String> {
    let store = app.store(STORE_FILE).ok()?;
    store.get(VAULT_KEY).and_then(|v| v.as_str().map(|s| s.to_owned()))
}

#[tauri::command]
fn set_vault_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(VAULT_KEY, serde_json::Value::String(path));
    store.save().map_err(|e| e.to_string())
}

// ── Directory picker ─────────────────────────────────────────────────────────

#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

// ── Vault initializer ────────────────────────────────────────────────────────

#[tauri::command]
fn initialize_vault(base_path: String, is_new: bool) -> Result<(), String> {
    use std::fs;

    let vault_root = if is_new {
        Path::new(&base_path).join("MyLife Vault")
    } else {
        Path::new(&base_path).to_path_buf()
    };

    if !is_new {
        fs::create_dir_all(vault_root.join("05 - Routines")).map_err(|e| e.to_string())?;
        return Ok(());
    }

    static FILES: &[(&str, &str)] = &[
        (".obsidian/app.json",               include_str!("../vault-template/.obsidian/app.json")),
        (".obsidian/appearance.json",        include_str!("../vault-template/.obsidian/appearance.json")),
        (".obsidian/backlink.json",          include_str!("../vault-template/.obsidian/backlink.json")),
        (".obsidian/community-plugins.json", include_str!("../vault-template/.obsidian/community-plugins.json")),
        (".obsidian/core-plugins.json",      include_str!("../vault-template/.obsidian/core-plugins.json")),
        (".obsidian/daily-notes.json",       include_str!("../vault-template/.obsidian/daily-notes.json")),
        (".obsidian/graph.json",             include_str!("../vault-template/.obsidian/graph.json")),
        (".obsidian/templates.json",         include_str!("../vault-template/.obsidian/templates.json")),
        (".obsidian/types.json",             include_str!("../vault-template/.obsidian/types.json")),
        (".obsidian/workspace.json",         include_str!("../vault-template/.obsidian/workspace.json")),
        ("00 - Dashboard/HOME.md",           include_str!("../vault-template/00 - Dashboard/HOME.md")),
        ("02 - Knowledge/Computer/Algorithm_complexity.md",
                                             include_str!("../vault-template/02 - Knowledge/Computer/Algorithm_complexity.md")),
        ("99 - Meta/System/General guide.md",
                                             include_str!("../vault-template/99 - Meta/System/General guide.md")),
        ("99 - Meta/System/Guide to Tags and Links - Note Organization in Obsidian.md",
                                             include_str!("../vault-template/99 - Meta/System/Guide to Tags and Links - Note Organization in Obsidian.md")),
        ("99 - Meta/System/Organizational Methods in Obsidian.md",
                                             include_str!("../vault-template/99 - Meta/System/Organizational Methods in Obsidian.md")),
        ("99 - Meta/System/Practical Usage Guide.md",
                                             include_str!("../vault-template/99 - Meta/System/Practical Usage Guide.md")),
        ("99 - Meta/Templates/(TEMPLATE) Daily.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Daily.md")),
        ("99 - Meta/Templates/(TEMPLATE) Idea.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Idea.md")),
        ("99 - Meta/Templates/(TEMPLATE) Project.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Project.md")),
        ("99 - Meta/Templates/(TEMPLATE) Reflection.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Reflection.md")),
        ("99 - Meta/Templates/(TEMPLATE) Review.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Review.md")),
        ("99 - Meta/Templates/(TEMPLATE) Studies.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Studies.md")),
        ("99 - Meta/Templates/(TEMPLATE) Task.md",
                                             include_str!("../vault-template/99 - Meta/Templates/(TEMPLATE) Task.md")),
    ];

    static DIRS: &[&str] = &[
        "01 - Personal/Daily",
        "01 - Personal/Fleeting",
        "01 - Personal/Health",
        "01 - Personal/Plans",
        "01 - Personal/Reflections",
        "02 - Knowledge/Health",
        "02 - Knowledge/History",
        "02 - Knowledge/Literature",
        "02 - Knowledge/Philosophy",
        "03 - Projects",
        "04 - References/Articles",
        "04 - References/Books",
        "04 - References/Courses",
        "04 - References/Tutorials",
        "05 - Routines",
    ];

    for (rel, content) in FILES {
        let dest = vault_root.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&dest, content).map_err(|e| e.to_string())?;
    }

    for dir in DIRS {
        fs::create_dir_all(vault_root.join(dir)).map_err(|e| e.to_string())?;
    }

    // Index notes — one per top-level folder so Obsidian can navigate to them
    let index_notes: &[(&str, &str)] = &[
        ("01 - Personal/Personal.md",     "---\ntipo: index\n---\n\n# Personal\n"),
        ("02 - Knowledge/Knowledge.md",   "---\ntipo: index\n---\n\n# Knowledge\n"),
        ("03 - Projects/Projects.md",     "---\ntipo: index\n---\n\n# Projects\n"),
        ("04 - References/References.md", "---\ntipo: index\n---\n\n# References\n"),
        ("05 - Routines/Routines.md",     "---\ntipo: index\n---\n\n# Routines\n"),
        ("99 - Meta/Meta.md",             "---\ntipo: index\n---\n\n# Meta\n"),
    ];
    for (rel, content) in index_notes {
        let dest = vault_root.join(rel);
        if !dest.exists() {
            fs::write(&dest, content).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// ── File reader (recursive, with relative path) ───────────────────────────────

#[derive(Serialize)]
struct ObsidianFile {
    name: String,
    path: String, // relative to vault root
    content: String,
}

fn collect_md_files(dir: &Path, vault_root: &Path, result: &mut Vec<ObsidianFile>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_path = entry.path();
        if file_path.is_dir() {
            // Skip hidden dirs like .obsidian, .git
            let dir_name = file_path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name.starts_with('.') {
                continue;
            }
            collect_md_files(&file_path, vault_root, result)?;
        } else if file_path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let name = file_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let rel_path = file_path
                    .strip_prefix(vault_root)
                    .unwrap_or(&file_path)
                    .to_string_lossy()
                    .into_owned();
                result.push(ObsidianFile { name, path: rel_path, content });
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn read_obsidian_file(path: &str) -> Result<Vec<ObsidianFile>, String> {
    let vault_root = Path::new(path);
    let mut files_data = Vec::new();
    collect_md_files(vault_root, vault_root, &mut files_data).map_err(|e| e.to_string())?;
    Ok(files_data)
}

// ── File watcher ──────────────────────────────────────────────────────────────

#[tauri::command]
fn start_watching(app: tauri::AppHandle, path: String) {
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = notify::recommended_watcher(tx).expect("failed to create watcher");
        watcher
            .watch(Path::new(&path), RecursiveMode::Recursive)
            .expect("failed to watch path");

        for res in rx {
            if let Ok(event) = res {
                let has_md = event
                    .paths
                    .iter()
                    .any(|p| p.extension().and_then(|s| s.to_str()) == Some("md"));
                if has_md {
                    app.emit("vault-changed", ()).ok();
                }
            }
        }
    });
}

// --- Create SubFolders --------------------------------------

#[tauri::command]
fn list_subfolders(vault_path: String, folder: String) -> Vec<String> {
    let dir = Path::new(&vault_path).join(&folder);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

// ── Note creator (vault-template aware) ──────────────────────────────────────

fn safe_filename(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '-' })
        .collect::<String>()
        .replace(' ', "-")
        .to_lowercase()
}

#[tauri::command]
fn create_note(
    vault_path: String,
    note_type: String,
    title: String,
    horario: Option<String>,
    subfolder: Option<String>,
    date: String,
) -> Result<(), String> {
    let (base_folder, frontmatter, fallback_body, template_name) = match note_type.as_str() {
        "rotina" => {
            let h = horario.unwrap_or_else(|| "09:00".to_owned());
            let fm = format!(
                "---\ntipo: rotina\nhorario: \"{h}\"\nstatus: pendente\ntags:\n  - type/task\n  - context/personal\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!("# {title}\n");
            ("05 - Routines", fm, body, None)
        }
        "tarefa" => {
            let fm = format!(
                "---\naliases:\n  - Task - {title}\ntags:\n  - type/task\n  - context/default\n  - theme/default\n  - priority/high\n  - status/in-progress\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!(
                "# {title}\n\n## Due date\n- {date}\n\n## Description\n\n## Steps\n\n- [ ] \n\n## Notes\n\n## References\n"
            );
            ("01 - Personal/Plans", fm, body, Some("Task"))
        }
        "estudo" => {
            let fm = format!(
                "---\naliases:\n  - Study - {title}\ntags:\n  - type/study\n  - context/studies\n  - theme/default\n  - status/in-progress\n  - review/pending\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!(
                "# Study: {title}\n\n## Objective\n\n## Content\n\n## Review Questions\n\n- \n\n## Summary\n\n## References\n"
            );
            ("02 - Knowledge", fm, body, Some("Studies"))
        }
        "projeto" => {
            let fm = format!(
                "---\naliases:\n  - Project - {title}\ntags:\n  - type/project\n  - context/default\n  - theme/default\n  - priority/high\n  - status/planning\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!(
                "# Project: {title}\n\n## Overview\n- **Main Objective**: \n- **Current Status**: Planning\n\n## Timeline\n- **Start Date**: {date}\n- **Due Date**: \n\n## Tasks\n### Phase 1: Planning\n- [ ] \n\n## Progress Log\n### {date}\n- Next steps:\n\n## References\n"
            );
            ("03 - Projects", fm, body, Some("Project"))
        }
        "reflexao" => {
            let fm = format!(
                "---\naliases:\n  - Reflection - {title}\ntags:\n  - type/reflection\n  - context/personal\n  - theme/default\n  - review/pending\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!(
                "# Reflection: {title}\n\n## Context\n\n## Thoughts\n\n## Insights & Realizations\n\n## Questions & Future Considerations\n\n## References\n"
            );
            ("01 - Personal/Reflections", fm, body, Some("Reflection"))
        }
        "ideia" | _ => {
            let fm = format!(
                "---\naliases:\n  - Idea - {title}\ntags:\n  - type/idea\n  - context/default\n  - theme/default\n  - status/idea\ndate: {date}\nlast_updated: {date}\n---"
            );
            let body = format!(
                "# Idea: {title}\n\n## Description\n\n## Context & Origin\n\n## Potential Applications\n\n- \n\n## Next Steps\n\n- [ ] \n\n## References\n"
            );
            ("01 - Personal/Fleeting", fm, body, Some("Idea"))
        }
    };

    // Load body from vault template if available; fall back to hardcoded
    let body = template_name
        .and_then(|name| {
            let tpl_path = Path::new(&vault_path)
                .join("99 - Meta/Templates")
                .join(format!("(TEMPLATE) {name}.md"));
            std::fs::read_to_string(&tpl_path).ok()
        })
        .map(|raw| {
            // Strip YAML frontmatter from template file — we use our own
            let body_start = if raw.starts_with("---") {
                raw[3..].find("---").map(|i| i + 6).unwrap_or(0)
            } else {
                0
            };
            raw[body_start..]
                .trim_start()
                .replace("{{title}}", &title)
                .replace("{{date}}", &date)
        })
        .unwrap_or(fallback_body);

    let folder = match subfolder.as_deref() {
        Some(sub) if !sub.is_empty() => format!("{base_folder}/{sub}"),
        _ => base_folder.to_owned(),
    };

    let dir = Path::new(&vault_path).join(&folder);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let content = format!("{frontmatter}\n\n{body}");
    let base = safe_filename(&title);
    let mut file_path = dir.join(format!("{base}.md"));
    let mut counter = 1;
    while file_path.exists() {
        file_path = dir.join(format!("{base}-{counter}.md"));
        counter += 1;
    }

    std::fs::write(file_path, content).map_err(|e| e.to_string())
}

// ── Complete a routine (toggle pendente → concluido) ─────────────────────────

#[tauri::command]
fn complete_routine(vault_path: String, file_rel_path: String) -> Result<(), String> {
    let path = Path::new(&vault_path).join(&file_rel_path);
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let updated = content.replace("status: pendente", "status: concluido");
    std::fs::write(&path, updated).map_err(|e| e.to_string())
}

// ── Quit ─────────────────────────────────────────────────────────────────────

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ── Open file or vault in Obsidian ────────────────────────────────────────────

#[tauri::command]
fn open_in_obsidian(
    app: tauri::AppHandle,
    vault_path: String,
    file_rel: Option<String>,
) -> Result<(), String> {
    let vault_name = Path::new(&vault_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let uri = match file_rel {
        Some(rel) => {
            let full = Path::new(&vault_path).join(&rel);
            if full.is_dir() {
                // Open first .md file in folder — Obsidian reveals the folder in sidebar
                let index = std::fs::read_dir(&full)
                    .ok()
                    .and_then(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .find(|e| {
                                e.path().extension().and_then(|s| s.to_str()) == Some("md")
                            })
                            .and_then(|e| {
                                e.path()
                                    .strip_prefix(&vault_path)
                                    .ok()
                                    .map(|r| r.to_string_lossy().into_owned())
                            })
                    });
                match index {
                    Some(f) => {
                        let f_no_ext = f.trim_end_matches(".md").to_owned();
                        format!("obsidian://open?vault={}&file={}", vault_name, f_no_ext)
                    }
                    None => format!("obsidian://open?vault={}", vault_name),
                }
            } else {
                let rel_no_ext = rel.trim_end_matches(".md");
                format!(
                    "obsidian://open?vault={}&file={}",
                    vault_name, rel_no_ext
                )
            }
        }
        None => format!("obsidian://open?vault={}", vault_name),
    };

    app.opener().open_url(&uri, None::<&str>).map_err(|e| e.to_string())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("MyLife")
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click { button_state: MouseButtonState::Up, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                window.hide().ok();
                            } else {
                                #[cfg(target_os = "macos")]
                                app.show().ok();
                                #[cfg(target_os = "windows")]
                                let _ = window.move_window(Position::TrayBottomCenter);
                                #[cfg(not(target_os = "windows"))]
                                let _ = window.move_window(Position::TrayBottomCenter);
                                window.show().ok();
                                window.set_focus().ok();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_obsidian_file,
            start_watching,
            create_note,
            complete_routine,
            open_in_obsidian,
            list_subfolders,
            get_vault_path,
            set_vault_path,
            pick_directory,
            initialize_vault,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
