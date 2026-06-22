use std::sync::atomic::Ordering;

use engine::engine_outpost::EngineCommand;
use media::fps::Fps;
use tauri::State;
use tauri::ipc::Response;

use crate::state::engine_state::EngineState;
use crate::state::frame_state::FrameState;

/// Pull the latest rendered frame. Called by the Vue rAF loop.
///
/// Returns a raw binary payload (an `ArrayBuffer` on the JS side) so frame
/// data skips JSON serialization: little-endian `width: u32`, `height: u32`,
/// `src_width: u32`, `src_height: u32`, `generation: u64`, then tightly
/// packed RGBA pixels. The src dimensions are the engine output resolution
/// before preview downscaling. An empty payload means no frame yet.
#[tauri::command]
pub fn get_frame(state: State<'_, FrameState>) -> Response {
    let lock = state.latest_frame.read().unwrap();
    let Some(frame) = lock.as_ref() else {
        return Response::new(Vec::new());
    };

    let mut payload = Vec::with_capacity(24 + frame.bytes.len());
    payload.extend_from_slice(&frame.width.to_le_bytes());
    payload.extend_from_slice(&frame.height.to_le_bytes());
    payload.extend_from_slice(&frame.src_width.to_le_bytes());
    payload.extend_from_slice(&frame.src_height.to_le_bytes());
    payload.extend_from_slice(&frame.generation.to_le_bytes());
    payload.extend_from_slice(&frame.bytes);
    Response::new(payload)
}

/// Tell the bridge how large the preview canvas is (physical pixels) so
/// frames can be GPU-downscaled to that size before the CPU readback.
#[tauri::command]
pub fn set_preview_size(width: u32, height: u32, state: State<'_, FrameState>) {
    state.preview_width.store(width, Ordering::Relaxed);
    state.preview_height.store(height, Ordering::Relaxed);
}

/// Override the engine tick rate with a fixed FPS.
#[tauri::command]
pub fn set_target_fps(fps: f64, engine: State<'_, EngineState>) -> Result<(), String> {
    let fps = Fps::from_float(fps).map_err(|e| e.to_string())?;
    engine.send(EngineCommand::SetGlobalStreamTargetFps(fps))
}

/// Clear the manual FPS override and resume auto-adjusting from the output node.
#[tauri::command]
pub fn clear_target_fps(engine: State<'_, EngineState>) -> Result<(), String> {
    engine.send(EngineCommand::ClearManualFps)
}
