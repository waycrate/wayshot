use std::cmp;

use crate::error::{Error, Result};
use crate::output::OutputInfo;

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
            height
        } else {
            return None;
        };

        Some(Self {
            relative_to: relative_to,
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
            "({position}) ({size})",
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
            inner: output_info.region,
        }
    }
}

impl TryFrom<&Vec<OutputInfo>> for LogicalRegion {
    type Error = Error;

    fn try_from(output_info: &Vec<OutputInfo>) -> std::result::Result<Self, Self::Error> {
        let x1 = output_info
            .iter()
            .map(|output| output.region.position.x)
            .min()
            .ok_or(Error::NoOutputs)?;
        let y1 = output_info
            .iter()
            .map(|output| output.region.position.y)
            .min()
            .ok_or(Error::NoOutputs)?;
        let x2 = output_info
            .iter()
            .map(|output| output.region.position.x + output.region.size.width as i32)
            .max()
            .ok_or(Error::NoOutputs)?;
        let y2 = output_info
            .iter()
            .map(|output| output.region.position.y + output.region.size.height as i32)
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
