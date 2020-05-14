use std::io::{Result as IOResult, Cursor};

use wgpu::{
    Surface,
    Adapter,
    Device,
    Queue,
    SwapChainDescriptor,
    SwapChain,
    Color,
    RenderPipeline,
    ProgrammableStageDescriptor,
    BlendDescriptor,
    BufferUsage,
    Buffer,
    BindGroupLayoutDescriptor,
    BindGroupLayoutEntry,
    BindGroupDescriptor,
    ShaderStage,
    BindingType,
    BlendFactor,
    BlendOperation,
    Binding, BindingResource,
    TextureUsage, TextureFormat,
    AddressMode, FilterMode,
};

use winit::{
    dpi::PhysicalSize,
    window::Window,
};

use image::GenericImageView;

use crate::into_ioerror;
use crate::gpu_primitives::Vertex;
use crate::state::State;

pub struct RenderState {
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,

    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,

    logo_render_pipeline: RenderPipeline,
    screen_size_buffer: Buffer,
    logo_bindgroup: wgpu::BindGroup,
}

const VERTEX_SHADER: &[u8] = include_bytes!("../compiled-shaders/shader-vert.spv");
const FRAGMENT_SHADER: &[u8] = include_bytes!("../compiled-shaders/shader-frag.spv");

const LOGO_VERTEX_SHADER: &[u8] = include_bytes!("../compiled-shaders/logo-vert.spv");
const LOGO_FRAGMENT_SHADER: &[u8] = include_bytes!("../compiled-shaders/logo-frag.spv");

const LOGO_IMAGE_PNG: &[u8] = include_bytes!("../resources/rakoune_logo.png");

impl RenderState {
    pub async fn new(window: &Window) -> IOResult<RenderState> {
        let size = window.inner_size();
        let surface = Surface::create(window);

        let adapter = Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        ).await.ok_or(into_ioerror("No adapter available"))?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                extensions: Default::default(),
                limits: Default::default(),
            }
        ).await;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: TextureUsage::OUTPUT_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let vs_data = wgpu::read_spirv(Cursor::new(VERTEX_SHADER)).map_err(into_ioerror)?;
        let fs_data = wgpu::read_spirv(Cursor::new(FRAGMENT_SHADER)).map_err(into_ioerror)?;

        let vs_module = device.create_shader_module(&vs_data);
        let fs_module = device.create_shader_module(&fs_data);

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[],
            },
        );

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &render_pipeline_layout,
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
                        format: sc_desc.format,
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

        let vertex_buffer = device.create_buffer_with_data(
            &[0; 1024],
            BufferUsage::VERTEX | BufferUsage::COPY_DST,
        );

        // Load logo image
        let gen_image = image::load_from_memory_with_format(LOGO_IMAGE_PNG, image::ImageFormat::Png)
            .map_err(into_ioerror)?;

        let (logo_width, logo_height) = gen_image.dimensions();
        let image_data: Vec<u8> = gen_image
            .to_rgba()
            .into_vec();

        debug_assert_eq!(image_data.len(), (logo_width * logo_height * 4) as usize);

        let logo_texture_size = wgpu::Extent3d {
            width: logo_width,
            height: logo_height,
            depth: 1,
        };

        let logo_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Logo image"),
                size: logo_texture_size,
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsage::COPY_DST | TextureUsage::SAMPLED,
            },
        );

        // Copy logo data into logo texture
        let logo_buffer = device.create_buffer_with_data(
            &image_data,
            BufferUsage::COPY_SRC,
        );

        let mut logo_upload_encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Logo uploader"),
            }
        );

        logo_upload_encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &logo_buffer,
                offset: 0,
                bytes_per_row: 4 * logo_width,
                rows_per_image: logo_height,
            },
            wgpu::TextureCopyView {
                texture: &logo_texture,
                mip_level: 0,
                array_layer: 0,
                origin: Default::default(),
            },
            logo_texture_size,
        );

        queue.submit(&[logo_upload_encoder.finish()]);

        let logo_texture_view = logo_texture.create_default_view();

        let logo_sampler = device.create_sampler(
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

        let logo_vs_data = wgpu::read_spirv(Cursor::new(LOGO_VERTEX_SHADER)).map_err(into_ioerror)?;
        let logo_fs_data = wgpu::read_spirv(Cursor::new(LOGO_FRAGMENT_SHADER)).map_err(into_ioerror)?;

        let logo_vs_module = device.create_shader_module(&logo_vs_data);
        let logo_fs_module = device.create_shader_module(&logo_fs_data);

        let screen_size_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&[sc_desc.width, sc_desc.height]),
            BufferUsage::UNIFORM | BufferUsage::COPY_DST,
        );

        let logo_bindgroup_layout_desc = BindGroupLayoutDescriptor {
            label: Some("Logo bindgroup"),
            bindings: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX,
                    ty: BindingType::UniformBuffer { dynamic: false },
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Uint,
                        multisampled: false,
                    },
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler { comparison: false },
                },
            ],
        };

        let logo_bindgroup_layout = device.create_bind_group_layout(&logo_bindgroup_layout_desc);

        let logo_bindgroup_desc = BindGroupDescriptor {
            label: Some("logo bindgroup"),
            layout: &logo_bindgroup_layout,
            bindings: &[
                Binding {
                    binding: 0,
                    resource: BindingResource::Buffer {
                        buffer: &screen_size_buffer,
                        range: 0..(2 * std::mem::size_of::<u32>()) as u64,
                    },
                },
                Binding {
                    binding: 1,
                    resource: BindingResource::TextureView(&logo_texture_view),
                },
                Binding {
                    binding: 2,
                    resource: BindingResource::Sampler(&logo_sampler),
                },
            ],
        };

        let logo_bindgroup = device.create_bind_group(&logo_bindgroup_desc);

        let logo_render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &logo_bindgroup_layout,
                ],
            },
        );

        let logo_render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &logo_render_pipeline_layout,
                vertex_stage: ProgrammableStageDescriptor {
                    module: &logo_vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(ProgrammableStageDescriptor {
                    module: &logo_fs_module,
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
                        format: sc_desc.format,
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
                    ],
                },
                depth_stencil_state: None,
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        Ok(Self {
            surface, adapter, device, queue, sc_desc, swap_chain, render_pipeline, vertex_buffer, logo_render_pipeline, screen_size_buffer, logo_bindgroup,
        })
    }

    pub fn resize(&mut self, into_size: PhysicalSize<u32>) {
        eprintln!("Recreating swapchain!");
        self.sc_desc.width = into_size.width;
        self.sc_desc.height = into_size.height;

        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        let staging_screen_size_mapped = self.device.create_buffer_mapped(
            &wgpu::BufferDescriptor {
                label: Some("Staging screen size buffer"),
                size: (2 * std::mem::size_of::<u32>()) as u64,
                usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC | BufferUsage::STORAGE,
            }
        );
        staging_screen_size_mapped.data.copy_from_slice(
            bytemuck::cast_slice(&[self.sc_desc.width, self.sc_desc.height]),
        );
        let staging_screen_size_buffer = staging_screen_size_mapped.finish();

        let mut stage_upload_encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Staging upload encoder"),
            }
        );

        stage_upload_encoder.copy_buffer_to_buffer(
            &staging_screen_size_buffer,
            0,
            &self.screen_size_buffer,
            0,
            (2 * std::mem::size_of::<u32>()) as u64,
        );

        self.queue.submit(&[stage_upload_encoder.finish()]);
    }

    pub async fn render(&mut self, state: &State) -> IOResult<()> {
        // Upload vertex buffer
        let vertex_buffer_content: &[u8] = bytemuck::cast_slice(&state.verticies);

        // See https://github.com/gfx-rs/wgpu-rs/issues/9#issuecomment-494022784
        // This is a very cheap action since the backing memory is already allocated
        let staging_buffer_mapped = self.device.create_buffer_mapped(
            &wgpu::BufferDescriptor {
                label: Some("Staging buffer"),
                size: 1024,
                usage: BufferUsage::MAP_WRITE | BufferUsage::COPY_SRC | BufferUsage::STORAGE,
            }
        );
        staging_buffer_mapped.data[..vertex_buffer_content.len()].copy_from_slice(vertex_buffer_content);
        let staging_buffer = staging_buffer_mapped.finish();

        let mut stage_upload_encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Staging upload encoder"),
            }
        );

        stage_upload_encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &self.vertex_buffer,
            0,
            1024,
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            }
        );

        let current_texture_view = &self.swap_chain.get_next_texture().map_err(|_| into_ioerror("Timeout"))?.view;

        let mut render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: current_texture_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: Color::BLUE,
                    }
                ],
                depth_stencil_attachment: None,
            }
        );

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, &self.vertex_buffer, 0, 1024);
        render_pass.draw(0..6, 0..1);

        std::mem::drop(render_pass);

        let mut logo_render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: current_texture_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Load,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: Color::RED,
                    }
                ],
                depth_stencil_attachment: None,
            }
        );

        logo_render_pass.set_pipeline(&self.logo_render_pipeline);
        logo_render_pass.set_bind_group(0, &self.logo_bindgroup, &[]);
        logo_render_pass.draw(0..6, 0..1);

        std::mem::drop(logo_render_pass);

        self.queue.submit(&[stage_upload_encoder.finish(), encoder.finish()]);

        Ok(())
    }
}
