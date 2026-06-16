use std::fmt::Display;

use wayland_client::protocol::{wl_output, wl_output::WlOutput};

use crate::region::{LogicalRegion, Position, Size};

/// Represents an accessible wayland output.
///
/// Do not instantiate, instead use [`crate::WayshotConnection::get_all_outputs`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub description: String,
    pub transform: wl_output::Transform,
    pub physical_size: Size,
    pub logical_region: LogicalRegion,
}

impl AsRef<WlOutput> for OutputInfo {
    fn as_ref(&self) -> &WlOutput {
        &self.wl_output
    }
}

impl Display for OutputInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{name} ({description})",
            name = self.name,
            description = self.description
        )
    }
}

impl OutputInfo {
    pub(crate) fn scale(&self) -> f64 {
        self.physical_size.height as f64 / self.logical_region.inner.size.height as f64
    }

    /// return the physical_size
    pub fn physical_size(&self) -> Size {
        self.physical_size
    }

    /// return the logical_size
    pub fn logical_size(&self) -> Size {
        self.logical_region.inner.size
    }

    /// return the position of screen
    pub fn logical_position(&self) -> Position {
        self.logical_region.inner.position
    }
}
