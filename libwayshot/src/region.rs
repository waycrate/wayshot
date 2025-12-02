use crate::{
    WayshotConnection,
    error::{Error, Result},
    output::OutputInfo,
};
use std::cmp;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

pub type FreezeCallback = Box<dyn Fn(&WayshotConnection) -> Result<LogicalRegion>>;

/// Ways to say how a region for a screenshot should be captured.
pub enum RegionCapturer {
    /// Capture all of the given outputs.
    Outputs(Vec<OutputInfo>),
    /// Capture an already known `LogicalRegion`.
    Region(LogicalRegion),
    /// Capture a specific toplevel window.
    TopLevel(TopLevel),
    /// The outputs will be "frozen" to the user at which point the given
    /// callback is called to get the region to capture. This callback is often
    /// a user interaction to let the user select a region.
    Freeze(FreezeCallback),
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub handle: ExtForeignToplevelHandleV1,
    pub title: String,
    pub app_id: String,
    pub identifier: String,
    pub active: bool,
}

impl TopLevel {
    pub fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: String::new(),
            app_id: String::new(),
            identifier: String::new(),
            active: true,
        }
    }

    pub fn id_and_title(&self) -> String {
        format!("{} {}", self.app_id, self.title)
    }
}

/// `Region` where the coordinate system is the logical coordinate system used
/// in Wayland to position outputs. Top left is (0, 0) and any transforms and
/// scaling have been applied. A unit is a logical pixel, meaning that this is
/// after scaling has been applied.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LogicalRegion {
    pub inner: Region,
}

/// An embedded region is a region entirely inside of another (often an output).
///
/// It can only be contained inside of another and cannot exceed its bounds.
///
/// Example:
///
/// ````ignore
/// ┌─────────────┐
/// │             │
/// │  ┌──────────┼──────┐
/// │  │          │      ├──► Viewport
/// │  │          │      │
/// │  │          ├──────┼─────────────────┐
/// │  │          │xxxxxx│                 │
/// │  │          │xxxxx◄├─── Inner region │
/// │  └──────────┼──────┘                 │
/// │             │                        │
/// │             │               Screen 2 ├──► Relative to
/// │             ├────────────────────────┘
/// │             │
/// │    Screen 1 │
/// └─────────────┘
/// ````
#[derive(Debug, Copy, Clone)]
pub struct EmbeddedRegion {
    /// The coordinate sysd
    pub relative_to: LogicalRegion,
    pub inner: Region,
}

/// Rectangle area in an unspecified coordinate system.
///
/// Use `LogicalRegion` or `EmbeddedRegion` instead as they convey the
/// coordinate system used.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Region {
    /// Position of the region.
    pub position: Position,
    /// Size of the region.
    pub size: Size,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Position {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Size {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl EmbeddedRegion {
    /// Given two `LogicalRegion`s, one seen as the `viewport` and the other
    /// `relative_to` (think the output we want to capture), create an
    /// embedded region that is entirely inside of the `relative_to` region.
    ///
    /// See `EmbeddedRegion` for an example ASCII visualisation.
    #[tracing::instrument(ret, level = "debug")]
    pub fn new(viewport: LogicalRegion, relative_to: LogicalRegion) -> Option<Self> {
        let x_relative: i32 = viewport.inner.position.x - relative_to.inner.position.x;
        let y_relative = viewport.inner.position.y - relative_to.inner.position.y;

        let x1 = cmp::max(x_relative, 0);
        let x2 = cmp::min(
            x_relative + viewport.inner.size.width as i32,
            relative_to.inner.size.width as i32,
        );
        let width = if let Ok(width) = (x2 - x1).try_into() {
            if width < 1 {
                return None;
            };
            width
        } else {
            return None;
        };

        let y1 = cmp::max(y_relative, 0);
        let y2 = cmp::min(
            y_relative + viewport.inner.size.height as i32,
            relative_to.inner.size.height as i32,
        );
        let height = if let Ok(height) = (y2 - y1).try_into() {
            if height < 1 {
                return None;
            };
            height
        } else {
            return None;
        };

        Some(Self {
            relative_to,
            inner: Region {
                position: Position { x: x1, y: y1 },
                size: Size { width, height },
            },
        })
    }

    /// Return the `LogicalRegion` of the embedded region.
    ///
    /// Note that this remains a region of the same size, it's not the inverse
    /// of `EmbeddedRegion::new` which removes the parts that are outside of
    /// the `relative_to` region.
    pub fn logical(&self) -> LogicalRegion {
        LogicalRegion {
            inner: Region {
                position: Position {
                    x: self.relative_to.inner.position.x + self.inner.position.x,
                    y: self.relative_to.inner.position.y + self.inner.position.y,
                },
                size: self.inner.size,
            },
        }
    }
}

impl std::fmt::Display for EmbeddedRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{region} relative to {relative_to}",
            region = self.inner,
            relative_to = self.relative_to,
        )
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({x}, {y})", x = self.x, y = self.y,)
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({width}x{height})",
            width = self.width,
            height = self.height,
        )
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{position} {size}",
            position = self.position,
            size = self.size,
        )
    }
}

impl std::fmt::Display for LogicalRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{inner}", inner = self.inner)
    }
}

impl From<&OutputInfo> for LogicalRegion {
    fn from(output_info: &OutputInfo) -> Self {
        LogicalRegion {
            inner: output_info.logical_region.inner,
        }
    }
}

impl TryFrom<&[OutputInfo]> for LogicalRegion {
    type Error = Error;

    fn try_from(output_info: &[OutputInfo]) -> std::result::Result<Self, Self::Error> {
        let x1 = output_info
            .iter()
            .map(|output| output.logical_region.inner.position.x)
            .min()
            .ok_or(Error::NoOutputs)?;
        let y1 = output_info
            .iter()
            .map(|output| output.logical_region.inner.position.y)
            .min()
            .ok_or(Error::NoOutputs)?;
        let x2 = output_info
            .iter()
            .map(|output| {
                output.logical_region.inner.position.x
                    + output.logical_region.inner.size.width as i32
            })
            .max()
            .ok_or(Error::NoOutputs)?;
        let y2 = output_info
            .iter()
            .map(|output| {
                output.logical_region.inner.position.y
                    + output.logical_region.inner.size.height as i32
            })
            .max()
            .ok_or(Error::NoOutputs)?;
        Ok(LogicalRegion {
            inner: Region {
                position: Position { x: x1, y: y1 },
                size: Size {
                    width: (x2 - x1) as u32,
                    height: (y2 - y1) as u32,
                },
            },
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::mem;
    use std::os::unix::net::UnixStream;
    use wayland_backend::client::Backend;
    use wayland_client::{Proxy, protocol::wl_output::WlOutput};

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
}
