use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;
use util::local_data;

use crate::state::project_state::ProjectState;

const DATA_FILE: &str = "data.json";

// ── Saved graph types ─────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedGraph {
    pub nodes: Vec<SavedNode>,
    pub edges: Vec<SavedEdge>,
    pub viewport: Option<SavedViewport>,
}

/// One saved node. `node_type` is `"output"` for the output sink node,
/// otherwise it is the engine node type string (e.g. `"ColorCorrect"`).
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedNode {
    pub id: String,
    pub node_type: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub values: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedEdge {
    pub id: String,
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct SavedViewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

// ── Helpers ───────────────────────────────────────────────

fn data_path(project_id: &str) -> std::path::PathBuf {
    local_data::projects_path()
        .join(project_id)
        .join(DATA_FILE)
}

// ── Commands ──────────────────────────────────────────────

/// Returns the project ID the editor was opened with, or `null` if none.
#[tauri::command]
pub fn get_project_id(state: State<'_, ProjectState>) -> Option<String> {
    state.id.lock().unwrap().clone()
}

/// Read the saved graph for the current project. Returns an empty graph when
/// no project is open or no data file exists yet.
#[tauri::command]
pub fn load_project(state: State<'_, ProjectState>) -> Result<SavedGraph, String> {
    let guard = state.id.lock().unwrap();
    let Some(id) = guard.as_deref() else {
        return Ok(SavedGraph::default());
    };
    let path = data_path(id);
    if !path.exists() {
        return Ok(SavedGraph::default());
    }
    let file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    serde_json::from_reader(file).map_err(|e| e.to_string())
}

/// Persist the current graph to the project's `data.json`.
/// No-op when no project is open.
#[tauri::command]
pub fn save_project(
    graph: SavedGraph,
    state: State<'_, ProjectState>,
) -> Result<(), String> {
    let guard = state.id.lock().unwrap();
    let Some(id) = guard.as_deref() else {
        return Ok(());
    };
    let path = data_path(id);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(file, &graph).map_err(|e| e.to_string())
}

/// Called from the frontend after the JS close-listener has been unregistered.
/// Closing via the window (not app.exit) lets WebView2 unregister its window
/// classes cleanly, avoiding the Chrome_WidgetWin_0 ERROR_CLASS_HAS_WINDOWS log.
#[tauri::command]
pub fn close_editor(app: tauri::AppHandle) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.close();
    }
}
