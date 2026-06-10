//! Commands for the detached output window's direct wgpu rendering path.

use tauri::{AppHandle, Manager, State};

use crate::state::output_state::{OutputInfo, OutputRect, OutputState, OutputSurface};

/// Create a wgpu surface on the given window so the bridge thread can present
/// engine frames straight to it. Called by the output window's frontend after
/// mount; the webview must be transparent where the video should show.
#[tauri::command]
pub fn attach_output_surface(
    window_label: String,
    app: AppHandle,
    output: State<'_, OutputState>,
) -> Result<(), String> {
    let gpu = output
        .gpu
        .lock()
        .unwrap()
        .clone()
        .ok_or("GPU not ready yet")?;
    let window = app
        .get_webview_window(&window_label)
        .ok_or_else(|| format!("no window '{window_label}'"))?;

    let surface = gpu
        .instance
        .create_surface(window.clone())
        .map_err(|e| format!("failed to create surface: {e}"))?;
    let caps = surface.get_capabilities(&gpu.adapter);
    if caps.formats.is_empty() {
        return Err("GPU adapter cannot present to this window".into());
    }
    let format = pick_format(&caps.formats);

    // Drop the surface before the OS window goes away, otherwise the bridge
    // thread would present to a dead window handle.
    {
        let app = app.clone();
        window.on_window_event(move |event| {
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) {
                if let Some(output) = app.try_state::<OutputState>() {
                    output.surface.lock().unwrap().take();
                    output.rect.lock().unwrap().take();
                }
            }
        });
    }

    util::debug_log_info!("Output surface attached to window '{window_label}' ({format:?})");
    *output.surface.lock().unwrap() = Some(OutputSurface {
        surface,
        window,
        format,
        configured: None,
    });
    Ok(())
}

/// Non-srgb keeps the same passthrough behaviour as the CPU preview path:
/// the engine writes sRGB-encoded values into an Rgba8Unorm texture.
fn pick_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    for preferred in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        if formats.contains(&preferred) {
            return preferred;
        }
    }
    formats[0]
}

#[tauri::command]
pub fn detach_output_surface(output: State<'_, OutputState>) {
    output.surface.lock().unwrap().take();
    output.rect.lock().unwrap().take();
}

/// Where in the output window the video should letterbox, physical pixels.
#[tauri::command]
pub fn set_output_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    output: State<'_, OutputState>,
) {
    *output.rect.lock().unwrap() = Some(OutputRect {
        x,
        y,
        width,
        height,
    });
}

/// Resolution / liveness for the output window's status bar, so it doesn't
/// need to poll full frames in surface mode.
#[tauri::command]
pub fn get_output_info(output: State<'_, OutputState>) -> OutputInfo {
    *output.info.lock().unwrap()
}
