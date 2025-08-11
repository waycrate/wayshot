use wayland_client::WEnum;

use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

use wayland_client::protocol::{
    wl_buffer::WlBuffer,
    wl_output::{self, WlOutput},
    wl_shm::Format,
};

use wayland_protocols_wlr::layer_shell::v1::client::{
	zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
	zwlr_layer_surface_v1::{Anchor, self, ZwlrLayerSurfaceV1},
};

use std::sync::{Arc, RwLock};

use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options;

use image::{DynamicImage, ImageBuffer};

use image::ColorType;

use std::os::fd::{AsFd};

use std::fs::File;

use crate::WayshotConnection;
use crate::WayshotError; // Removed WayshotBase import
use crate::dispatch::{FrameState, LayerShellState};
use crate::region::{Position, Region, Size, LogicalRegion};
use crate::screencopy::{create_shm_fd, FrameFormat};

#[allow(unused)]
#[derive(Debug)]
struct CaptureTopLevelData {
	buffer: WlBuffer,
	frame_info: FrameFormat,
	transform: wl_output::Transform,
	mmap: Option<memmap2::MmapMut>, // Add mmap for pixel data
}

#[derive(Debug)]
pub(crate) struct CaptureOutputData {
    pub(crate) output: WlOutput,

    pub(crate) buffer: WlBuffer,

    pub(crate) frame_info: FrameFormat,
	pub(crate) color_type: ColorType, // added here
	pub(crate) mmap: Option<memmap2::MmapMut>, // NEW: store mmap for image data
	pub(crate) transform: wl_output::Transform,
	pub(crate) logical_region: LogicalRegion, // replaces width, height, screen_position
	pub(crate) physical_size: Size, // replaced real_width/real_height
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) title: String,
	pub(crate) app_id: String,
	pub(crate) identifier: String,
	pub(crate) active: bool,
}

impl TopLevel {
    pub(crate) fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
			title: "".to_owned(),
			app_id: "".to_owned(),
			identifier: "".to_owned(),
			active: true,
		}
    }

	pub fn id_and_title(&self) -> String {
		format!("{} {}", self.app_id, self.title)
	}
}

pub(crate) struct CaptureInfo {
    pub(crate) transform: wl_output::Transform,
    pub(crate) state: FrameState,
}

impl CaptureInfo {
    pub(crate) fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            transform: wl_output::Transform::Normal,
            state: FrameState::Pending,
        }))
    }

    pub(crate) fn transform(&self) -> wl_output::Transform {
        self.transform
    }
    pub(crate) fn state(&self) -> FrameState {
        self.state
    }
}

pub trait AreaSelectCallback {
    fn screenshot(self, state: &WayshotConnection) -> Result<Region, WayshotError>;
}

impl<F> AreaSelectCallback for F
where
    F: Fn(&WayshotConnection) -> Result<Region, WayshotError>,
{
    fn screenshot(self, state: &WayshotConnection) -> Result<Region, WayshotError> {
        self(state)
    }
}
impl AreaSelectCallback for Region {
    fn screenshot(self, _state: &WayshotConnection) -> Result<Region, WayshotError> {
        Ok(self)
    }
}

/// Describe the capture option
/// Now this library provide two options
/// [CaptureOption::PaintCursors] and [CaptureOption::None]
/// It decides whether cursor will be shown
#[derive(Debug, Clone, Copy)]
pub enum CaptureOption {
    PaintCursors,
    None,
}

impl From<CaptureOption> for Options {
    fn from(val: CaptureOption) -> Self {
        match val {
            CaptureOption::None => Options::empty(),
            CaptureOption::PaintCursors => Options::PaintCursors,
        }
    }
}

pub(crate) struct AreaShotInfo {
    pub(crate) data: CaptureOutputData,
}

impl AreaShotInfo {
    pub(crate) fn in_this_screen(
        &self,
        Region {
            position: point, ..
        }: Region,
    ) -> bool {
        let CaptureOutputData {
            physical_size,
            logical_region,
            ..
        } = &self.data;
        let Position { x, y } = logical_region.inner.position;
        if point.y < y
            || point.x < x
            || point.x > x + physical_size.width as i32
            || point.y > y + physical_size.height as i32
        {
            return false;
        }
        true
    }
    pub(crate) fn clip_area(&self, region: Region) -> Option<Region> {
        if !self.in_this_screen(region) {
            return None;
        }
        let CaptureOutputData {
            physical_size,
            logical_region,
            ..
        } = &self.data;
        let width = logical_region.inner.size.width;
        let height = logical_region.inner.size.height;
        let screen_position = logical_region.inner.position;
        let Region {
            position: point,
            size,
        } = region;
        let relative_point = point - screen_position;
        let position = Position {
            x: (relative_point.x as f64 * width as f64 / physical_size.width as f64) as i32,
            y: (relative_point.y as f64 * height as f64 / physical_size.height as f64) as i32,
        };

        Some(Region {
            position,
            size: Size {
                width: (size.width as f64 * width as f64 / physical_size.width as f64) as u32,
                height: (size.height as f64 * height as f64 / physical_size.height as f64) as u32,
            },
        })
    }
}

// Implementation of WayshotConnection methods related to ext_image_protocols
impl crate::WayshotConnection {

	/// Capture a single output and return a DynamicImage
	pub fn ext_capture_single_output(
		&mut self,
		option: CaptureOption,
		output: crate::output::OutputInfo,
	) -> std::result::Result<DynamicImage, crate::WayshotError> {
		let mem_fd = create_shm_fd().unwrap();
		let mem_file = File::from(mem_fd);
		let mut capture_data = self.ext_capture_output_inner(
			output.clone(),
			option,
			mem_file.as_fd(),
			Some(&mem_file),
		)?;

		let mut frame_mmap = unsafe { memmap2::MmapMut::map_mut(&mem_file).unwrap() };

		let converter = crate::convert::create_converter(capture_data.frame_info.format).unwrap();
		let color_type = converter.convert_inplace(&mut frame_mmap);

		capture_data.color_type = color_type;
		capture_data.mmap = Some(frame_mmap);

		// Use TryFrom to convert to DynamicImage
		(&capture_data).try_into()
	}

    pub fn ext_capture_area2<F>(
        &mut self,
        option: CaptureOption,
        callback: F,
    ) -> std::result::Result<(Vec<u8>, u32, u32, ColorType, Region), crate::WayshotError>
    where
        F: AreaSelectCallback,
    {
        use wayland_client::{protocol::wl_surface::WlSurface, EventQueue};
        use wayland_protocols::xdg::shell::client::{xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel};
        use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
        use wayland_client::protocol::wl_compositor::WlCompositor;
        use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
        use tracing::debug;

        let outputs = self.vector_of_Outputs().clone();

        let mut data_list = vec![];
        for data in outputs.into_iter() {
            let mem_fd = create_shm_fd().unwrap();
            let mem_file = File::from(mem_fd);
            let mut data =
                self.ext_capture_output_inner(data, option, mem_file.as_fd(), Some(&mem_file))?;
            // Set mmap in CaptureOutputData
            let frame_mmap = unsafe { memmap2::MmapMut::map_mut(&mem_file).unwrap() };
            data.mmap = Some(frame_mmap);
            data_list.push(AreaShotInfo { data })
        }

        let mut state = LayerShellState::new();
        let mut event_queue: EventQueue<LayerShellState> = self.conn.new_event_queue();
        let globals = &self.globals;
        let qh = event_queue.handle();

        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 3..=3, ())?;
        let layer_shell = globals.bind::<ZwlrLayerShellV1, _, _>(&qh, 1..=1, ())?;
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ())?;

		let mut layer_shell_surfaces: Vec<(WlSurface, ZwlrLayerSurfaceV1)> =
			Vec::with_capacity(data_list.len());
        for AreaShotInfo { data, .. } in data_list.iter() {
            let CaptureOutputData {
                output,
                buffer,
                physical_size,
                transform,
                ..
            } = data;
            let surface = compositor.create_surface(&qh, ());
			let layer_surface = layer_shell.get_layer_surface(
				&surface,
				Some(output),
				Layer::Top,
				"wayshot".to_string(),
				&qh,
				output.clone(),
			);

            // Configure the toplevel to be fullscreen on the specific output
			layer_surface.set_exclusive_zone(-1);
			layer_surface.set_anchor(Anchor::all());
			layer_surface.set_margin(0, 0, 0, 0);

            debug!("Committing surface creation changes.");
            surface.commit();

            debug!("Waiting for layer surface to be configured.");
			while !state.configured_outputs.contains(output) {
				event_queue.blocking_dispatch(&mut state)?;
            }

            surface.set_buffer_transform(*transform);
            // surface.set_buffer_scale(output_info.scale());
            surface.attach(Some(buffer), 0, 0);

            let viewport = viewporter.get_viewport(&surface, &qh, ());
            viewport.set_destination(physical_size.width as i32, physical_size.height as i32);

            debug!("Committing surface with attached buffer.");
            surface.commit();
			layer_shell_surfaces.push((surface, layer_surface));
			event_queue.blocking_dispatch(&mut state)?;
        }

        let region_re = callback.screenshot(self);

        debug!("Unmapping and destroying layer shell surfaces.");
		for (surface, layer_shell_surface) in layer_shell_surfaces.iter() {
			surface.attach(None, 0, 0);
			surface.commit(); //unmap surface by committing a null buffer
			layer_shell_surface.destroy();
        }
        event_queue.roundtrip(&mut state)?;
        let region = region_re?;

        let shotdata = data_list
            .iter()
            .find(|data| data.in_this_screen(region))
            .ok_or(crate::WayshotError::CaptureFailed("not in region".to_owned()))?;
        let area = shotdata.clip_area(region).expect("should have");
        // Use mmap from CaptureOutputData
        let shotdata_ref = &shotdata.data;
        let frame_mmap = shotdata_ref.mmap.as_ref().unwrap();
        let converter = crate::convert::create_converter(shotdata_ref.frame_info.format).unwrap();
        let mut mmap_vec = frame_mmap.to_vec();
        let color_type = converter.convert_inplace(&mut mmap_vec);
        // Return tuple instead of ImageViewInfo
        Ok((
            mmap_vec,
            shotdata_ref.logical_region.inner.size.width,
            shotdata_ref.logical_region.inner.size.height,
            color_type,
            area,
        ))
    }

	/// Capture a single output
	pub fn ext_capture_toplevel2(
		&mut self,
		option: CaptureOption,
		toplevel: TopLevel,
	) -> Result<DynamicImage, WayshotError> {
		let mem_fd = create_shm_fd().unwrap();
		let mem_file = File::from(mem_fd);
		let mut capture = self.ext_capture_toplevel_inner(toplevel, option, mem_file.as_fd(), Some(&mem_file))?;

		let mut frame_mmap = unsafe { memmap2::MmapMut::map_mut(&mem_file).unwrap() };
		let converter = crate::convert::create_converter(capture.frame_info.format).unwrap();
		converter.convert_inplace(&mut frame_mmap);
		capture.mmap = Some(frame_mmap);

        // Use TryFrom to convert to DynamicImage
        (&capture).try_into()
	}

	fn ext_capture_output_inner<T: AsFd>(
		&mut self,
		output_info: crate::output::OutputInfo,
		option: CaptureOption,
		fd: T,
		file: Option<&File>,
	) -> std::result::Result<CaptureOutputData, crate::WayshotError> {
		let crate::output::OutputInfo {
			output,
			logical_region,
			..
		} = output_info;

		let mut event_queue = self
			.ext_image
			.as_mut()
			.expect("ext_image should be initialized")
			.event_queue
			.take()
			.expect("Control your self");
		let img_manager = self
			.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.output_image_manager
			.as_ref()
			.expect("Should init");
		let capture_manager = self
			.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.img_copy_manager
			.as_ref()
			.expect("Should init");
		let qh = self
			.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.qh
			.as_ref()
			.expect("Should init");
		let source = img_manager.create_source(&output, qh, ());
		let info = Arc::new(RwLock::new(FrameFormat {
			format: Format::Xrgb8888, // placeholder, will be set by protocol event
			size: Size { width: 0, height: 0 }, // placeholder
			stride: 0, // placeholder
		}));
		let session = capture_manager.create_session(&source, option.into(), qh, info.clone());

		let capture_info = CaptureInfo::new();
		let frame = session.create_frame(qh, capture_info.clone());
		event_queue.blocking_dispatch(self).unwrap();
		let qh = self
			.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.qh
			.as_ref()
			.expect("Should init");
		let shm = self
			.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.shm
			.as_ref()
			.expect("Should init");
		let info = info.read().unwrap();

		// Use direct field access for FrameInfo
		let Size { width, height } = info.size;
		let frame_format = info.format;
		if !matches!(
            frame_format,
			Format::Xbgr2101010
				| Format::Xrgb2101010
                | Format::Abgr2101010
                | Format::Argb8888
                | Format::Xrgb8888
                | Format::Xbgr8888
				| Format::Bgr888
        ) {
			println!("Unsupported format: {:?}", frame_format);
			return Err(crate::WayshotError::NotSupportFormat);
		} else {
			println!("Matched format: {:?}", frame_format);
		}

		let frame_bytes = 4 * height * width;
		let mem_fd = fd.as_fd();

		if let Some(file) = file {
			file.set_len(frame_bytes as u64).unwrap();
		}

		let stride = 4 * width;

		let shm_pool = shm.create_pool(mem_fd, (width * height * 4) as i32, qh, ());
		let buffer = shm_pool.create_buffer(
			0,
			width as i32,
			height as i32,
			stride as i32,
			frame_format,
			qh,
			(),
		);
		frame.attach_buffer(&buffer);
		frame.capture();

		let transform;
		loop {
			event_queue.blocking_dispatch(self)?;
			let info = capture_info.read().unwrap();
			match info.state() {
				FrameState::Succeeded => {
					transform = info.transform();
					break;
				}
				FrameState::Failed(info) => match info {
					Some(WEnum::Value(reason)) => match reason {
						wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason::Stopped => {
							return Err(crate::WayshotError::CaptureFailed("Stopped".to_owned()));
						}

						wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason::BufferConstraints => {
							return Err(crate::WayshotError::CaptureFailed(
								"BufferConstraints".to_owned(),
							));
						}
						wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason::Unknown | _ => {
							return Err(crate::WayshotError::CaptureFailed("Unknown".to_owned()));
						}
					},
					Some(WEnum::Unknown(code)) => {
						return Err(crate::WayshotError::CaptureFailed(format!(
							"Unknown reason, code : {code}"
						)));
					}
					None => {
						return Err(crate::WayshotError::CaptureFailed(
							"No failure reason provided".to_owned(),
						));
					}
				},
				FrameState::Pending => {}
			}
		}

		self.reset_event_queue(event_queue);

		Ok(CaptureOutputData {
			output,
			buffer,
			logical_region: logical_region.clone(),
			frame_info: FrameFormat {
				format: frame_format,
				size: Size {
					width: logical_region.inner.size.width as u32,
					height: logical_region.inner.size.height as u32,
				},
				stride,
			},
			transform,
			color_type: ColorType::Rgba8, // placeholder, will be set after conversion
			physical_size: Size {
				width: logical_region.inner.size.width as u32,
				height: logical_region.inner.size.height as u32,
			},
			mmap: None, // Initialize mmap as None
		})
	}

	fn ext_capture_toplevel_inner<T: AsFd>(
		&mut self,
		TopLevel { handle, .. }: TopLevel,
		option: CaptureOption,
		fd: T,
		file: Option<&File>,
	) -> Result<CaptureTopLevelData, WayshotError> {
		let mut event_queue = self.ext_image
			.as_mut()
			.expect("ext_image should be initialized")
			.event_queue
			.take()
			.expect("Control your self");
		let img_manager = self.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.toplevel_image_manager
			.as_ref()
			.expect("Should init");
		let capture_manager = self.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.img_copy_manager
			.as_ref()
			.expect("Should init");
		let qh = self.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.qh
			.as_ref()
			.expect("Should init");
		let source = img_manager.create_source(&handle, qh, ());
        // Provide a default FrameInfo since FrameInfo::default() does not exist
        let info = Arc::new(RwLock::new(FrameFormat {
			format: Format::Xrgb8888, // placeholder, will be set by protocol event
			size: Size { width: 0, height: 0 }, // placeholder
			stride: 0, // placeholder
		}));
		let session = capture_manager.create_session(&source, option.into(), qh, info.clone());

		let capture_info = CaptureInfo::new();
		let frame = session.create_frame(qh, capture_info.clone());
		event_queue.blocking_dispatch(self).unwrap();
		let qh = self.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.qh
			.as_ref()
			.expect("Should init");
		let shm = self.ext_image
			.as_ref()
			.expect("ext_image should be initialized")
			.shm
			.as_ref()
			.expect("Should init");
		let info = info.read().unwrap();

		let Size { width, height } = info.size;
		let frame_format = info.format;
		let frame_bytes = 4 * height * width;
		if !matches!(
			frame_format,
			Format::Xbgr2101010
				| Format::Abgr2101010
				| Format::Argb8888
				| Format::Xrgb8888
				| Format::Xbgr8888
        ) {
			return Err(WayshotError::NotSupportFormat);
		}
		let mem_fd = fd.as_fd();

		if let Some(file) = file {
			file.set_len(frame_bytes as u64).unwrap();
		}

		let stride = 4 * width;

		let shm_pool = shm.create_pool(mem_fd, (width * height * 4) as i32, qh, ());
		let buffer = shm_pool.create_buffer(
		0,
			width as i32,
			height as i32,
			stride as i32,
			frame_format,
			qh,
			(),
		);
		frame.attach_buffer(&buffer);
		frame.capture();

		let transform;
		loop {
			event_queue.blocking_dispatch(self)?;
			let info = capture_info.read().unwrap();
			match info.state() {
				FrameState::Succeeded => {
					transform = info.transform();
					break;
				}
				FrameState::Failed(info) => match info {
					Some(WEnum::Value(reason)) => match reason {
						FailureReason::Stopped => {
							return Err(WayshotError::CaptureFailed("Stopped".to_owned()));
						}

						FailureReason::BufferConstraints => {
							return Err(WayshotError::CaptureFailed("BufferConstraints".to_owned()));
						}
						FailureReason::Unknown | _ => {
							return Err(WayshotError::CaptureFailed("Unknown".to_owned()));
						}
					},
					Some(WEnum::Unknown(code)) => {
						return Err(WayshotError::CaptureFailed(format!(
							"Unknown reason, code : {code}"
						)));
					}
					None => {
						return Err(WayshotError::CaptureFailed(
							"No failure reason provided".to_owned(),
						));
					}
				},
				FrameState::Pending => {}
			}
		}

		self.reset_event_queue(event_queue);

		Ok(CaptureTopLevelData {
			transform,
			buffer,
			frame_info: FrameFormat {
				format: frame_format,
				size: Size { width, height },
				stride,
			},
			mmap: None, // mmap will be set after return, just like Output
		})
	}
}

use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason;

impl TryFrom<&CaptureOutputData> for DynamicImage {
    type Error = WayshotError;

    fn try_from(value: &CaptureOutputData) -> Result<Self, WayshotError> {
        let mmap = value.mmap.as_ref().ok_or(WayshotError::BufferTooSmall)?;
        let width = value.frame_info.size.width;
        let height = value.frame_info.size.height;
        match value.color_type {
            image::ColorType::Rgb8 => {
                let buffer = ImageBuffer::from_vec(width, height, mmap.to_vec())
                    .ok_or(WayshotError::BufferTooSmall)?;
                Ok(DynamicImage::ImageRgb8(buffer))
            }
            image::ColorType::Rgba8 => {
                let buffer = ImageBuffer::from_vec(width, height, mmap.to_vec())
                    .ok_or(WayshotError::BufferTooSmall)?;
                Ok(DynamicImage::ImageRgba8(buffer))
            }
            _ => Err(WayshotError::InvalidColor),
        }
    }
}

impl TryFrom<&CaptureTopLevelData> for DynamicImage {
    type Error = WayshotError;

    fn try_from(value: &CaptureTopLevelData) -> Result<Self, WayshotError> {
        let mmap = value.mmap.as_ref().ok_or(WayshotError::BufferTooSmall)?;
        let width = value.frame_info.size.width;
        let height = value.frame_info.size.height;
        // Assume RGBA8 for toplevel, adjust if you add color_type
        let buffer = ImageBuffer::from_vec(width, height, mmap.to_vec())
            .ok_or(WayshotError::BufferTooSmall)?;
        Ok(DynamicImage::ImageRgba8(buffer))
    }
}
