use std::process::Command;
use std::time::SystemTime;

use serde::Serialize;
use tauri::{command, Manager};
use util::local_data::{
    self,
    project::{Project, ProjectHeader, ProjectId, ProjectInfo},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    id: String,
    name: String,
    created: String,
    last_edited: Option<String>,
}

/// Build a DTO and a sort key from a loaded project.
fn into_dto(project: Project) -> (Option<SystemTime>, ProjectDto) {
    let (last_edited_time, last_edited) = match project.last_edited_with_string() {
        Ok(Some((t, s))) => (Some(t), Some(s)),
        _ => (None, None),
    };
    let info = project.cached_info().clone();
    let dto = ProjectDto {
        id: String::from(info.id().clone()),
        name: info.name().to_string(),
        created: info.created_string(),
        last_edited,
    };
    (last_edited_time, dto)
}

/// List all projects, sorted by last-edited descending (never-edited last).
#[command]
pub fn list_projects() -> Result<Vec<ProjectDto>, String> {
    let entries = std::fs::read_dir(local_data::projects_path()).map_err(|e| e.to_string())?;

    let mut items: Vec<(Option<SystemTime>, ProjectDto)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let id_str = entry.file_name().to_string_lossy().into_owned();
            let id = ProjectId::try_from(id_str).ok()?;
            let project = Project::load(&id).ok()?;
            Some(into_dto(project))
        })
        .collect();

    items.sort_unstable_by(|(a, _), (b, _)| match (a, b) {
        (Some(a), Some(b)) => b.cmp(a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    Ok(items.into_iter().map(|(_, dto)| dto).collect())
}

/// Create a new project with the given name.
#[command]
pub fn create_project(name: String) -> Result<ProjectDto, String> {
    let project = ProjectInfo::new(name)
        .create_project()
        .map_err(|e| e.to_string())?;
    let (_, dto) = into_dto(project);
    Ok(dto)
}

/// Rename an existing project.
#[command]
pub fn rename_project(id: String, name: String) -> Result<(), String> {
    let project_id = ProjectId::try_from(id).map_err(|e: util::local_data::project::ProjectError| e.to_string())?;
    let mut project = Project::load(&project_id).map_err(|e| e.to_string())?;
    project
        .with_info_mut(|info| *info.name_mut() = name)
        .map_err(|e| e.to_string())
}

/// Permanently delete a project from disk.
#[command]
pub fn delete_project(id: String) -> Result<(), String> {
    let project_id = ProjectId::try_from(id).map_err(|e: util::local_data::project::ProjectError| e.to_string())?;
    let project = Project::load(&project_id).map_err(|e| e.to_string())?;
    project.delete().map_err(|e| e.to_string())
}

/// Open the project's directory in the OS file manager.
#[command]
pub fn show_project_in_explorer(id: String) -> Result<(), String> {
    let project_id = ProjectId::try_from(id)
        .map_err(|e: util::local_data::project::ProjectError| e.to_string())?;
    let project = Project::load(&project_id).map_err(|e| e.to_string())?;
    let dir = project.dir_path().to_owned();
    open_in_file_manager(&dir).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn open_in_file_manager(path: &std::path::Path) -> std::io::Result<()> {
    Command::new("explorer.exe").arg(path).spawn().map(|_| ())
}

#[cfg(target_os = "macos")]
fn open_in_file_manager(path: &std::path::Path) -> std::io::Result<()> {
    Command::new("open").arg(path).spawn().map(|_| ())
}

#[cfg(target_os = "linux")]
fn open_in_file_manager(path: &std::path::Path) -> std::io::Result<()> {
    Command::new("xdg-open").arg(path).spawn().map(|_| ())
}

/// Spawn the editor binary with the given project ID.
///
/// When `keep_open` is false the launcher window is hidden immediately and a
/// background thread re-shows it once the editor process exits.
#[command]
pub fn open_project(
    app: tauri::AppHandle,
    id: String,
    keep_open: bool,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    const EDITOR_BIN: &str = "tauri-editor.exe";
    #[cfg(not(target_os = "windows"))]
    const EDITOR_BIN: &str = "tauri-editor";

    let editor = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or_else(|| "Cannot determine launcher directory".to_string())?
        .join(EDITOR_BIN);

    let mut child = Command::new(&editor)
        .arg("--project")
        .arg(&id)
        .spawn()
        .map_err(|e| format!("Failed to launch editor ({editor:?}): {e}"))?;

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    std::thread::spawn(move || {
        let _ = child.wait();
        if keep_open {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        } else {
            std::process::exit(0);
        }
    });

    Ok(())
}
