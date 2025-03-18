use crate::state::WaylandEGLState;
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{wl_compositor, wl_registry, wl_surface},
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandEGLState {
    #[tracing::instrument(skip(registry, queue_handle, state), ret, level = "trace")]
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "xdg_wm_base" => {
                    state.xdg_wm_base = Some(registry.bind::<xdg_wm_base::XdgWmBase, _, _>(
                        name,
                        version,
                        queue_handle,
                        (),
                    ));
                }
                "wl_compositor" => {
                    state.wl_compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        version,
                        queue_handle,
                        (),
                    ));
                }
                _ => {}
            }
        }
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

                    state
                        .egl_window
                        .clone()
                        .unwrap()
                        .resize(state.width, state.height, 0, 0);

                    unsafe {
                        gl::Viewport(0, 0, state.width, state.height);
                    }
                    state.wl_surface.clone().unwrap().commit();
                }
            }
            xdg_toplevel::Event::Close {} => {
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
delegate_noop!(WaylandEGLState: wl_compositor::WlCompositor);
