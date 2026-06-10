use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use engine::engine_outpost::{EngineCommand, EngineCommandSender};
use engine::node::NodeLibrary;
use engine::node_graph::EngineNodeId;

/// Engine connection shared with Tauri command handlers.
///
/// `command_sender` and `library` start out empty and are populated by the
/// bridge thread once the engine outpost is up. Commands invoked before that
/// point report "engine not ready".
pub struct EngineState {
    pub command_sender: Mutex<Option<EngineCommandSender>>,
    pub library: Mutex<Option<Arc<NodeLibrary>>>,
    /// Stable mapping from frontend node ids to engine node ids so stream
    /// state inside the engine survives graph edits.
    pub node_ids: Mutex<HashMap<String, EngineNodeId>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            command_sender: Mutex::new(None),
            library: Mutex::new(None),
            node_ids: Mutex::new(HashMap::new()),
        }
    }

    /// Send a command to the engine outpost, if it is running.
    pub fn send(&self, command: EngineCommand) -> Result<(), String> {
        let sender = self.command_sender.lock().unwrap();
        let sender = sender.as_ref().ok_or("engine not ready")?;
        sender
            .send(command)
            .map_err(|_| "engine disconnected".to_string())?;
        Ok(())
    }
}
