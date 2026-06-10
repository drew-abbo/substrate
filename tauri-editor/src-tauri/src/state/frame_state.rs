use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, RwLock};

pub struct FrameData {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Engine output resolution before preview downscaling.
    pub src_width: u32,
    pub src_height: u32,
    pub generation: u64,
}

/// Shared state between the engine bridge thread and Tauri command handlers.
pub struct FrameState {
    /// Latest CPU-side RGBA frame bytes from the engine. Updated by the bridge thread.
    pub latest_frame: Arc<RwLock<Option<FrameData>>>,
    /// Bumped whenever the node graph changes, so the Vue side can discard stale frames.
    pub generation: Arc<AtomicU64>,
    /// Preview canvas size in physical pixels. Frames are GPU-downscaled to
    /// fit before readback so IPC ships preview-sized payloads, not full res.
    /// Zero means no preview registered; frames pass through at full size.
    pub preview_width: Arc<AtomicU32>,
    pub preview_height: Arc<AtomicU32>,
}

impl FrameState {
    pub fn new() -> Self {
        Self {
            latest_frame: Arc::new(RwLock::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            preview_width: Arc::new(AtomicU32::new(0)),
            preview_height: Arc::new(AtomicU32::new(0)),
        }
    }
}
