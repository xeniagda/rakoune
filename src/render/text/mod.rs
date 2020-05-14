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
};

use winit::dpi::PhysicalSize;

use image::GenericImageView;

use super::RenderBackend;
use crate::into_ioerror;
use crate::state::State;

mod text_gpu_primitives;
use text_gpu_primitives::Vertex;

const VS_DATA: &[u8] = include_bytes!("../../../compiled-shaders/text-vert.spv");
const FS_DATA: &[u8] = include_bytes!("../../../compiled-shaders/text-frag.spv");

const FONT_SIZE: f32 = 40.0;
const FONT_DATA: &[u8] = include_bytes!("../../../resources/linja-pona-4.1.otf");

pub(super) struct TextRenderer {
    render_pipeline: RenderPipeline,
    bind_group: wgpu::BindGroup,

    glyph_canvas: wgpu::Texture,
    glyph_vertex_buffer: wgpu::Buffer,
}

impl TextRenderer {
    pub async fn new(backend: &mut RenderBackend) -> IOResult<Self> {
        let canvas_size = wgpu::Extent3d {
            width: 512,
            height: 512,
            depth: 1,
        };
        let glyph_canvas = backend.device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Glyph image"),
                size: canvas_size,
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::COPY_DST | TextureUsage::SAMPLED,
            },
        );
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
                size: 1024, // For now
                usage: BufferUsage::COPY_DST | BufferUsage::VERTEX,
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

        Ok(Self {
            render_pipeline,
            bind_group,
            glyph_canvas,
            glyph_vertex_buffer,
        })
    }

    pub fn resize(&mut self, backend: &mut RenderBackend, into_size: PhysicalSize<u32>) -> IOResult<()> {
        Ok(())
    }

    pub fn render(&mut self, backend: &mut RenderBackend, to_view: &wgpu::TextureView, state: &State) -> IOResult<wgpu::CommandBuffer> {
        let mut encoder = backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Text render encoder"),
            }
        );

        let mut logo_render_pass = encoder.begin_render_pass(
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

        logo_render_pass.set_pipeline(&self.render_pipeline);
        logo_render_pass.set_bind_group(0, &self.bind_group, &[]);
        logo_render_pass.set_vertex_buffer(0, &self.glyph_vertex_buffer, 0, 1024);
        logo_render_pass.draw(0..6, 0..1);

        std::mem::drop(logo_render_pass);

        Ok(encoder.finish())
    }
}

