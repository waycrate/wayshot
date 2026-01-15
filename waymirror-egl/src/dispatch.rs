use crate::state::WaylandEGLState;
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    globals::GlobalListContents,
    protocol::{wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_surface},
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandEGLState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandEGLState {
    #[tracing::instrument(skip(xdg_wm_base), ret, level = "trace")]
    fn event(
        _: &mut Self,
        xdg_wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            xdg_wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandEGLState {
    #[tracing::instrument(skip(xdg_surface), ret, level = "trace")]
    fn event(
        _: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandEGLState {
    #[tracing::instrument(skip(), ret, level = "trace")]
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width == 0 || height == 0 {
                    return; // We do not respect this configure
                }

                if state.width != width || state.height != height {
                    state.width = width;
                    state.height = height;

                    state.egl_window.resize(state.width, state.height, 0, 0);

                    unsafe {
                        gl::Viewport(0, 0, state.width, state.height);
                    }
                    state.wl_surface.commit();
                }
            }
            xdg_toplevel::Event::Close => {
                state.running = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandEGLState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: <wl_surface::WlSurface as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandEGLState {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
            && capabilities.contains(wl_seat::Capability::Keyboard)
        {
            seat.get_keyboard(qh, ());
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandEGLState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event
            && key == 1
        {
            // ESC key
            state.running = false;
        }
    }
}

delegate_noop!(WaylandEGLState: wl_compositor::WlCompositor);
