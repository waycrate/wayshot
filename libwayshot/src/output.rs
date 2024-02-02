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
    pub scale: i32,
    pub dimensions: OutputPositioning,
    pub mode: WlOutputMode,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WlOutputMode {
    pub width: i32,
    pub height: i32,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputPositioning {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
