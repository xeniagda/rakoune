use std::io::Result as IOResult;

use wgpu::{
    Buffer,
    Texture,
    RenderPipeline,
    TextureUsage,
    BufferUsage,
    BindGroup,
    BindGroupLayoutEntry,
    ShaderStage,
    BindingType,
    AddressMode,
    FilterMode,
    Binding,
    BindingResource,
    BlendFactor,
    BlendOperation,
    ProgrammableStageDescriptor,
    BlendDescriptor,
    TextureFormat,
    Extent3d,
};

use winit::dpi::PhysicalSize;

use image::GenericImageView;

use super::{RenderBackend, RichTexture};
use crate::into_ioerror;
use crate::state::State;

mod text_gpu_primitives;
use text_gpu_primitives::Vertex;

mod glypher;
use glypher::Glypher;

const VS_DATA: &[u8] = include_bytes!("../../../compiled-shaders/text-vert.spv");
const FS_DATA: &[u8] = include_bytes!("../../../compiled-shaders/text-frag.spv");

pub(super) struct TextRenderer {
    render_pipeline: RenderPipeline,
    bind_group: wgpu::BindGroup,

    glyph_canvas: RichTexture,
    glyph_vertex_buffer: wgpu::Buffer,
    n_verticies: u32,

    glypher: Glypher,
}

impl TextRenderer {
    pub async fn new(backend: &mut RenderBackend) -> IOResult<Self> {
        let glyph_canvas = RichTexture::new(
            backend,
            TextureFormat::Rgba8UnormSrgb,
            Extent3d {
                width: 1024,
                height: 1024,
                depth: 1,
            },
            Some("Glyph canvas"),
        )?;

        let glyph_canvas_view = glyph_canvas.create_default_view();

        let sampler = backend.device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Linear,
                lod_min_clamp: -100.,
                lod_max_clamp: 100.,
                compare: wgpu::CompareFunction::Always,
            },
        );

        let glyph_vertex_buffer = backend.device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Glyph vertex buffer"),
                size: 4096, // For now
                usage: BufferUsage::COPY_DST | BufferUsage::VERTEX | BufferUsage::MAP_WRITE,
            },
        );

        let bind_group_layout = backend.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Text bind group layout"),
                bindings: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::SampledTexture {
                            dimension: wgpu::TextureViewDimension::D2,
                            component_type: wgpu::TextureComponentType::Uint,
                            multisampled: false,
                        },
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStage::FRAGMENT,
                        ty: BindingType::Sampler {
                            comparison: false,
                        },
                    },
                ],
            },
        );

        let bind_group = backend.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Text bind group"),
                layout: &bind_group_layout,
                bindings: &[
                    Binding {
                        binding: 0,
                        resource: BindingResource::TextureView(
                            &glyph_canvas_view,
                        ),
                    },
                    Binding {
                        binding: 1,
                        resource: BindingResource::Sampler(
                            &sampler,
                        ),
                    },
                ],
            },
        );

        let pipeline_layout = backend.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &bind_group_layout,
                ],
            },
        );

        let vs_module = backend.load_shader_mod(VS_DATA)?;
        let fs_module = backend.load_shader_mod(FS_DATA)?;

        let render_pipeline = backend.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
                vertex_stage: ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::Back,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[
                    wgpu::ColorStateDescriptor {
                        format: backend.sc_desc.format,
                        color_blend: BlendDescriptor {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha_blend: BlendDescriptor {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        write_mask: wgpu::ColorWrite::ALL,
                    }
                ],
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint32,
                    vertex_buffers: &[
                        Vertex::desc(),
                    ],
                },
                depth_stencil_state: None,
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        let glypher = Glypher::new()?;

        Ok(Self {
            render_pipeline,
            bind_group,
            glyph_canvas,
            glyph_vertex_buffer,
            n_verticies: 0,
            glypher,
        })
    }

    pub fn resize(&mut self, backend: &mut RenderBackend, into_size: PhysicalSize<u32>) -> IOResult<()> {
        self.glypher.resize(backend, into_size)
    }

    pub async fn write_data(&mut self, backend: &mut RenderBackend, state: &State) -> IOResult<()> {
        if let Some(n) = self
            .glypher
            .upload(
                backend,
                state,
                &mut self.glyph_vertex_buffer,
                &mut self.glyph_canvas,
            )
            .await? {
            self.n_verticies = n;
        }
        Ok(())
    }

    pub async fn render(&mut self, backend: &mut RenderBackend, to_view: &wgpu::TextureView, state: &State) -> IOResult<wgpu::CommandBuffer> {
        self.write_data(backend, state).await?;

        let mut encoder = backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Text render encoder"),
            }
        );

        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: to_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Load,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::WHITE,
                    }
                ],
                depth_stencil_attachment: None,
            }
        );

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, &self.glyph_vertex_buffer, 0, 0);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..self.n_verticies, 0..1);

        std::mem::drop(render_pass);

        Ok(encoder.finish())
    }

    pub fn collect_textures<'a>(&'a self) -> Vec<&'a RichTexture> {
        vec![
            &self.glyph_canvas,
        ]
    }
}

