use std::io::{Result as IOResult, Cursor};

use wgpu::{
    Surface,
    Adapter,
    Device,
    Queue,
    SwapChainDescriptor,
    SwapChain,
    Color,
    Texture, TextureUsage, TextureFormat,
    Extent3d,
};

use winit::{
    dpi::PhysicalSize,
    window::Window,
};

use crate::into_ioerror;
use crate::state::State;

mod logo;
use logo::LogoRenderer;

mod text;
use text::TextRenderer;

struct RichTexture {
    content: Texture,
    format: TextureFormat,
    extent: Extent3d,
}

impl RichTexture {
    fn new(backend: &mut RenderBackend, format: TextureFormat, extent: Extent3d, label: Option<&str>) -> IOResult<Self> {
        Self::new_with_usage(backend, format, extent, label, TextureUsage::COPY_DST | TextureUsage::SAMPLED)
    }

    fn new_with_usage(backend: &mut RenderBackend, format: TextureFormat, extent: Extent3d, label: Option<&str>, usage: TextureUsage) -> IOResult<Self> {
        let content = backend.device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size: extent,
                array_layer_count: 1,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: format,
                usage,
            },
        );

        Ok(Self {
            content, format, extent,
        })
    }
}

impl std::ops::Deref for RichTexture {
    type Target = Texture;
    fn deref(&self) -> &Texture {
        &self.content
    }
}

struct RenderBackend {
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,
}

impl RenderBackend {
    async fn new(window: &Window) -> IOResult<Self> {
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

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
        })
    }

    fn load_shader_mod(&mut self, shader_data: &[u8]) -> IOResult<wgpu::ShaderModule> {
        let parsed_data = wgpu::read_spirv(Cursor::new(shader_data)).map_err(into_ioerror)?;

        Ok(self.device.create_shader_module(&parsed_data))
    }

    fn recreate_swapchain(&mut self, into_size: PhysicalSize<u32>) -> IOResult<()> {
        eprintln!("Recreating swapchain!");
        self.sc_desc.width = into_size.width;
        self.sc_desc.height = into_size.height;

        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

        Ok(())
    }

}

pub struct RenderState {
    backend: RenderBackend,
    logo_renderer: LogoRenderer,
    text_renderer: TextRenderer,
}

impl RenderState {
    pub async fn new(window: &Window) -> IOResult<RenderState> {
        let mut backend = RenderBackend::new(window).await?;

        let logo_renderer = LogoRenderer::new(&mut backend).await?;
        let text_renderer = TextRenderer::new(&mut backend).await?;

        Ok(Self {
            backend,
            logo_renderer,
            text_renderer,
        })
    }

    pub fn resize(&mut self, into_size: PhysicalSize<u32>) -> IOResult<()> {
        self.backend.recreate_swapchain(into_size)?;

        self.logo_renderer.resize(&mut self.backend, into_size)?;
        self.text_renderer.resize(&mut self.backend, into_size)?;

        Ok(())
    }

    pub async fn render(&mut self, state: &State) -> IOResult<()> {
        let current_texture_view = &self.backend.swap_chain.get_next_texture().map_err(|_| into_ioerror("Timeout"))?.view;

        let mut encoder = self.backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            }
        );

        let clear_render_pass = encoder.begin_render_pass(
            &wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: current_texture_view,
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: Color::BLACK,
                    }
                ],
                depth_stencil_attachment: None,
            }
        );
        std::mem::drop(clear_render_pass);

        let clear_screen = encoder.finish();

        let logo_render = self.logo_renderer.render(&mut self.backend, &current_texture_view, state).await?;
        let text_render = self.text_renderer.render(&mut self.backend, &current_texture_view, state).await?;

        self.backend.queue.submit(&[clear_screen, logo_render, text_render]);

        Ok(())
    }
}
