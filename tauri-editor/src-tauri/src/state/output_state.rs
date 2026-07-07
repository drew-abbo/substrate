//! Shared state for presenting engine frames directly onto the detached
//! output window's wgpu surface (zero-copy GPU path).

use std::sync::{Arc, Mutex};

use serde::Serialize;

/// GPU objects shared between the bridge thread and command handlers, so the
/// `attach_output_surface` command can create surfaces on the same instance
/// the engine renders with. The bridge thread owns the device and queue.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
}

/// A wgpu surface attached to the detached output window.
pub struct OutputSurface {
    pub surface: wgpu::Surface<'static>,
    pub window: tauri::WebviewWindow,
    pub format: wgpu::TextureFormat,
    /// Size the surface was last configured at, physical pixels.
    pub configured: Option<(u32, u32)>,
}

/// Area of the output window the video letterboxes into, physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OutputRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Lightweight status the output window's UI polls for its status bar,
/// replacing the full-frame `get_frame` polling in surface mode.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputInfo {
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

pub struct OutputState {
    pub gpu: Mutex<Option<Arc<GpuContext>>>,
    pub surface: Mutex<Option<OutputSurface>>,
    pub rect: Mutex<Option<OutputRect>>,
    pub info: Mutex<OutputInfo>,
}

impl OutputState {
    pub fn new() -> Self {
        Self {
            gpu: Mutex::new(None),
            surface: Mutex::new(None),
            rect: Mutex::new(None),
            info: Mutex::new(OutputInfo::default()),
        }
    }
}
