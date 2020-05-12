mod render;

use std::io::{Result as IOResult, Error, ErrorKind};
use futures::executor::block_on;
use winit::{
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},

    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState},
};

use render::RenderState;

pub fn into_ioerror<T: ToString>(x: T) -> Error {
    Error::new(
        ErrorKind::Other,
        x.to_string()
    )
}

fn main() -> IOResult<()> {
    let el = EventLoop::new();

    let mut window = WindowBuilder::new()
        .with_title("rakoune")
        .build(&el)
        .map_err(into_ioerror)?;

    let mut render_state = block_on(RenderState::new(&window))?;

    el.run(move |event, _, cf| {
        match event {
            Event::WindowEvent {
                event: w_event, ..
            } => {
                block_on(handle_window_event(w_event, &mut window, cf, &mut render_state));
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                render_state.render().expect("Render failed");
            }
            _ => {}
        }
    });
}

async fn handle_window_event(w_event: WindowEvent<'_>, window: &mut Window, cf: &mut ControlFlow, render_state: &mut RenderState) {
    match w_event {
        WindowEvent::CloseRequested => {
            *cf = ControlFlow::Exit;
        }
        WindowEvent::Resized(phys_size) |
        WindowEvent::ScaleFactorChanged {
            new_inner_size: &mut phys_size,
            ..
        } => {
            render_state.resize(phys_size);
        }
        WindowEvent::KeyboardInput {
            input: KeyboardInput {
                virtual_keycode: Some(kc),
                state: ElementState::Pressed,
                ..
            },
            ..
        } => {
            if kc == VirtualKeyCode::Escape {
                *cf = ControlFlow::Exit;
            } else {
                eprintln!("Pressed {:?}", kc);
            }
        }
        _ => {}
    }
}
