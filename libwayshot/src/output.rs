use crate::region::{LogicalRegion, Size};
use std::fmt::Display;
use wayland_client::protocol::{wl_output, wl_output::WlOutput};

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
    pub scale: i32,
}

// The scale in OutputInfo is currently not being used anywhere,
// I originally planned to use it to calculate the stride somehow rather than hard-coding it,
// But I suppose it went over my head, and I couldn't perform it.

// For whoever is planning to work on it,
// Be mindful of the scaling as it can break the image structure and wlshm memory.
// Again As I was testing on Cosmic, Format which are being displayed and do end up working,
// Don't again necessarily work on Cosmic due to Wl-shm memory handling which could again be Cosmic's Alpha stage issue.

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
