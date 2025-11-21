use std::fmt::Display;

use wayland_client::protocol::{wl_output, wl_output::WlOutput};

use crate::region::{LogicalRegion, Size};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::{Position, Region};
    use std::mem::{self, MaybeUninit};
    use std::ptr;

    fn make_output_info(
        name: &str,
        description: &str,
        physical_size: Size,
        logical_region: LogicalRegion,
    ) -> OutputInfo {
        let dummy_wl_output = unsafe {
            let mut uninit = MaybeUninit::<WlOutput>::uninit();
            ptr::write_bytes(uninit.as_mut_ptr(), 1, 1);
            uninit.assume_init()
        };

        OutputInfo {
            wl_output: dummy_wl_output,
            name: name.to_string(),
            description: description.to_string(),
            transform: wl_output::Transform::Normal,
            physical_size,
            logical_region,
        }
    }

    #[test]
    fn display_formats_name_and_description() {
        let output_info = make_output_info(
            "HDMI-A-1",
            "Main Display",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );

        assert_eq!(output_info.to_string(), "HDMI-A-1 (Main Display)");

        mem::forget(output_info);
    }

    #[test]
    fn display_formats_empty_name_and_description() {
        let output_info = make_output_info(
            "",
            "",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );

        assert_eq!(output_info.to_string(), " ()");

        mem::forget(output_info);
    }

    #[test]
    fn scale_returns_ratio_between_physical_and_logical_heights() {
        let output_info = make_output_info(
            "DP-1",
            "Secondary Display",
            Size {
                width: 3840,
                height: 2160,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );

        assert_eq!(output_info.scale(), 2.0);

        mem::forget(output_info);
    }

    #[test]
    fn scale_returns_correct_ratio_for_various_sizes() {
        let output_info_1 = make_output_info(
            "eDP-1",
            "Laptop Screen",
            Size {
                width: 1920,
                height: 1080,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 1920,
                        height: 1080,
                    },
                },
            },
        );
        assert_eq!(output_info_1.scale(), 1.0);
        mem::forget(output_info_1);

        let output_info_1_5 = make_output_info(
            "DP-2",
            "HiDPI Display",
            Size {
                width: 3840,
                height: 2160,
            },
            LogicalRegion {
                inner: Region {
                    position: Position { x: 0, y: 0 },
                    size: Size {
                        width: 2560,
                        height: 1440,
                    },
                },
            },
        );
        assert_eq!(output_info_1_5.scale(), 1.5);
        mem::forget(output_info_1_5);
    }

}
