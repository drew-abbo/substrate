mod commands;
mod engine_bridge;
mod state;

use state::engine_state::EngineState;
use state::frame_state::FrameState;
use state::output_state::OutputState;
use state::project_state::ProjectState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A GUI app should never hard-crash on a debug log error — disable the
    // panic-on-errors behaviour that is on by default in debug builds.
    util::debug_log::panic_on_errors::disable();

    // The launcher spawns us as: tauri-editor --project <id>
    let project_id = {
        let args: Vec<String> = std::env::args().collect();
        args.windows(2)
            .find(|w| w[0] == "--project")
            .map(|w| w[1].clone())
    };

    tauri::Builder::default()
        .manage(FrameState::new())
        .manage(EngineState::new())
        .manage(OutputState::new())
        .manage(ProjectState::new(project_id))
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
            commands::frame::clear_target_fps,
            commands::graph::update_graph,
            commands::nodes::get_node_definitions,
            commands::nodes::list_midi_ports,
            commands::output::attach_output_surface,
            commands::output::detach_output_surface,
            commands::output::set_output_rect,
            commands::output::get_output_info,
            commands::project::get_project_id,
            commands::project::load_project,
            commands::project::save_project,
            commands::project::close_editor,
            commands::playback::play_streams,
            commands::playback::pause_streams,
            commands::playback::get_playback_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
