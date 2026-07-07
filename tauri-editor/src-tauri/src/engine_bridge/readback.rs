//! GPU→CPU frame readback for IPC to the webview.

use std::sync::Arc;
use std::sync::mpsc;

/// A frame copied out of GPU memory as tightly packed RGBA bytes.
pub struct CpuFrame {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Engine output resolution at the time this copy was submitted, before
    /// preview downscaling — captured alongside the copy so it can't drift out
    /// of sync with `bytes` across the readback's multi-tick latency.
    pub src_width: u32,
    pub src_height: u32,
}

/// Everything needed to finish a copy+map_async submitted on a previous tick.
struct PendingReadback {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    swap_bgra: bool,
    src_width: u32,
    src_height: u32,
    rx: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

/// What to copy out of GPU memory on a given tick.
pub struct ReadbackRequest<'a> {
    pub texture: &'a wgpu::Texture,
    pub width: u32,
    pub height: u32,
    pub src_width: u32,
    pub src_height: u32,
}

/// Copies engine output textures into CPU memory, reusing the staging buffer
/// across frames of the same size.
///
/// Readback is pipelined across bridge ticks rather than blocking: each call
/// polls the GPU non-blockingly and submits at most one new copy, only once
/// the previous one has finished mapping. This never stalls the shared wgpu
/// queue the engine renders on — the trade-off is a tick or two of latency
/// between a frame rendering and its bytes becoming available, which is
/// invisible at the bridge's ~2ms poll interval.
pub struct FrameReadback {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Buffer ready to receive the next copy. Only touched when `pending` is
    /// `None`, so a size change (e.g. preview resize) can never disturb a
    /// buffer that's still being read back.
    staging: Option<(wgpu::Buffer, u64)>,
    pending: Option<PendingReadback>,
}

impl FrameReadback {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            staging: None,
            pending: None,
        }
    }

    /// Advances any in-flight readback and, once free, submits a copy of
    /// `req`. Returns `Ok(None)` when no new frame is ready yet — the caller
    /// should keep showing the last frame and call again next tick.
    pub fn read(&mut self, req: ReadbackRequest) -> Result<Option<CpuFrame>, String> {
        if req.width == 0 || req.height == 0 {
            return Err("zero-sized frame".to_string());
        }

        // Non-blocking: just checks already-completed work, never waits.
        let _ = self.device.poll(wgpu::PollType::Poll);

        let ready = self.take_ready_frame()?;

        // Only one readback in flight at a time. If the previous one hasn't
        // finished mapping yet, skip submitting a new copy this tick instead
        // of blocking for a free buffer — the next tick will pick up whatever
        // the newest frame is by then.
        if self.pending.is_none() {
            self.submit_copy(req)?;
        }

        Ok(ready)
    }

    /// Non-blocking check of the in-flight readback.
    fn take_ready_frame(&mut self) -> Result<Option<CpuFrame>, String> {
        let Some(pending) = self.pending.take() else {
            return Ok(None);
        };

        match pending.rx.try_recv() {
            Ok(Ok(())) => {
                let mut bytes = {
                    let data = pending.buffer.slice(..).get_mapped_range();
                    strip_row_padding(&data, pending.width, pending.height, pending.padded_bytes_per_row)
                };
                pending.buffer.unmap();
                for pixel in bytes.chunks_exact_mut(4) {
                    if pending.swap_bgra {
                        pixel.swap(0, 2);
                    }
                    // Canvas compositing honours alpha; engine output is opaque.
                    pixel[3] = 255;
                }
                Ok(Some(CpuFrame {
                    bytes,
                    width: pending.width,
                    height: pending.height,
                    src_width: pending.src_width,
                    src_height: pending.src_height,
                }))
            }
            Ok(Err(e)) => {
                pending.buffer.unmap();
                Err(format!("buffer map failed: {e:?}"))
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Still mapping; keep waiting on this same readback next tick.
                self.pending = Some(pending);
                Ok(None)
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                pending.buffer.unmap();
                Err("map callback dropped".to_string())
            }
        }
    }

    /// Copies the requested texture into the staging buffer and kicks off an
    /// async map; `take_ready_frame` picks up the result on a later tick.
    fn submit_copy(&mut self, req: ReadbackRequest) -> Result<(), String> {
        let ReadbackRequest {
            texture,
            width,
            height,
            src_width,
            src_height,
        } = req;

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
        // ORDERING: the caller must ensure this readback is submitted to the
        // queue AFTER any `write_texture` that fills `texture`. Both share
        // this queue, so submitting the readback encoder after the upload
        // encoder guarantees correct ordering.
        self.queue.submit(Some(encoder.finish()));

        let swap_bgra = matches!(
            texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );

        let slice = buffer.slice(..buffer_size);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        self.pending = Some(PendingReadback {
            buffer,
            width,
            height,
            padded_bytes_per_row,
            swap_bgra,
            src_width,
            src_height,
            rx,
        });
        Ok(())
    }

    /// wgpu resources are internally reference counted, so returning a clone
    /// of the cached buffer is cheap. Only called when no readback is in
    /// flight (see `staging`'s doc comment).
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
    // `extend_from_slice` writes each row directly into spare capacity, unlike
    // `vec![0; n]` which would zero-fill the whole buffer before every byte
    // gets overwritten by the loop below anyway.
    let mut out = Vec::with_capacity(unpadded * height as usize);
    for row in 0..height as usize {
        out.extend_from_slice(&padded[row * padded_row..row * padded_row + unpadded]);
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
