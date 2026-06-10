use media::frame::Uid;
use std::sync::Arc;

/// GPU frame handle with its dimensions. Holds a texture and view plus its size so
/// downstream consumers can size new textures correctly.
///
/// The `texture` field is needed for GPU→CPU readback (e.g. Tauri IPC to a webview).
/// Use `copy_texture_to_buffer` against `texture` after verifying `COPY_SRC` usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuFrame {
    pub texture: Arc<wgpu::Texture>,
    pub view: Arc<wgpu::TextureView>,
    pub size: wgpu::Extent3d,
    pub frame_id: Uid,
}

impl GpuFrame {
    pub fn new(
        texture: Arc<wgpu::Texture>,
        view: wgpu::TextureView,
        size: wgpu::Extent3d,
        frame_id: Uid,
    ) -> Self {
        Self {
            texture,
            view: Arc::new(view),
            size,
            frame_id,
        }
    }

    pub fn texture(&self) -> &Arc<wgpu::Texture> {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn size(&self) -> wgpu::Extent3d {
        self.size
    }

    pub fn frame_id(&self) -> Uid {
        self.frame_id
    }
}
