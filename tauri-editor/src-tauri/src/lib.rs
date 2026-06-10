mod commands;
mod engine_bridge;
mod state;

use state::engine_state::EngineState;
use state::frame_state::FrameState;
use state::output_state::OutputState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A GUI app should never hard-crash on a debug log error — disable the
    // panic-on-errors behaviour that is on by default in debug builds.
    util::debug_log::panic_on_errors::disable();

    tauri::Builder::default()
        .manage(FrameState::new())
        .manage(EngineState::new())
        .manage(OutputState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            engine_bridge::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::frame::get_frame,
            commands::frame::set_preview_size,
            commands::frame::set_target_fps,
            commands::graph::update_graph,
            commands::nodes::get_node_definitions,
            commands::output::attach_output_surface,
            commands::output::detach_output_surface,
            commands::output::set_output_rect,
            commands::output::get_output_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
