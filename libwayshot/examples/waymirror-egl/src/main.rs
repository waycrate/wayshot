mod dispatch;
mod error;
mod state;
mod utils;

use error::Result;
use state::WaylandEGLState;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(std::io::stderr)
        .init();

    let mut state = WaylandEGLState::new()?;
    let mut event_queue = state.wl_connection.new_event_queue();

    let queue_handle = event_queue.handle();
    let _registry = state.wl_display.get_registry(&queue_handle, ());

    event_queue.roundtrip(&mut state)?;
    state.validate_globals()?;

    state.wl_surface = Some(
        state
            .wl_compositor
            .as_ref()
            .unwrap()
            .create_surface(&queue_handle, ()),
    );

    state.xdg_surface = Some(state.xdg_wm_base.clone().unwrap().get_xdg_surface(
        &state.wl_surface.clone().unwrap(),
        &queue_handle,
        (),
    ));
    state.xdg_toplevel = Some(
        state
            .xdg_surface
            .clone()
            .unwrap()
            .get_toplevel(&queue_handle, ()),
    );
    state
        .xdg_toplevel
        .clone()
        .unwrap()
        .set_title(state.title.clone());
    state.wl_surface.clone().unwrap().commit();

    state.init_egl()?;
    while state.running {
        event_queue.dispatch_pending(&mut state)?;
        // event_queue.blocking_dispatch(&mut state)?;
        state.draw();
        state
            .egl
            .swap_buffers(state.egl_display.unwrap(), state.egl_surface.unwrap())?;

        //tracing::event!(tracing::Level::DEBUG, "eglSwapBuffers called");
    }
    state.deinit()?;

    Ok(())
}
