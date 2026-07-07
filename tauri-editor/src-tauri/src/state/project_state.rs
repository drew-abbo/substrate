use std::sync::Mutex;

/// Holds the project ID passed via `--project <id>` from the launcher.
/// `None` when the editor is opened without a project (bare launch).
pub struct ProjectState {
    pub id: Mutex<Option<String>>,
}

impl ProjectState {
    pub fn new(id: Option<String>) -> Self {
        Self { id: Mutex::new(id) }
    }
}
