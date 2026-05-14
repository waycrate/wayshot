use crate::error::Error;
use crate::output::OutputInfo;
use crate::region::{EmbeddedRegion, LogicalRegion, Position, Region, Size, TopLevel};
use std::mem;
use std::os::unix::net::UnixStream;
use wayland_backend::client::Backend;
use wayland_client::Proxy;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

fn make_output(name: &str, position: Position, size: Size) -> OutputInfo {
    OutputInfo {
        wl_output: dummy_wl_output(),
        name: name.to_string(),
        description: format!("{name} description"),
        transform: wayland_client::protocol::wl_output::Transform::Normal,
        physical_size: size,
        logical_region: LogicalRegion {
            inner: Region { position, size },
        },
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

fn dummy_toplevel_handle() -> ExtForeignToplevelHandleV1 {
    let (client, server) = UnixStream::pair().expect("unix stream");
    Box::leak(Box::new(server));
    let backend = Backend::connect(client).expect("backend");
    let weak = backend.downgrade();
    Box::leak(Box::new(backend));
    ExtForeignToplevelHandleV1::inert(weak)
}

#[test]
fn embedded_region_new_clamps_to_relative_bounds() {
    let viewport = LogicalRegion {
        inner: Region {
            position: Position { x: 5, y: -5 },
            size: Size {
                width: 20,
                height: 20,
            },
        },
    };
    let relative_to = LogicalRegion {
        inner: Region {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 15,
                height: 10,
            },
        },
    };
    let embedded = EmbeddedRegion::new(viewport, relative_to).expect("should be clamped");
    assert_eq!(
        embedded.inner,
        Region {
            position: Position { x: 5, y: 0 },
            size: Size {
                width: 10,
                height: 10
            }
        }
    );
}

#[test]
fn embedded_region_new_returns_none_when_outside() {
    let viewport = LogicalRegion {
        inner: Region {
            position: Position { x: 20, y: 20 },
            size: Size {
                width: 5,
                height: 5,
            },
        },
    };
    let relative_to = LogicalRegion {
        inner: Region {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 10,
                height: 10,
            },
        },
    };
    assert!(EmbeddedRegion::new(viewport, relative_to).is_none());
}

#[test]
fn embedded_region_logical_restores_absolute_coordinates() {
    let relative_to = LogicalRegion {
        inner: Region {
            position: Position { x: 10, y: 15 },
            size: Size {
                width: 100,
                height: 100,
            },
        },
    };
    let embedded = EmbeddedRegion {
        relative_to,
        inner: Region {
            position: Position { x: 5, y: 5 },
            size: Size {
                width: 20,
                height: 30,
            },
        },
    };
    let logical = embedded.logical();
    assert_eq!(
        logical,
        LogicalRegion {
            inner: Region {
                position: Position { x: 15, y: 20 },
                size: Size {
                    width: 20,
                    height: 30
                }
            }
        }
    );
}

#[test]
fn display_formatters_match_expected_layout() {
    let position = Position { x: -5, y: 10 };
    let size = Size {
        width: 42,
        height: 24,
    };
    let region = Region { position, size };
    let logical = LogicalRegion { inner: region };
    let embedded = EmbeddedRegion {
        relative_to: logical,
        inner: region,
    };
    assert_eq!(position.to_string(), "(-5, 10)");
    assert_eq!(size.to_string(), "(42x24)");
    assert_eq!(region.to_string(), "(-5, 10) (42x24)");
    assert_eq!(logical.to_string(), "(-5, 10) (42x24)");
    assert_eq!(
        embedded.to_string(),
        "(-5, 10) (42x24) relative to (-5, 10) (42x24)"
    );
}

#[test]
fn logical_region_from_output_copies_inner_region() {
    let output = make_output(
        "primary",
        Position { x: 100, y: 50 },
        Size {
            width: 1920,
            height: 1080,
        },
    );
    let logical = LogicalRegion::from(&output);
    assert_eq!(logical.inner.position.x, 100);
    assert_eq!(logical.inner.position.y, 50);
    assert_eq!(logical.inner.size.width, 1920);
    assert_eq!(logical.inner.size.height, 1080);
    mem::forget(output);
}

#[test]
fn logical_region_try_from_outputs_spans_all_outputs() {
    let mut outputs = vec![
        make_output(
            "A",
            Position { x: 0, y: 0 },
            Size {
                width: 1920,
                height: 1080,
            },
        ),
        make_output(
            "B",
            Position { x: 1920, y: -100 },
            Size {
                width: 1280,
                height: 1024,
            },
        ),
    ];
    let logical = LogicalRegion::try_from(outputs.as_slice()).expect("valid slice");
    assert_eq!(logical.inner.position.x, 0);
    assert_eq!(logical.inner.position.y, -100);
    assert_eq!(logical.inner.size.width, 1920 + 1280);
    assert_eq!(logical.inner.size.height, 1180);
    for output in outputs.drain(..) {
        mem::forget(output);
    }
}

#[test]
fn logical_region_try_from_empty_slice_errors() {
    let empty: [OutputInfo; 0] = [];
    let err = LogicalRegion::try_from(empty.as_slice()).unwrap_err();
    match err {
        Error::NoOutputs => {}
        _ => panic!("expected Error::NoOutputs"),
    }
}

#[test]
fn position_size_region_logical_region_default() {
    let pos = Position::default();
    assert_eq!(pos.x, 0);
    assert_eq!(pos.y, 0);

    let size = Size::default();
    assert_eq!(size.width, 0);
    assert_eq!(size.height, 0);

    let region = Region::default();
    assert_eq!(region.position, pos);
    assert_eq!(region.size, size);

    let logical = LogicalRegion::default();
    assert_eq!(logical.inner, region);
}

#[test]
fn embedded_region_exact_fit() {
    let viewport = LogicalRegion {
        inner: Region {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 100,
                height: 50,
            },
        },
    };
    let relative_to = LogicalRegion {
        inner: Region {
            position: Position { x: 10, y: 20 },
            size: Size {
                width: 100,
                height: 50,
            },
        },
    };
    let embedded = EmbeddedRegion::new(viewport, relative_to).expect("exact fit");
    assert_eq!(embedded.inner.position.x, 0);
    assert_eq!(embedded.inner.position.y, 0);
    assert_eq!(embedded.inner.size.width, 100);
    assert_eq!(embedded.inner.size.height, 50);
}

#[test]
fn embedded_region_fully_inside() {
    let viewport = LogicalRegion {
        inner: Region {
            position: Position { x: 5, y: 5 },
            size: Size {
                width: 10,
                height: 10,
            },
        },
    };
    let relative_to = LogicalRegion {
        inner: Region {
            position: Position { x: 0, y: 0 },
            size: Size {
                width: 100,
                height: 100,
            },
        },
    };
    let embedded = EmbeddedRegion::new(viewport, relative_to).expect("fully inside");
    assert_eq!(embedded.inner.position.x, 5);
    assert_eq!(embedded.inner.position.y, 5);
    assert_eq!(embedded.inner.size.width, 10);
    assert_eq!(embedded.inner.size.height, 10);
}

#[test]
fn logical_region_from_output_ref() {
    let output = make_output(
        "primary",
        Position { x: 42, y: 43 },
        Size {
            width: 1920,
            height: 1080,
        },
    );
    let logical = LogicalRegion::from(&output);
    assert_eq!(logical.inner.position.x, 42);
    assert_eq!(logical.inner.position.y, 43);
    assert_eq!(logical.inner.size.width, 1920);
    assert_eq!(logical.inner.size.height, 1080);
    mem::forget(output);
}

#[test]
fn toplevel_id_and_title_formats_app_id_and_title() {
    let handle = dummy_toplevel_handle();
    let mut toplevel = TopLevel::new(handle);
    toplevel.app_id = "com.example.myapp".to_string();
    toplevel.title = "My App Window".to_string();
    assert_eq!(toplevel.id_and_title(), "com.example.myapp My App Window");
}

#[test]
fn toplevel_new_has_empty_fields_and_active_true() {
    let handle = dummy_toplevel_handle();
    let toplevel = TopLevel::new(handle);
    assert!(toplevel.title.is_empty());
    assert!(toplevel.app_id.is_empty());
    assert!(toplevel.identifier.is_empty());
    assert!(toplevel.active);
}

#[test]
fn toplevel_id_and_title_empty_fields() {
    let handle = dummy_toplevel_handle();
    let toplevel = TopLevel::new(handle);
    assert_eq!(toplevel.id_and_title(), " ");
}

#[test]
fn toplevel_as_ref_returns_handle() {
    let handle = dummy_toplevel_handle();
    let toplevel = TopLevel::new(handle);
    let handle_ref: &ExtForeignToplevelHandleV1 = toplevel.as_ref();
    assert!(std::ptr::eq(handle_ref, &toplevel.handle));
    mem::forget(toplevel);
}
