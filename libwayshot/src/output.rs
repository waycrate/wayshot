use wayland_client::protocol::{wl_output, wl_output::WlOutput};

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub transform: wl_output::Transform,
    pub dimensions: OutputPositioning,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct OutputPositioning {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
