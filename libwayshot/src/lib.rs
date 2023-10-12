//! `libwayshot` is a convenient wrapper over the wlroots screenshot protocol
//! that provides a simple API to take screenshots with.
//!
//! To get started, look at [`WayshotConnection`].

mod convert;
mod dispatch;
mod error;
mod image_util;
pub mod output;
mod screencopy;

use std::{
    cmp,
    fs::File,
    os::fd::AsFd,
    process::exit,
    sync::atomic::{AtomicBool, Ordering},
    thread,
};

use image::{imageops::overlay, RgbaImage};
use memmap2::MmapMut;
use wayland_client::{
    globals::{registry_queue_init, GlobalList},
    protocol::{
        wl_output::{Transform, WlOutput},
        wl_shm::{self, WlShm},
    },
    Connection, EventQueue,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::{
    convert::create_converter,
    dispatch::{CaptureFrameState, FrameState, OutputCaptureState, WayshotState},
    output::OutputInfo,
    screencopy::{create_shm_fd, FrameCopy, FrameFormat},
};

pub use crate::error::{Error, Result};

pub mod reexport {
    use wayland_client::protocol::wl_output;
    pub use wl_output::{Transform, WlOutput};
}

type Frame = (Vec<FrameCopy>, (i32, i32));

/// Struct to store region capture details.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CaptureRegion {
    /// X coordinate of the area to capture.
    pub x_coordinate: i32,
    /// y coordinate of the area to capture.
    pub y_coordinate: i32,
    /// Width of the capture area.
    pub width: i32,
    /// Height of the capture area.
    pub height: i32,
}

#[derive(Debug)]
struct IntersectingOutput {
    output: WlOutput,
    region: CaptureRegion,
    transform: Transform,
}

/// Struct to store wayland connection and globals list.
/// # Example usage
///
/// ```
/// let wayshot_connection = WayshotConnection::new().unwrap();
/// let image_buffer = wayshot_connection.screenshot_all().unwrap();
/// ```
#[derive(Debug)]
pub struct WayshotConnection {
    pub conn: Connection,
    pub globals: GlobalList,
    output_infos: Vec<OutputInfo>,
}

impl WayshotConnection {
    pub fn new() -> Result<Self> {
        let conn = Connection::connect_to_env()?;

        Self::from_connection(conn)
    }

    /// Recommended if you already have a [`wayland_client::Connection`].
    pub fn from_connection(conn: Connection) -> Result<Self> {
        let (globals, _) = registry_queue_init::<WayshotState>(&conn)?;

        let mut initial_state = Self {
            conn,
            globals,
            output_infos: Vec::new(),
        };

        initial_state.refresh_outputs()?;

        Ok(initial_state)
    }

    /// Fetch all accessible wayland outputs.
    pub fn get_all_outputs(&self) -> &Vec<OutputInfo> {
        &self.output_infos
    }

    /// refresh the outputs, to get new outputs
    pub fn refresh_outputs(&mut self) -> Result<()> {
        // Connecting to wayland environment.
        let mut state = OutputCaptureState {
            outputs: Vec::new(),
        };
        let mut event_queue = self.conn.new_event_queue::<OutputCaptureState>();
        let qh = event_queue.handle();

        // Bind to xdg_output global.
        let zxdg_output_manager = match self.globals.bind::<ZxdgOutputManagerV1, _, _>(
            &qh,
            3..=3,
            (),
        ) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("Failed to create ZxdgOutputManagerV1 version 3. Does your compositor implement ZxdgOutputManagerV1?");
                panic!("{:#?}", e);
            }
        };

        // Fetch all outputs; when their names arrive, add them to the list
        let _ = self.conn.display().get_registry(&qh, ());
        event_queue.roundtrip(&mut state)?;
        event_queue.roundtrip(&mut state)?;

        // We loop over each output and request its position data.
        let xdg_outputs: Vec<ZxdgOutputV1> = state
            .outputs
            .iter()
            .enumerate()
            .map(|(index, output)| {
                zxdg_output_manager.get_xdg_output(&output.wl_output, &qh, index)
            })
            .collect();

        event_queue.roundtrip(&mut state)?;

        for xdg_output in xdg_outputs {
            xdg_output.destroy();
        }

        if state.outputs.is_empty() {
            tracing::error!("Compositor did not advertise any wl_output devices!");
            exit(1);
        }
        tracing::debug!("Outputs detected: {:#?}", state.outputs);
        self.output_infos = state.outputs;

        Ok(())
    }

    /// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
    ///  Data will be written to fd.
    pub fn capture_output_frame_shm_fd<T: AsFd>(
        &self,
        cursor_overlay: i32,
        output: &WlOutput,
        fd: T,
        capture_region: Option<CaptureRegion>,
    ) -> Result<FrameFormat> {
        let (state, event_queue, frame, frame_format) =
            self.capture_output_frame_get_state(cursor_overlay, output, capture_region)?;
        self.capture_output_frame_inner(state, event_queue, frame, frame_format, fd)
    }

    fn capture_output_frame_get_state(
        &self,
        cursor_overlay: i32,
        output: &WlOutput,
        capture_region: Option<CaptureRegion>,
    ) -> Result<(
        CaptureFrameState,
        EventQueue<CaptureFrameState>,
        ZwlrScreencopyFrameV1,
        FrameFormat,
    )> {
        let mut state = CaptureFrameState {
            formats: Vec::new(),
            state: None,
            buffer_done: AtomicBool::new(false),
        };
        let mut event_queue = self.conn.new_event_queue::<CaptureFrameState>();
        let qh = event_queue.handle();

        // Instantiating screencopy manager.
        let screencopy_manager = match self.globals.bind::<ZwlrScreencopyManagerV1, _, _>(
            &qh,
            3..=3,
            (),
        ) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!("Failed to create screencopy manager. Does your compositor implement ZwlrScreencopy?");
                tracing::error!("err: {e}");
                return Err(Error::ProtocolNotFound(
                    "ZwlrScreencopy Manager not found".to_string(),
                ));
            }
        };

        // Capture output.
        let frame: ZwlrScreencopyFrameV1 = if let Some(region) = capture_region {
            screencopy_manager.capture_output_region(
                cursor_overlay,
                output,
                region.x_coordinate,
                region.y_coordinate,
                region.width,
                region.height,
                &qh,
                (),
            )
        } else {
            screencopy_manager.capture_output(cursor_overlay, output, &qh, ())
        };

        // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
        // event is fired, aka the capture from the compositor is succesful.
        while !state.buffer_done.load(Ordering::SeqCst) {
            event_queue.blocking_dispatch(&mut state)?;
        }

        tracing::debug!(
            "Received compositor frame buffer formats: {:#?}",
            state.formats
        );
        // Filter advertised wl_shm formats and select the first one that matches.
        let frame_format = state
            .formats
            .iter()
            .find(|frame| {
                matches!(
                    frame.format,
                    wl_shm::Format::Xbgr2101010
                        | wl_shm::Format::Abgr2101010
                        | wl_shm::Format::Argb8888
                        | wl_shm::Format::Xrgb8888
                        | wl_shm::Format::Xbgr8888
                )
            })
            .copied();
        tracing::debug!("Selected frame buffer format: {:#?}", frame_format);

        // Check if frame format exists.
        let frame_format = match frame_format {
            Some(format) => format,
            None => {
                tracing::error!("No suitable frame format found");
                return Err(Error::NoSupportedBufferFormat);
            }
        };
        Ok((state, event_queue, frame, frame_format))
    }

    fn capture_output_frame_inner<T: AsFd>(
        &self,
        mut state: CaptureFrameState,
        mut event_queue: EventQueue<CaptureFrameState>,
        frame: ZwlrScreencopyFrameV1,
        frame_format: FrameFormat,
        fd: T,
    ) -> Result<FrameFormat> {
        // Connecting to wayland environment.
        let qh = event_queue.handle();

        // Bytes of data in the frame = stride * height.
        let frame_bytes = frame_format.stride * frame_format.height;

        // Instantiate shm global.
        let shm = self.globals.bind::<WlShm, _, _>(&qh, 1..=1, ()).unwrap();
        let shm_pool = shm.create_pool(fd.as_fd(), frame_bytes as i32, &qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            frame_format.width as i32,
            frame_format.height as i32,
            frame_format.stride as i32,
            frame_format.format,
            &qh,
            (),
        );

        // Copy the pixel data advertised by the compositor into the buffer we just created.
        frame.copy(&buffer);
        // On copy the Ready / Failed events are fired by the frame object, so here we check for them.
        loop {
            // Basically reads, if frame state is not None then...
            if let Some(state) = state.state {
                match state {
                    FrameState::Failed => {
                        tracing::error!("Frame copy failed");
                        return Err(Error::FramecopyFailed);
                    }
                    FrameState::Finished => {
                        buffer.destroy();
                        shm_pool.destroy();
                        return Ok(frame_format);
                    }
                }
            }

            event_queue.blocking_dispatch(&mut state)?;
        }
    }

    fn capture_output_frame_shm_from_file(
        &self,
        cursor_overlay: bool,
        output: &WlOutput,
        file: &File,
        capture_region: Option<CaptureRegion>,
    ) -> Result<FrameFormat> {
        let (state, event_queue, frame, frame_format) =
            self.capture_output_frame_get_state(cursor_overlay as i32, output, capture_region)?;

        // Bytes of data in the frame = stride * height.
        let frame_bytes = frame_format.stride * frame_format.height;
        file.set_len(frame_bytes as u64)?;

        self.capture_output_frame_inner(state, event_queue, frame, frame_format, file)
    }

    /// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
    fn capture_output_frame(
        &self,
        cursor_overlay: bool,
        output: &WlOutput,
        transform: Transform,
        capture_region: Option<CaptureRegion>,
    ) -> Result<FrameCopy> {
        // Create an in memory file and return it's file descriptor.
        let fd = create_shm_fd()?;
        // Create a writeable memory map backed by a mem_file.
        let mem_file = File::from(fd);

        let frame_format = self.capture_output_frame_shm_from_file(
            cursor_overlay,
            output,
            &mem_file,
            capture_region,
        )?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file)? };
        let data = &mut *frame_mmap;
        let frame_color_type = if let Some(converter) = create_converter(frame_format.format) {
            converter.convert_inplace(data)
        } else {
            tracing::error!("Unsupported buffer format: {:?}", frame_format.format);
            tracing::error!("You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot.");
            return Err(Error::NoSupportedBufferFormat);
        };
        Ok(FrameCopy {
            frame_format,
            frame_color_type,
            frame_mmap,
            transform,
        })
    }

    fn create_frame_copy(
        &self,
        capture_region: CaptureRegion,
        cursor_overlay: bool,
    ) -> Result<Frame> {
        let frame_copies = thread::scope(|scope| -> Result<_> {
            let join_handles = self
                .get_all_outputs()
                .into_iter()
                .filter_map(|output| {
                    let x1: i32 = cmp::max(output.dimensions.x, capture_region.x_coordinate);
                    let y1: i32 = cmp::max(output.dimensions.y, capture_region.y_coordinate);
                    let x2: i32 = cmp::min(
                        output.dimensions.x + output.dimensions.width,
                        capture_region.x_coordinate + capture_region.width,
                    );
                    let y2: i32 = cmp::min(
                        output.dimensions.y + output.dimensions.height,
                        capture_region.y_coordinate + capture_region.height,
                    );

                    let width = x2 - x1;
                    let height = y2 - y1;

                    if width <= 0 || height <= 0 {
                        return None;
                    }

                    let true_x = capture_region.x_coordinate - output.dimensions.x;
                    let true_y = capture_region.y_coordinate - output.dimensions.y;
                    let true_region = CaptureRegion {
                        x_coordinate: true_x,
                        y_coordinate: true_y,
                        width: capture_region.width,
                        height: capture_region.height,
                    };
                    Some(IntersectingOutput {
                        output: output.wl_output.clone(),
                        region: true_region,
                        transform: output.transform,
                    })
                })
                .map(|intersecting_output| {
                    scope.spawn(move || {
                        self.capture_output_frame(
                            cursor_overlay,
                            &intersecting_output.output,
                            intersecting_output.transform,
                            Some(intersecting_output.region),
                        )
                    })
                })
                .collect::<Vec<_>>();

            join_handles
                .into_iter()
                .map(|join_handle| join_handle.join())
                .flatten()
                .collect::<Result<_>>()
        })?;

        Ok((frame_copies, (capture_region.width, capture_region.height)))
    }

    /// Take a screenshot from the specified region.
    pub fn screenshot(
        &self,
        capture_region: CaptureRegion,
        cursor_overlay: bool,
    ) -> Result<RgbaImage> {
        let (frame_copies, (width, height)) =
            self.create_frame_copy(capture_region, cursor_overlay)?;

        thread::scope(|scope| {
            let rotate_join_handles = frame_copies
                .into_iter()
                .map(|frame_copy| {
                    scope.spawn(move || {
                        let transform = frame_copy.transform;
                        let image = frame_copy.try_into()?;
                        Ok(image_util::rotate_image_buffer(
                            image,
                            transform,
                            width as u32,
                            height as u32,
                        ))
                    })
                })
                .collect::<Vec<_>>();

            rotate_join_handles
                .into_iter()
                .map(|join_handle| join_handle.join())
                .flatten()
                .fold(
                    None,
                    |possible_overlayed_image_or_error: Option<Result<_>>, image: Result<_>| {
                        if let Some(overlayed_image_or_error) = possible_overlayed_image_or_error {
                            if let Ok(mut overlayed_image) = overlayed_image_or_error {
                                if let Ok(image) = image {
                                    overlay(&mut overlayed_image, &image, 0, 0);
                                    Some(Ok(overlayed_image))
                                } else {
                                    Some(image)
                                }
                            } else {
                                Some(image)
                            }
                        } else {
                            Some(image)
                        }
                    },
                )
                .ok_or_else(|| {
                    tracing::error!("Provided capture region doesn't intersect with any outputs!");
                    Error::NoOutputs
                })?
        })
    }

    /// shot one ouput
    pub fn screenshot_single_output(
        &self,
        output_info: &OutputInfo,
        cursor_overlay: bool,
    ) -> Result<RgbaImage> {
        let frame_copy = self.capture_output_frame(
            cursor_overlay,
            &output_info.wl_output,
            output_info.transform,
            None,
        )?;
        frame_copy.try_into()
    }

    /// Take a screenshot from all of the specified outputs.
    pub fn screenshot_outputs(
        &self,
        outputs: &Vec<OutputInfo>,
        cursor_overlay: bool,
    ) -> Result<RgbaImage> {
        if outputs.is_empty() {
            return Err(Error::NoOutputs);
        }

        let x1 = outputs
            .iter()
            .map(|output| output.dimensions.x)
            .min()
            .unwrap();
        let y1 = outputs
            .iter()
            .map(|output| output.dimensions.y)
            .min()
            .unwrap();
        let x2 = outputs
            .iter()
            .map(|output| output.dimensions.x + output.dimensions.width)
            .max()
            .unwrap();
        let y2 = outputs
            .iter()
            .map(|output| output.dimensions.y + output.dimensions.height)
            .max()
            .unwrap();
        let capture_region = CaptureRegion {
            x_coordinate: x1,
            y_coordinate: y1,
            width: x2 - x1,
            height: y2 - y1,
        };
        self.screenshot(capture_region, cursor_overlay)
    }

    /// Take a screenshot from all accessible outputs.
    pub fn screenshot_all(&self, cursor_overlay: bool) -> Result<RgbaImage> {
        self.screenshot_outputs(self.get_all_outputs(), cursor_overlay)
    }
}
