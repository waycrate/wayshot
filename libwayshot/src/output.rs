use std::fmt::Display;
use std::sync::OnceLock;
use wayland_client::protocol::{wl_output, wl_output::WlOutput};
use crate::ext_image_protocols::WlOutputInfo;
use crate::region::{LogicalRegion, Position, Size};

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
}

//impl OutputInfo {
//	pub fn name(&self) -> &str {
//		&self.name
//	}
//
//	pub(crate) fn new(output: WlOutput) -> Self {
//		Self {
//			wl_output: output,
//			
//			position: Position::default(),
//			size: Size::default(),
//			logical_size: Size::default(),
//			
//			name: "".to_owned(),
//			description: "".to_owned(),
//			
//			xdg_output: OnceLock::new(),
//			
//			transform: wl_output::Transform::Normal,
//			scale: 1,
//		}
//	}
//}