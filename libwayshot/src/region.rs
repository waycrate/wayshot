use std::cmp;

use wayland_client::protocol::wl_output::Transform;

use crate::error::{Error, Result};
use crate::output::OutputInfo;
use crate::screencopy::FrameCopy;

/// Ways to say how a region for a screenshot should be captured.
pub enum RegionCapturer {
    /// Capture all of the given outputs.
    Outputs(Vec<OutputInfo>),
    /// Capture an already known `LogicalRegion`.
    Region(LogicalRegion),
    /// The outputs will be "frozen" to the user at which point the given
    /// callback is called to get the region to capture. This callback is often
    /// a user interaction to let the user select a region.
    Freeze(Box<dyn Fn() -> Result<LogicalRegion>>),
}

/// `Region` where the coordinate system is the logical coordinate system used
/// in Wayland to position outputs. Top left is (0, 0) and any transforms and
/// scaling have been applied.
#[derive(Debug, Copy, Clone)]
pub struct LogicalRegion {
    pub inner: Region,
}

/// An embedded region is a region entirely inside of another (often an output).
///
/// It can only be contained inside of another and cannot exceed its bounds.
///
/// Example of what
///
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
#[derive(Debug, Copy, Clone)]
pub struct Region {
    /// X coordinate of the area to capture.
    pub x: i32,
    /// y coordinate of the area to capture.
    pub y: i32,
    /// Width of the capture area.
    pub width: i32,
    /// Height of the capture area.
    pub height: i32,
}

impl EmbeddedRegion {
    /// Given two `LogicalRegion`s, one seen as the `viewport` and the other
    /// `relative_to` (think the output we want to capture), create an
    /// embedded region that is entirely inside of the `relative_to` region.
    ///
    /// See `EmbeddedRegion` for an example ASCII visualisation.
    #[tracing::instrument(ret, level = "debug")]
    pub fn new(viewport: LogicalRegion, relative_to: LogicalRegion) -> Option<Self> {
        let x_relative: i32 = viewport.inner.x - relative_to.inner.x;
        let y_relative = viewport.inner.y - relative_to.inner.y;

        let x1 = cmp::max(x_relative, 0);
        let x2 = cmp::min(x_relative + viewport.inner.width, relative_to.inner.width);
        let width = x2 - x1;
        if width <= 0 {
            return None;
        }

        let y1 = cmp::max(y_relative, 0);
        let y2 = cmp::min(y_relative + viewport.inner.height, relative_to.inner.height);
        let height = y2 - y1;
        if height <= 0 {
            return None;
        }

        Some(Self {
            relative_to: relative_to,
            inner: Region {
                x: x1,
                y: y1,
                width,
                height,
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
                x: self.relative_to.inner.x as i32 + self.inner.x,
                y: self.relative_to.inner.y as i32 + self.inner.y,
                width: self.inner.width,
                height: self.inner.height,
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

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({x}, {y}) ({width}x{height})",
            x = self.x,
            y = self.y,
            width = self.width,
            height = self.height,
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
            inner: Region {
                x: output_info.dimensions.x,
                y: output_info.dimensions.y,
                width: output_info.dimensions.width,
                height: output_info.dimensions.height,
            },
        }
    }
}

impl TryFrom<&Vec<OutputInfo>> for LogicalRegion {
    type Error = Error;

    fn try_from(output_info: &Vec<OutputInfo>) -> std::result::Result<Self, Self::Error> {
        let x1 = output_info
            .iter()
            .map(|output| output.dimensions.x)
            .min()
            .unwrap();
        let y1 = output_info
            .iter()
            .map(|output| output.dimensions.y)
            .min()
            .unwrap();
        let x2 = output_info
            .iter()
            .map(|output| output.dimensions.x + output.dimensions.width)
            .max()
            .unwrap();
        let y2 = output_info
            .iter()
            .map(|output| output.dimensions.y + output.dimensions.height)
            .max()
            .unwrap();
        Ok(LogicalRegion {
            inner: Region {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            },
        })
    }
}

impl From<&FrameCopy> for LogicalRegion {
    fn from(frame_copy: &FrameCopy) -> Self {
        let (width, height) = (
            frame_copy.frame_format.width as i32,
            frame_copy.frame_format.height as i32,
        );
        let is_portait = match frame_copy.transform {
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => true,
            _ => false,
        };
        LogicalRegion {
            inner: Region {
                x: frame_copy.position.0 as i32,
                y: frame_copy.position.1 as i32,
                width: if is_portait { height } else { width },
                height: if is_portait { width } else { height },
            },
        }
    }
}
