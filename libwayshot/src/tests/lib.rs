use crate::WayshotTarget;
use crate::region::TopLevel;
use std::mem;
use std::os::unix::net::UnixStream;
use wayland_backend::client::Backend;
use wayland_client::Proxy;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

fn inert_wl_output() -> WlOutput {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    let weak = backend.downgrade();
    Box::leak(Box::new(backend));
    WlOutput::inert(weak)
}

fn inert_toplevel_handle() -> ExtForeignToplevelHandleV1 {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    let weak = backend.downgrade();
    Box::leak(Box::new(backend));
    ExtForeignToplevelHandleV1::inert(weak)
}

#[test]
fn wayshot_target_screen_is_not_alive_when_inert() {
    let output = inert_wl_output();
    let target = WayshotTarget::Screen(output);
    // Inert objects are not alive
    assert!(!target.is_alive());
    mem::forget(target);
}

#[test]
fn wayshot_target_toplevel_is_not_alive_when_inert() {
    let handle = inert_toplevel_handle();
    let target = WayshotTarget::Toplevel(handle);
    assert!(!target.is_alive());
    mem::forget(target);
}

#[test]
fn wayshot_target_from_toplevel() {
    let handle = inert_toplevel_handle();
    let toplevel = TopLevel::new(handle);
    let target = WayshotTarget::from(toplevel);
    match target {
        WayshotTarget::Toplevel(_) => {}
        _ => panic!("expected WayshotTarget::Toplevel"),
    }
    mem::forget(target);
}
