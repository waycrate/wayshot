use std::fmt::Display;

use wayland_client::protocol::{wl_output, wl_output::WlOutput};

use crate::region::{LogicalRegion, Position, Size};

/// Represents an accessible wayland output.
///
/// Record the useful information of a WlOutput
/// The most important part is wl_output and transform
/// The two part will influence the output of the image
/// If you are using [crate::WayshotConnection::screenshot_single_output], you can do not care about
/// the physical_size and logical_region
/// But with region screenshot, they are needed
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub description: String,
    pub transform: wl_output::Transform,
    pub physical_size: Size,
    pub logical_region: LogicalRegion,
}

impl OutputInfo {
    /// create a [OutputInfo] with new
    pub fn new(wl_output: WlOutput) -> Self {
        Self {
            wl_output,
            name: "".to_owned(),
            description: "".to_owned(),
            transform: wl_output::Transform::Normal,
            physical_size: Size::default(),
            logical_region: LogicalRegion::default(),
        }
    }
    /// set the transform of [OutputInfo]
    pub fn transform(mut self, transform: wl_output::Transform) -> Self {
        self.transform = transform;
        self
    }
}

impl AsRef<WlOutput> for OutputInfo {
    fn as_ref(&self) -> &WlOutput {
        &self.wl_output
    }
}

pub trait ToOutputInfo {
    fn output_info(self) -> OutputInfo;
}

impl ToOutputInfo for WlOutput {
    fn output_info(self) -> OutputInfo {
        OutputInfo {
            wl_output: self,
            name: "".to_owned(),
            description: "".to_owned(),
            transform: wl_output::Transform::Normal,
            physical_size: Size::default(),
            logical_region: LogicalRegion::default(),
        }
    }
}

impl ToOutputInfo for &WlOutput {
    fn output_info(self) -> OutputInfo {
        OutputInfo {
            wl_output: self.clone(),
            name: "".to_owned(),
            description: "".to_owned(),
            transform: wl_output::Transform::Normal,
            physical_size: Size::default(),
            logical_region: LogicalRegion::default(),
        }
    }
}

impl ToOutputInfo for &OutputInfo {
    fn output_info(self) -> OutputInfo {
        self.clone()
    }
}

impl ToOutputInfo for OutputInfo {
    fn output_info(self) -> OutputInfo {
        self
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
