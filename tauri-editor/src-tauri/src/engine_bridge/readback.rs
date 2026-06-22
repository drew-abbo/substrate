//! GPU→CPU frame readback for IPC to the webview.

use std::sync::Arc;
use std::sync::mpsc;

/// A frame copied out of GPU memory as tightly packed RGBA bytes.
pub struct CpuFrame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Copies engine output textures into CPU memory, reusing the staging buffer
/// across frames of the same size.
pub struct FrameReadback {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    staging: Option<(wgpu::Buffer, u64)>,
}

impl FrameReadback {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            staging: None,
        }
    }

    pub fn read(&mut self, texture: &wgpu::Texture, width: u32, height: u32) -> Result<CpuFrame, String> {
        if width == 0 || height == 0 {
            return Err("zero-sized frame".to_string());
        }

        let unpadded_bytes_per_row = width * 4;
        // WebGPU requires bytes_per_row alignment for copy_texture_to_buffer.
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;

        let buffer = self.staging_buffer(buffer_size);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..buffer_size);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| format!("device poll failed: {e:?}"))?;
        rx.recv()
            .map_err(|_| "map callback dropped".to_string())?
            .map_err(|e| format!("buffer map failed: {e:?}"))?;

        let swap_bgra = matches!(
            texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );

        let mut bytes = {
            let data = slice.get_mapped_range();
            strip_row_padding(&data, width, height, padded_bytes_per_row)
        };
        buffer.unmap();

        for pixel in bytes.chunks_exact_mut(4) {
            if swap_bgra {
                pixel.swap(0, 2);
            }
            // Canvas compositing honours alpha; engine output is opaque.
            pixel[3] = 255;
        }

        Ok(CpuFrame {
            bytes,
            width,
            height,
        })
    }

    /// wgpu resources are internally reference counted, so returning a clone
    /// of the cached buffer is cheap.
    ///
    /// ORDERING: the caller must ensure `copy_texture_to_buffer` for this
    /// readback is submitted to the queue AFTER any `write_texture` that fills
    /// the source texture. Both operations share the same queue, so submitting
    /// the readback encoder after the upload encoder guarantees correct ordering.
    fn staging_buffer(&mut self, size: u64) -> wgpu::Buffer {
        if self.staging.as_ref().is_none_or(|(_, s)| *s != size) {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frame-readback-staging"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            self.staging = Some((buffer, size));
        }
        self.staging.as_ref().unwrap().0.clone()
    }
}

/// Copies rows from a WebGPU-padded buffer into a tightly-packed output vec,
/// stripping the alignment padding that `copy_texture_to_buffer` adds to each row.
fn strip_row_padding(padded: &[u8], width: u32, height: u32, padded_bytes_per_row: u32) -> Vec<u8> {
    let unpadded = (width * 4) as usize;
    let padded_row = padded_bytes_per_row as usize;
    let mut out = vec![0u8; unpadded * height as usize];
    for row in 0..height as usize {
        out[row * unpadded..(row + 1) * unpadded]
            .copy_from_slice(&padded[row * padded_row..row * padded_row + unpadded]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_row_padding_single_row_removes_alignment() {
        // 2-pixel wide row = 8 bytes of real data, padded to 256
        let padded_bpr = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let mut data = vec![0u8; padded_bpr as usize];
        data[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let out = strip_row_padding(&data, 2, 1, padded_bpr);
        assert_eq!(out, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn strip_row_padding_multi_row_extracts_correct_rows() {
        let padded_bpr = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let mut data = vec![0u8; padded_bpr as usize * 2];
        // Row 0: single pixel [10, 20, 30, 40]
        data[0..4].copy_from_slice(&[10, 20, 30, 40]);
        // Row 1: single pixel [50, 60, 70, 80]
        data[padded_bpr as usize..padded_bpr as usize + 4].copy_from_slice(&[50, 60, 70, 80]);
        let out = strip_row_padding(&data, 1, 2, padded_bpr);
        assert_eq!(out, vec![10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn strip_row_padding_tightly_packed_is_identity() {
        // When padded_bpr equals unpadded_bpr, strip is a no-op copy
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let out = strip_row_padding(&data, 2, 1, 8);
        assert_eq!(out, data);
    }
}
