use std::time::{Duration, Instant};
use std::sync::mpsc;
use thiserror::Error;

pub mod font;


#[derive(Debug, Error)]
enum Error {
    #[error("Font error: {0}")]
    FontError(#[from] font::Error),
    #[error("Windowing error: {0}")]
    WindowingError(#[from] winit::error::OsError),
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Failed: {:?}", e);
    }
}

fn run() -> Result<(), Error> {
    eprintln!("Loading fonts...");
    let path_arg = std::env::args().skip(1).next().unwrap_or("./resources/linja-pona-4.1.otf".to_string());
    let path = std::path::Path::new(&path_arg);
    let mut fontstack = font::FontStack::new(path)?;
    fontstack.add_fallback(std::path::Path::new("/System/Library/Fonts/Helvetica.ttc"))?;
    fontstack.add_fallback(std::path::Path::new("/System/Library/Fonts/Apple Color Emoji.ttc"))?;
    eprintln!("Loaded fonts");

    // let text = "pona mute tawa sina Σ 🇵🇱 mjau 🐔🐔 👉👈 ☝🏾 ☝🏽<=> mjau";
    // debug_font_text(&fontstack, text.to_string());

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(600, 400))
        .with_resizable(true)
        .with_title("rakoune :3")
        .build(&event_loop)?;

    // FPS monitoring
    let (frame_tx, frame_rx) = mpsc::channel::<Duration>();
    std::thread::spawn(move || {
        let mut last_print = Instant::now();
        let mut frame_times: Vec<Duration> = Vec::new();
        loop {
            let frame_time = match frame_rx.recv() {
                Ok(t) => t,
                Err(mpsc::RecvError) => { // Sender disconnected
                    eprintln!("FPS monitor: channel died");
                    return;
                }
            };
            frame_times.push(frame_time);
            if last_print.elapsed() > Duration::from_secs(1) {
                last_print = Instant::now();
                let average_frame_time = frame_times.iter().map(|x| x.as_secs_f64()).sum::<f64>() / frame_times.len() as f64;

                eprintln!("Rendering at {} FPS. Average frame took {:.4}ms to render.", frame_times.len(), average_frame_time * 1000.);
                frame_times.drain(..);
            }
        }
    });

    let mut has_warned_about_channel_died = false;
    event_loop.run(move |evt, _target, ctrl| {
        use winit::event::{Event, WindowEvent};
        match evt {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                eprintln!("bye");
                *ctrl = winit::event_loop::ControlFlow::ExitWithCode(0);
            }
            Event::RedrawRequested(_) => {
                let start = Instant::now();
                if let Err(mpsc::SendError(_)) = frame_tx.send(start.elapsed()) {
                    if !has_warned_about_channel_died {
                        eprintln!("Could not send frame time to FPS monitoring thread");
                        has_warned_about_channel_died = true;
                    }
                }

                window.request_redraw();
            }
            _ => {}
        }
    });
}

#[allow(unused)]
fn debug_font_text(fontstack: &font::FontStack, text: String) {
    let face = &fontstack.faces[0];
    println!("got font {face:?}");
    println!("FAMILY_NAME    = {:?}", font::get_name_by_id(&face.ttf_face, font::NAME_ID_FAMILY_NAME));
    println!("SUBFAMILY_NAME = {:?}", font::get_name_by_id(&face.ttf_face, font::NAME_ID_SUBFAMILY_NAME));
    println!("UNIQUE_NAME    = {:?}", font::get_name_by_id(&face.ttf_face, font::NAME_ID_UNIQUE_NAME));
    println!("FULL_NAME      = {:?}", font::get_name_by_id(&face.ttf_face, font::NAME_ID_FULL_NAME));

    println!("got font with {} glyphs", face.n_glyphs);
    let shaped = fontstack.shape(&text);

    for (shaped, byte_range) in shaped {
        let ch = &text[byte_range];
        print!("Shaping character {ch:?}: ");
        if let Some(shape) = shaped {
            println!("Glyph ID {} on face {}. Glyph is named {:?}", shape.glyph, shape.face.name, shape.face.ttf_face.glyph_name(ttf_parser::GlyphId(shape.glyph)));
        } else {
            println!("unknown");
        }
    }
}
