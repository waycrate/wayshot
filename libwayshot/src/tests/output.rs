use crate::output::OutputInfo;
use crate::region::{LogicalRegion, Position, Region, Size};
use std::mem;
use std::os::unix::net::UnixStream;
use wayland_backend::client::Backend;
use wayland_client::Proxy;
use wayland_client::protocol::wl_output::WlOutput;

fn make_output_info(
    name: &str,
    description: &str,
    physical_size: Size,
    logical_region: LogicalRegion,
) -> OutputInfo {
    OutputInfo {
        wl_output: dummy_wl_output(),
        name: name.to_string(),
        description: description.to_string(),
        transform: wayland_client::protocol::wl_output::Transform::Normal,
        physical_size,
        logical_region,
    }
}

fn dummy_wl_output() -> WlOutput {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    let weak = backend.downgrade();
    Box::leak(Box::new(backend));
    WlOutput::inert(weak)
}

#[test]
fn display_formats_name_and_description() {
    let output_info = make_output_info(
        "HDMI-A-1",
        "Main Display",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    assert_eq!(output_info.to_string(), "HDMI-A-1 (Main Display)");
    mem::forget(output_info);
}

#[test]
fn display_formats_empty_name_and_description() {
    let output_info = make_output_info(
        "",
        "",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    assert_eq!(output_info.to_string(), " ()");
    mem::forget(output_info);
}

#[test]
fn scale_returns_ratio_between_physical_and_logical_heights() {
    let output_info = make_output_info(
        "DP-1",
        "Secondary Display",
        Size {
            width: 3840,
            height: 2160,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    assert_eq!(output_info.scale(), 2.0);
    mem::forget(output_info);
}

#[test]
fn scale_returns_correct_ratio_for_various_sizes() {
    let o1 = make_output_info(
        "eDP-1",
        "Laptop Screen",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    assert_eq!(o1.scale(), 1.0);
    mem::forget(o1);

    let o15 = make_output_info(
        "DP-2",
        "HiDPI Display",
        Size {
            width: 3840,
            height: 2160,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 2560,
                    height: 1440,
                },
            },
        },
    );
    assert_eq!(o15.scale(), 1.5);
    mem::forget(o15);
}

#[test]
fn debug_format() {
    let output_info = make_output_info(
        "HDMI-1",
        "Debug Display",
        Size {
            width: 800,
            height: 600,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 800,
                    height: 600,
                },
            },
        },
    );
    let debug_str = format!("{:?}", output_info);
    assert!(debug_str.contains("OutputInfo"));
    assert!(debug_str.contains("HDMI-1"));
    assert!(debug_str.contains("Debug Display"));
    assert!(debug_str.contains("800"));
    assert!(debug_str.contains("600"));
    mem::forget(output_info);
}

#[test]
fn clone_and_partial_eq() {
    let o1 = make_output_info(
        "HDMI-1",
        "Clone Display",
        Size {
            width: 1024,
            height: 768,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1024,
                    height: 768,
                },
            },
        },
    );
    let o2 = o1.clone();
    assert_eq!(o1, o2);
    assert_eq!(o1.name, o2.name);
    assert_eq!(o1.description, o2.description);
    assert_eq!(o1.physical_size, o2.physical_size);
    assert_eq!(o1.logical_region, o2.logical_region);
    mem::forget(o1);
    mem::forget(o2);
}

#[test]
fn physical_size_returns_physical_size() {
    let output_info = make_output_info(
        "DP-1",
        "Display",
        Size {
            width: 3840,
            height: 2160,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 100, y: 50 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    let phys = output_info.physical_size();
    assert_eq!(phys.width, 3840);
    assert_eq!(phys.height, 2160);
    mem::forget(output_info);
}

#[test]
fn logical_size_returns_logical_size() {
    let output_info = make_output_info(
        "eDP-1",
        "Display",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 960,
                    height: 540,
                },
            },
        },
    );
    let log_size = output_info.logical_size();
    assert_eq!(log_size.width, 960);
    assert_eq!(log_size.height, 540);
    mem::forget(output_info);
}

#[test]
fn logical_position_returns_position() {
    let output_info = make_output_info(
        "HDMI-1",
        "Display",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: -100, y: 200 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    let pos = output_info.logical_position();
    assert_eq!(pos.x, -100);
    assert_eq!(pos.y, 200);
    mem::forget(output_info);
}

#[test]
fn wayshot_target_from_output_info() {
    use crate::WayshotTarget;
    let output_info = make_output_info(
        "HDMI-1",
        "Display",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    let target = WayshotTarget::from(output_info);
    match &target {
        crate::WayshotTarget::Screen(wl_output) => {
            let _ = wl_output.version();
        }
        crate::WayshotTarget::Toplevel(_) => panic!("Expected Screen variant"),
    }
    mem::forget(target);
}

#[test]
fn output_info_as_ref_returns_wl_output() {
    let output_info = make_output_info(
        "HDMI-1",
        "Display",
        Size {
            width: 1920,
            height: 1080,
        },
        LogicalRegion {
            inner: Region {
                position: Position { x: 0, y: 0 },
                size: Size {
                    width: 1920,
                    height: 1080,
                },
            },
        },
    );
    let wl_ref = output_info.as_ref();
    assert!(std::ptr::eq(wl_ref, &output_info.wl_output));
    mem::forget(output_info);
}
