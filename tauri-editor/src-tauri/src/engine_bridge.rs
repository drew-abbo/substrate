//! Bridge between the engine outpost and the Tauri frontend.
//!
//! Creates a wgpu device (no window surface at startup), spawns the engine
//! outpost on it, then loops over `FrameReady` events delivering frames two
//! ways:
//!
//! 1. **Preview** (floating panel in the editor): the frame is GPU-downscaled
//!    to the preview's pixel size, read back to CPU, and polled by the webview
//!    via `get_frame`. Downscaling first keeps the IPC payload tiny.
//! 2. **Detached output window**: the frame is blitted straight onto a wgpu
//!    surface attached to that window (see `commands::output`), never leaving
//!    the GPU. The webview on top is transparent where the video shows.

pub mod blit;
pub mod readback;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use engine::GpuFrame;
use engine::engine_outpost::{EngineOutpostEvent, EventFilter, EventKind};
use engine::node::NodeLibrary;
use media::frame::Uid;
use tauri::{AppHandle, Manager};

use crate::state::engine_state::EngineState;
use crate::state::frame_state::{FrameData, FrameState};
use crate::state::output_state::{GpuContext, OutputRect, OutputState};

/// Engine output format. RGBA so readback bytes go straight into a canvas
/// `ImageData` without channel swizzling.
const ENGINE_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// How long the bridge sleeps between event polls when no frame has arrived.
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(2);

/// Letterbox bar colour on the output surface: `--bg-screen` (#03030a).
const SCREEN_CLEAR: wgpu::Color = wgpu::Color {
    r: 3.0 / 255.0,
    g: 3.0 / 255.0,
    b: 10.0 / 255.0,
    a: 1.0,
};

/// Spawn the bridge thread. Engine startup failures are logged rather than
/// fatal so the editor UI still comes up without a GPU.
pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("engine-bridge".into())
        .spawn(move || {
            if let Err(e) = run(app) {
                util::debug_log_error!("Engine bridge failed: {e}");
            }
        })
        .expect("failed to spawn engine-bridge thread");
}

fn run(app: AppHandle) -> Result<(), String> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .map_err(|e| format!("no suitable GPU adapter: {e}"))?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("engine-bridge-device"),
        ..Default::default()
    }))
    .map_err(|e| format!("failed to create GPU device: {e}"))?;

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let library = Arc::new(
        NodeLibrary::load_all().map_err(|e| format!("failed to load node library: {e}"))?,
    );

    util::debug_log_info!("Spawning engine outpost");
    let handle = engine::spawn(
        device.clone(),
        queue.clone(),
        library.clone(),
        ENGINE_TEXTURE_FORMAT,
    );

    let engine_state = app.state::<EngineState>();
    *engine_state.command_sender.lock().unwrap() = Some(handle.command_sender());
    *engine_state.library.lock().unwrap() = Some(library);

    let output_state = app.state::<OutputState>();
    *output_state.gpu.lock().unwrap() = Some(Arc::new(GpuContext { instance, adapter }));

    let events = handle.subscribe(EventFilter::Only(vec![EventKind::FrameReady]));
    let frame_state = app.state::<FrameState>();

    let mut bridge = FrameBridge::new(device, queue);

    util::debug_log_info!("Engine bridge entering frame loop");

    // `handle` stays owned by this loop; dropping it would shut the engine down.
    let mut last_frame: Option<GpuFrame> = None;
    loop {
        // Keep only the newest frame if the engine outpaced us.
        let mut frame = None;
        for event in events.drain() {
            if let EngineOutpostEvent::FrameReady(f) = event {
                frame = Some(f);
            }
        }

        let new_frame = frame.is_some();
        if let Some(f) = frame {
            last_frame = Some(f);
        }

        let Some(frame) = last_frame.as_ref() else {
            std::thread::sleep(EVENT_POLL_INTERVAL);
            continue;
        };

        if new_frame {
            {
                let mut info = output_state.info.lock().unwrap();
                info.width = frame.size.width;
                info.height = frame.size.height;
                info.active = true;
            }
            if let Err(e) = bridge.update_preview(frame, &frame_state) {
                util::debug_log_warning!("Frame readback failed: {e}");
            }
        }

        // Presents again on window resize even without a new engine frame.
        if let Err(e) = bridge.present_output(frame, &output_state) {
            util::debug_log_warning!("Output present failed: {e}");
        }

        if !new_frame {
            std::thread::sleep(EVENT_POLL_INTERVAL);
        }
    }
}

/// Everything the frame loop needs to downscale, read back, and present.
struct FrameBridge {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    blitter: blit::Blitter,
    readback: readback::FrameReadback,
    /// Cached downscale target, recreated when the preview size changes.
    preview: Option<(wgpu::Texture, wgpu::TextureView, u32, u32)>,
    /// What was last presented to the output surface, to skip redundant work.
    last_present: Option<(Uid, (u32, u32), Option<OutputRect>)>,
}

impl FrameBridge {
    fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            blitter: blit::Blitter::new(device.clone()),
            readback: readback::FrameReadback::new(device.clone(), queue.clone()),
            device,
            queue,
            preview: None,
            last_present: None,
        }
    }

    /// Downscale the frame to preview size on the GPU, then read it back for
    /// the `get_frame` IPC path.
    fn update_preview(
        &mut self,
        frame: &GpuFrame,
        frame_state: &FrameState,
    ) -> Result<(), String> {
        let (src_w, src_h) = (frame.size.width, frame.size.height);
        let max_w = frame_state.preview_width.load(Ordering::Relaxed);
        let max_h = frame_state.preview_height.load(Ordering::Relaxed);

        let downscale = max_w > 0 && max_h > 0 && (max_w < src_w || max_h < src_h);
        let (texture, width, height) = if downscale {
            let (w, h) = fit_dims(src_w, src_h, max_w, max_h);
            let (texture, view) = self.preview_target(w, h);
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("preview-downscale"),
                });
            self.blitter.blit(
                &mut encoder,
                frame.view(),
                &view,
                ENGINE_TEXTURE_FORMAT,
                blit::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: w as f32,
                    height: h as f32,
                },
                Some(wgpu::Color::BLACK),
            );
            self.queue.submit(Some(encoder.finish()));
            (texture, w, h)
        } else {
            ((*frame.texture).clone(), src_w, src_h)
        };

        let cpu = self.readback.read(&texture, width, height)?;
        *frame_state.latest_frame.write().unwrap() = Some(FrameData {
            bytes: cpu.bytes,
            width: cpu.width,
            height: cpu.height,
            src_width: src_w,
            src_height: src_h,
            generation: frame_state.generation.load(Ordering::SeqCst),
        });
        Ok(())
    }

    /// Blit the frame onto the detached output window's surface, letterboxed
    /// into the rect the frontend reported. No-op without an attached surface.
    fn present_output(&mut self, frame: &GpuFrame, output_state: &OutputState) -> Result<(), String> {
        let mut surface_lock = output_state.surface.lock().unwrap();
        let Some(out) = surface_lock.as_mut() else {
            self.last_present = None;
            return Ok(());
        };

        let size = out.window.inner_size().map_err(|e| e.to_string())?;
        if size.width == 0 || size.height == 0 {
            return Ok(()); // minimized
        }

        let rect = *output_state.rect.lock().unwrap();
        let key = (frame.frame_id(), (size.width, size.height), rect);
        if self.last_present == Some(key) {
            return Ok(());
        }

        if out.configured != Some((size.width, size.height)) {
            out.surface.configure(
                &self.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: out.format,
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::AutoVsync,
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                },
            );
            out.configured = Some((size.width, size.height));
        }

        let surface_tex = match out.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                out.configured = None; // reconfigure next iteration
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(e) => return Err(format!("surface error: {e}")),
        };
        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let rect_px = rect.unwrap_or(OutputRect {
            x: 0.0,
            y: 0.0,
            width: size.width as f64,
            height: size.height as f64,
        });
        let viewport = letterbox(
            frame.size.width,
            frame.size.height,
            rect_px,
            (size.width, size.height),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("output-present"),
            });
        match viewport {
            Some(viewport) => self.blitter.blit(
                &mut encoder,
                frame.view(),
                &view,
                out.format,
                viewport,
                Some(SCREEN_CLEAR),
            ),
            None => blit::Blitter::clear(&mut encoder, &view, SCREEN_CLEAR),
        }
        self.queue.submit(Some(encoder.finish()));
        surface_tex.present();

        self.last_present = Some(key);
        Ok(())
    }

    /// wgpu resources are internally reference counted, so returning clones
    /// of the cached texture/view is cheap.
    fn preview_target(&mut self, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        if self
            .preview
            .as_ref()
            .is_none_or(|(_, _, w, h)| (*w, *h) != (width, height))
        {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("preview-downscale-target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ENGINE_TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.preview = Some((texture, view, width, height));
        }
        let (texture, view, ..) = self.preview.as_ref().unwrap();
        (texture.clone(), view.clone())
    }
}

/// Scale `src` down to fit within `max` preserving aspect ratio. Never
/// upscales.
fn fit_dims(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let scale = f64::min(
        1.0,
        f64::min(
            max_w as f64 / src_w as f64,
            max_h as f64 / src_h as f64,
        ),
    );
    (
        ((src_w as f64 * scale).round() as u32).max(1),
        ((src_h as f64 * scale).round() as u32).max(1),
    )
}

/// Aspect-fit the frame into `rect`, clamped to the surface bounds. Returns
/// `None` when there is no visible area to draw into.
fn letterbox(
    frame_w: u32,
    frame_h: u32,
    rect: OutputRect,
    surface: (u32, u32),
) -> Option<blit::Viewport> {
    if frame_w == 0 || frame_h == 0 {
        return None;
    }
    let start_x = rect.x.max(0.0);
    let start_y = rect.y.max(0.0);
    let end_x = (rect.x + rect.width).min(surface.0 as f64);
    let end_y = (rect.y + rect.height).min(surface.1 as f64);
    let (rect_w, rect_h) = (end_x - start_x, end_y - start_y);
    if rect_w < 1.0 || rect_h < 1.0 {
        return None;
    }

    let scale = f64::min(rect_w / frame_w as f64, rect_h / frame_h as f64);
    let (w, h) = (frame_w as f64 * scale, frame_h as f64 * scale);
    Some(blit::Viewport {
        x: (start_x + (rect_w - w) / 2.0) as f32,
        y: (start_y + (rect_h - h) / 2.0) as f32,
        width: w as f32,
        height: h as f32,
    })
}
