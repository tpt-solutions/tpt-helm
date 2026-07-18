// SPDX-License-Identifier: MIT OR Apache-2.0

//! `wgpu` GPU backend for chart rendering.
//!
//! Uploads a tessellated [`Frame`] to the GPU and draws it. This module is only
//! compiled with the `gpu` feature so that CI and embedded targets without a
//! graphics adapter can still build and test the headless pipeline.

use crate::render::tessellate::{Command, Frame, Primitive, Vertex};

/// WGSL shader drawing flat-colored 2D vertices.
const SHADER: &str = r#"
struct Uniforms {
    resolution: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    let clip = vec2<f32>(
        position.x / u.resolution.x * 2.0 - 1.0,
        1.0 - position.y / u.resolution.y * 2.0,
    );
    out.pos = vec4<f32>(clip, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// A GPU-backed chart renderer.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: Pipelines,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    max_vertices: u32,
}

struct Pipelines {
    triangle: wgpu::RenderPipeline,
    line: wgpu::RenderPipeline,
    point: wgpu::RenderPipeline,
}

impl GpuRenderer {
    /// Initialize the renderer against the first available GPU adapter.
    ///
    /// # Errors
    /// Returns a string if no adapter/device is available or shader compilation
    /// fails. Callers on headless CI should fall back to the CPU tessellator.
    pub async fn new(width: u32, height: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or_else(|| "no GPU adapter".to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| e.to_string())?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chart-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let mk_pipeline = |topology: wgpu::PrimitiveTopology| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chart-pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs",
                    buffers: &[Vertex::buffer_layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        let pipelines = Pipelines {
            triangle: mk_pipeline(wgpu::PrimitiveTopology::TriangleList),
            line: mk_pipeline(wgpu::PrimitiveTopology::LineList),
            point: mk_pipeline(wgpu::PrimitiveTopology::PointList),
        };

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[width as f32, height as f32]),
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind-group"),
            layout: &pipelines.triangle.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let max_vertices = 1 << 20;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex-buffer"),
            size: u64::from(max_vertices) * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            pipelines,
            bind_group,
            vertex_buffer,
            max_vertices,
        })
    }

    /// Upload a frame's vertices.
    ///
    /// # Errors
    /// Returns an error string if the frame exceeds the preallocated buffer.
    pub fn upload(&mut self, frame: &Frame) -> Result<(), String> {
        if frame.vertices.len() > self.max_vertices as usize {
            return Err("frame exceeds vertex buffer capacity".into());
        }
        let bytes: Vec<u8> = frame.vertices.iter().flat_map(Vertex::to_bytes).collect();
        self.queue.write_buffer(&self.vertex_buffer, 0, &bytes);
        Ok(())
    }

    /// Record draw commands into an encoder targeting `view`.
    pub fn encode(
        &self,
        commands: &[Command],
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chart-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        for cmd in commands {
            let (pipeline, topology) = match cmd.primitive {
                Primitive::TriangleList => (
                    &self.pipelines.triangle,
                    wgpu::PrimitiveTopology::TriangleList,
                ),
                Primitive::LineList => (&self.pipelines.line, wgpu::PrimitiveTopology::LineList),
                Primitive::PointList => (&self.pipelines.point, wgpu::PrimitiveTopology::PointList),
            };
            let _ = topology;
            pass.set_pipeline(pipeline);
            pass.draw(cmd.start..cmd.start + cmd.count, 0..1);
        }
    }

    /// Access the underlying device (used by callers to create the surface view).
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
}
