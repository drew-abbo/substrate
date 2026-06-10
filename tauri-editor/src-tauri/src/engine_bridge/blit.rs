//! GPU blit pass: samples a source texture and draws it into a target view.
//!
//! Used to downscale engine frames to preview size before CPU readback, and
//! to present frames directly onto the detached output window's surface.

use std::collections::HashMap;
use std::sync::Arc;

const SHADER: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    // Fullscreen triangle; the render pass viewport decides where it lands.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(src, src_sampler, in.uv).rgb, 1.0);
}
"#;

/// Where on the target the source should be drawn, in target pixels.
/// Must lie within the target's bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Blitter {
    device: Arc<wgpu::Device>,
    sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    /// One pipeline per target format (preview texture vs. window surface).
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}

impl Blitter {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit-bind-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit-pipeline-layout"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });

        Self {
            device,
            sampler,
            bind_layout,
            pipeline_layout,
            shader,
            pipelines: HashMap::new(),
        }
    }

    /// Draw `src` into `target` at `viewport`, optionally clearing the whole
    /// target first.
    pub fn blit(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        target: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        viewport: Viewport,
        clear: Option<wgpu::Color>,
    ) {
        let pipeline = self.pipeline(target_format);
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bind"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = Self::begin_pass(encoder, target, clear);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.set_viewport(viewport.x, viewport.y, viewport.width, viewport.height, 0.0, 1.0);
        pass.draw(0..3, 0..1);
    }

    /// Clear the target without drawing anything (e.g. no frame fits the rect).
    pub fn clear(encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, color: wgpu::Color) {
        Self::begin_pass(encoder, target, Some(color));
    }

    fn begin_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clear: Option<wgpu::Color>,
    ) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match clear {
                        Some(color) => wgpu::LoadOp::Clear(color),
                        None => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }

    /// wgpu resources are internally reference counted, so cloning the cached
    /// pipeline is cheap.
    fn pipeline(&mut self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        self.pipelines
            .entry(format)
            .or_insert_with(|| {
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("blit-pipeline"),
                        layout: Some(&self.pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &self.shader,
                            entry_point: Some("vs_main"),
                            compilation_options: Default::default(),
                            buffers: &[],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &self.shader,
                            entry_point: Some("fs_main"),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
                                blend: None,
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        primitive: wgpu::PrimitiveState::default(),
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState::default(),
                        multiview: None,
                        cache: None,
                    })
            })
            .clone()
    }
}
