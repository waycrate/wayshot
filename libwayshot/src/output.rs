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

/// When using [crate::WayshotConnection::screenshot_single_output],
/// We only need a [WlOutput] and [wl_output::Transform]
/// So we added this structure
pub struct SingleOutputInfo {
    wl_output: WlOutput,
    transform: wl_output::Transform,
}

impl SingleOutputInfo {
    /// Create a new SingleOutputInfo
    pub fn new(wl_output: WlOutput) -> Self {
        Self {
            wl_output,
            transform: wl_output::Transform::Normal,
        }
    }

    /// Change the [wl_output::Transform] in [SingleOutputInfo]
    pub fn transform(mut self, transform: wl_output::Transform) -> Self {
        self.transform = transform;
        self
    }

    /// return the [WlOutput] of the [SingleOutputInfo]
    pub fn wl_output(&self) -> WlOutput {
        self.wl_output.clone()
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

impl ToOutputInfo for &SingleOutputInfo {
    fn output_info(self) -> OutputInfo {
        OutputInfo {
            wl_output: self.wl_output.clone(),
            name: "".to_owned(),
            description: "".to_owned(),
            transform: self.transform,
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
