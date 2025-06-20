use crate::region::{LogicalRegion, Position, Size};
use std::fmt::Display;
use wayland_client::protocol::{wl_output, wl_output::WlOutput};

/// Represents an accessible wayland output.
///
/// Do not instantiate, instead use [`crate::WayshotConnection::get_all_outputs`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputInfo {
    pub output: WlOutput,
    pub name: String,
    pub description: String,
    pub transform: wl_output::Transform,
    pub physical_size: Size,
    pub logical_region: LogicalRegion,
    pub scale: i32,
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
    /// The name of the output or maybe the screen?
    pub fn name(&self) -> &str {
        &self.name
    }
}
