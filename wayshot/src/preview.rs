use image::DynamicImage;
use libwayshot::WayshotConnection;
use std::io::Write;
use std::os::fd::AsRawFd;
use wayland_client::{
    globals::registry_queue_init, protocol::*, Connection, Dispatch, QueueHandle, WEnum,
    delegate_noop,
};
use wayland_protocols::xdg::shell::client::*;

struct PreviewState {
    running: bool,
    confirmed: bool,
}

pub fn show_preview(image: &DynamicImage) -> eyre::Result<bool> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<PreviewState>(&conn)?;
    let qh = event_queue.handle();

    let compositor = globals.bind::<wl_compositor::WlCompositor, _, _>(&qh, 3..=3, ())?;
    let shm = globals.bind::<wl_shm::WlShm, _, _>(&qh, 1..=1, ())?;
    let surface = compositor.create_surface(&qh, ());

    globals.bind::<wl_seat::WlSeat, _, _>(&qh, 1..=1, ())?;

    let wm_base = globals.bind::<xdg_wm_base::XdgWmBase, _, _>(&qh, 2..=6, ())?;
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("Screenshot Preview".into());

    surface.commit();

    let mut state = PreviewState {
        running: true,
        confirmed: false,
    };

    event_queue.roundtrip(&mut state)?;

    let width = image.width() as i32;
    let height = image.height() as i32;
    let stride = width * 4;

    let mut file = tempfile::tempfile()?;
    let rgba = image.to_rgba8();
    let mut buffer = vec![0u8; (stride * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let pixel = rgba.get_pixel(x as u32, y as u32);
            let offset = ((y * width + x) * 4) as usize;
            buffer[offset] = pixel[2];
            buffer[offset + 1] = pixel[1];
            buffer[offset + 2] = pixel[0];
            buffer[offset + 3] = pixel[3];
        }
    }
    file.write_all(&buffer)?;

    let pool = shm.create_pool(file.as_raw_fd(), stride * height, &qh, ());
    let wl_buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, &qh, ());

    surface.attach(Some(&wl_buffer), 0, 0);
    surface.damage(0, 0, width, height);
    surface.commit();

    while state.running {
        event_queue.blocking_dispatch(&mut state)?;
    }

    Ok(state.confirmed)
}

impl Dispatch<wl_registry::WlRegistry, ()> for PreviewState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(PreviewState: ignore wl_compositor::WlCompositor);
delegate_noop!(PreviewState: ignore wl_surface::WlSurface);
delegate_noop!(PreviewState: ignore wl_shm::WlShm);
delegate_noop!(PreviewState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(PreviewState: ignore wl_buffer::WlBuffer);

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for PreviewState {
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

impl Dispatch<xdg_surface::XdgSurface, ()> for PreviewState {
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

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for PreviewState {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close = event {
            state.running = false;
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for PreviewState {
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

impl Dispatch<wl_keyboard::WlKeyboard, ()> for PreviewState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            match key {
                1 => state.running = false,              // ESC
                28 => {
                    state.confirmed = true;              // Enter
                    state.running = false;
                }
                _ => {}
            }
        }
    }
}
