#![feature(async_closure)]

mod render;
mod state;

use std::io::{Result as IOResult, Error, ErrorKind};
use std::time::{Instant, Duration};

use futures::executor::block_on;

use winit::{
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},

    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState},
};

use render::RenderState;
use state::State;

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

    let mut state = State::new();

    let mut frame_instants: Vec<Instant> = Vec::new();
    let mut frame_durations: Vec<Duration> = Vec::new();
    let mut last_debug_time: Option<Instant> = None;

    el.run(move |event, _, cf| {
        if last_debug_time.map(|ldt| Instant::now() - ldt > Duration::new(2, 0)).unwrap_or(true) {
            // Print some debug info

            let mut n_frames_last_second = 0;
            for &frame_inst in frame_instants.iter().rev() {
                if frame_inst > Instant::now() - Duration::new(1, 0) {
                    n_frames_last_second += 1;
                } else {
                    break;
                }
            }

            eprintln!("FPS: {}", n_frames_last_second);

            let mut mean_frame_durations = Duration::new(0, 0);
            for &frame_dur in &frame_durations {
                mean_frame_durations += frame_dur / frame_durations.len() as u32;
            }
            eprintln!("Average render time: {:?} = {:.3} optimal renders per seconds ", mean_frame_durations, 1. / mean_frame_durations.as_secs_f64());

            frame_instants.clear();
            frame_durations.clear();
            last_debug_time = Some(Instant::now());
        }
        match event {
            Event::WindowEvent {
                event: w_event, ..
            } => {
                block_on(handle_window_event(w_event, &mut window, cf, &mut render_state, &mut state));
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                frame_instants.push(Instant::now());

                state.step(1. / 60.); // Assume 60 FPS
                let render_start = Instant::now();
                block_on(render_state.render(&state)).expect("Render error");
                frame_durations.push(Instant::now() - render_start);
            }
            _ => {}
        }
    });
}

async fn handle_window_event(w_event: WindowEvent<'_>, _window: &mut Window, cf: &mut ControlFlow, render_state: &mut RenderState, state: &mut State) {
    match w_event {
        WindowEvent::CloseRequested => {
            *cf = ControlFlow::Exit;
        }
        WindowEvent::Resized(phys_size) |
        WindowEvent::ScaleFactorChanged {
            new_inner_size: &mut phys_size,
            ..
        } => {
            render_state.resize(phys_size).expect("Window resize failed");
        }
        WindowEvent::KeyboardInput {
            input: KeyboardInput {
                virtual_keycode: Some(VirtualKeyCode::Escape),
                state: ElementState::Pressed,
                ..
            },
            ..
        } => {
            *cf = ControlFlow::Exit;
        }
        WindowEvent::KeyboardInput {
            input: KeyboardInput {
                virtual_keycode: Some(VirtualKeyCode::Back),
                state: ElementState::Pressed,
                ..
            },
            ..
        } => {
            state.content.pop();
        }
        WindowEvent::ReceivedCharacter(ch) => {
            println!("Received {:?}", ch);
            if !ch.is_control() {
                state.content.push(ch);
            }
        }
        _ => {}
    }
}
