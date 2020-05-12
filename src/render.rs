use std::io::Result as IOResult;

use wgpu::{
    Surface,
    Adapter,
    Device,
    Queue,
    SwapChainDescriptor,
    SwapChain,
    Color,
};

use winit::{
    dpi::PhysicalSize,
    window::Window,
};

use crate::into_ioerror;

pub struct RenderState {
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,
}

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
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        Ok(Self {
            surface, adapter, device, queue, sc_desc, swap_chain,
        })
    }

    pub fn resize(&mut self, into_size: PhysicalSize<u32>) {
        eprintln!("Recreating swapchain!");
        self.sc_desc.width = into_size.width;
        self.sc_desc.height = into_size.height;

        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn render(&mut self) -> IOResult<()> {
        let current_texture_view = &self.swap_chain.get_next_texture().map_err(|_| into_ioerror("Timeout"))?.view;

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            }
        );

        let render_pass = encoder.begin_render_pass(
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

        std::mem::drop(render_pass);

        self.queue.submit(&[encoder.finish()]);

        Ok(())
    }
}
