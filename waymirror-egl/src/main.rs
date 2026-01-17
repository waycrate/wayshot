mod dispatch;
mod error;
mod state;
mod utils;

use std::time::{Duration, Instant};

use error::Result;
use r_egl_wayland::EGL_INSTALCE;
use state::WaylandEGLState;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    let (mut state, mut event_queue) = WaylandEGLState::new()?;

    state.init_program()?;

    println!("Starting the example EGL-enabled wayshot dmabuf demo app, press <ESC> to quit.");

    while state.running {
        let _ = event_queue.roundtrip(&mut state);
        if state.instant <= Instant::now() {
            state.instant = Instant::now()
                .checked_add(Duration::from_millis(10))
                .unwrap();
            state.draw();
            state.cast();
            let _ = EGL_INSTALCE.swap_buffers(state.egl_display, state.egl_surface);
            tracing::trace!("eglSwapBuffers called");
        }
    }
    state.deinit()?;

    Ok(())
}
