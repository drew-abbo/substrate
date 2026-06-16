use engine::engine_outpost::EngineCommand;
use std::sync::atomic::Ordering;
use tauri::State;

use crate::state::engine_state::EngineState;

#[tauri::command]
pub fn play_streams(engine: State<'_, EngineState>) -> Result<(), String> {
    engine.paused.store(false, Ordering::SeqCst);
    engine.send(EngineCommand::PlayStreams)
}

#[tauri::command]
pub fn pause_streams(engine: State<'_, EngineState>) -> Result<(), String> {
    engine.paused.store(true, Ordering::SeqCst);
    engine.send(EngineCommand::PauseStreams)
}

/// Returns true when the engine is playing (not paused). Defaults to true on startup.
#[tauri::command]
pub fn get_playback_state(engine: State<'_, EngineState>) -> bool {
    !engine.paused.load(Ordering::SeqCst)
}
