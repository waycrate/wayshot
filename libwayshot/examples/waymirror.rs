use std::time::{Duration, Instant};

use libwayshot::{Size, WayshotConnection, screencast::WayshotScreenCast};
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::{self},
        wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
};

use wayland_protocols::{
    wp::viewporter::client::{wp_viewport, wp_viewporter},
    xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

fn main() {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init::<State>(&conn).unwrap();

    let qhandle = event_queue.handle();

    let compositor = globals
        .bind::<wl_compositor::WlCompositor, _, _>(&qhandle, 3..=3, ())
        .unwrap();
    let base_surface = compositor.create_surface(&qhandle, ());

    globals
        .bind::<wl_seat::WlSeat, _, _>(&qhandle, 1..=1, ())
        .unwrap();

    let wm_base = globals
        .bind::<xdg_wm_base::XdgWmBase, _, _>(&qhandle, 2..=6, ())
        .unwrap();
    let xdg_surface = wm_base.get_xdg_surface(&base_surface, &qhandle, ());
    let toplevel = xdg_surface.get_toplevel(&qhandle, ());
    toplevel.set_title("DMABuf+wlr-screencopy example!".into());

    base_surface.commit();

    let mut wayshot = WayshotConnection::from_connection(conn).unwrap();

    use libwayshot::WayshotTarget;
    let output = wayshot.get_all_outputs()[0].wl_output.clone();
    wayshot
        .try_init_dmabuf(WayshotTarget::Screen(output.clone()))
        .expect("Cannot find a drm");
    let cast = wayshot
        .create_screencast_with_dmabuf(WayshotTarget::Screen(output), true, None)
        .unwrap();

    let view_porter = globals
        .bind::<wp_viewporter::WpViewporter, _, _>(&qhandle, 1..=1, ())
        .unwrap();
    let viewport = view_porter.get_viewport(&base_surface, &qhandle, ());

    let mut state = State {
        wayshot,
        running: true,
        base_surface,
        viewport,

        cast_size: libwayshot::Size::default(),
        configured: false,
        cast,
        instant: Instant::now()
            .checked_add(Duration::from_millis(10))
            .unwrap(),
    };

    println!("Starting the example wayshot dmabuf demo app, press <ESC> to quit.");

    while state.running {
        event_queue.roundtrip(&mut state).unwrap();
        if state.instant <= Instant::now() && state.configured {
            state.instant = Instant::now()
                .checked_add(Duration::from_millis(10))
                .unwrap();
            let _ = state.refresh_surface();
        }
    }
}

struct State {
    wayshot: WayshotConnection,
    running: bool,
    base_surface: wl_surface::WlSurface,
    viewport: wp_viewport::WpViewport,
    cast_size: libwayshot::Size<i32>,

    configured: bool,
    cast: WayshotScreenCast,
    instant: Instant,
}

impl State {
    fn refresh_surface(&mut self) -> libwayshot::Result<()> {
        self.wayshot.screencast(&mut self.cast)?;

        self.cast_size = self.cast.current_size();
        self.base_surface.attach(Some(self.cast.buffer()), 0, 0);
        let Size { width, height } = self.cast.current_size();
        self.base_surface.damage(0, 0, width, height);
        self.base_surface.commit();
        Ok(())
    }
    fn viewport_adjust(&self, width: i32, height: i32) {
        if self.cast_size.width == 0 || self.cast_size.height == 0 {
            return;
        }
        let width_fit = (self.cast_size.width as f32) / (width as f32);
        let height_fit = (self.cast_size.height as f32) / (height as f32);
        let fit = width_fit.max(height_fit);
        let new_width = ((self.cast_size.width as f32) / fit) as i32;
        let new_height = ((self.cast_size.height as f32) / fit) as i32;
        self.viewport.set_destination(new_width, new_height);
    }
}
impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut State,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

// Ignore events from these object types in this example.
delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore wp_viewport::WpViewport);
delegate_noop!(State: ignore wp_viewporter::WpViewporter);

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            xdg_surface.ack_configure(serial);
            state.configured = true;
            let _ = state.refresh_surface();
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Close => {
                state.running = false;
            }
            xdg_toplevel::Event::Configure { width, height, .. } => {
                state.viewport_adjust(width, height);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
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

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
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
