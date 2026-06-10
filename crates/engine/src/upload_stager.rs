//! Small helper for reusing an upload texture when copying CPU RGBA data
//! to the GPU. [UploadStager] owns a temporary [wgpu::Texture] sized to the
//! largest upload seen and exposes [cpu_to_gpu_rgba] to write CPU memory into
//! that texture and return a [wgpu::TextureView] suitable for sampling in shaders.
//!
//! This abstraction avoids allocating a new GPU texture every frame when
//! feeding CPU-decoded frames into the pipeline.
use std::sync::Arc;

use crate::engine_errors::EngineError;

/// Stages CPU RGBA data into a GPU texture and returns a view + shared texture handle.
///
/// The stager lazily allocates a backing texture sized to the requested
/// dimensions; subsequent calls with equal-or-smaller sizes reuse the same
/// texture. If a larger size is requested the backing texture is recreated.
///
/// The returned `Arc<wgpu::Texture>` enables GPU→CPU readback by callers
/// (e.g. the Tauri frame bridge) without exposing internal stager state.
pub struct UploadStager {
    tex: Option<Arc<wgpu::Texture>>,
    extent: wgpu::Extent3d,
}

impl Default for UploadStager {
    fn default() -> Self {
        Self::new()
    }
}

impl UploadStager {
    /// Create a new UploadStager.
    pub fn new() -> Self {
        Self {
            tex: None,
            extent: wgpu::Extent3d {
                width: 0,
                height: 0,
                depth_or_array_layers: 1,
            },
        }
    }

    /// Ensure the internal texture matches the requested size.
    fn ensure_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.extent.width == width && self.extent.height == height && self.tex.is_some() {
            return;
        }

        self.extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("upload_texture"),
            size: self.extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.tex = Some(Arc::new(tex));
    }

    /// Blit RGBA pixel data from CPU memory into the staging texture and
    /// return a `(TextureView, Arc<Texture>)` pair.
    ///
    /// The `Arc<Texture>` allows callers to do GPU→CPU readback without
    /// accessing stager internals. Note that the stager reuses the same
    /// texture across calls of equal dimensions, so do any readback before
    /// the next upload.
    pub fn cpu_to_gpu_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<(wgpu::TextureView, Arc<wgpu::Texture>), EngineError> {
        let expected_size = (width * height * 4) as usize;

        if data.len() < expected_size {
            return Err(EngineError::DataSizeMismatch {
                expected: expected_size,
                actual: data.len(),
            });
        }

        self.ensure_texture(device, width, height);

        let tex = self.tex.as_ref().ok_or(EngineError::TextureNotInitialized)?;

        // Copy CPU data into the GPU texture using a staged write.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            self.extent,
        );

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        Ok((view, tex.clone()))
    }
}
