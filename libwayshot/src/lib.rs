//! `libwayshot` is a convenient wrapper over the wlroots screenshot protocol
//! that provides a simple API to take screenshots with.
//!
//! To get started, look at [`WayshotConnection`].

mod convert;
mod dispatch;
pub mod error;
pub mod ext_image_protocols;
mod image_util;
pub mod output;
pub mod region;
mod screencopy;

use dispatch::{DMABUFState, XdgShellState};
use image::{DynamicImage, imageops::replace};
use khronos_egl::{self as egl, Instance};
use memmap2::MmapMut;
use region::{EmbeddedRegion, RegionCapturer};
use screencopy::{DMAFrameFormat, DMAFrameGuard, EGLImageGuard, FrameData, FrameGuard};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::{
    ffi::c_void,
    fs::File,
    os::fd::{AsFd, IntoRawFd, OwnedFd},
    sync::atomic::{AtomicBool, Ordering},
    thread,
};
use tracing::debug;
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle,
    globals::{GlobalList, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{Transform, WlOutput},
        wl_shm::{self, WlShm},
    },
};
use wayland_protocols::{
    wp::{
        linux_dmabuf::zv1::client::{
            zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        },
        viewporter::client::wp_viewporter::WpViewporter,
    },
    xdg::xdg_output::zv1::client::{
        zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
    },
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::{
    convert::create_converter,
    dispatch::{CaptureFrameState, FrameState, OutputCaptureState, WayshotState},
    output::OutputInfo,
    region::{LogicalRegion, Size},
    screencopy::{FrameCopy, FrameFormat, create_shm_fd},
};

pub use crate::error::{Result, WayshotError};

pub mod reexport {
    use wayland_client::protocol::wl_output;
    pub use wl_output::{Transform, WlOutput};
}
use crate::ext_image_protocols::{AreaSelectCallback, CaptureInfo, CaptureOption, FrameInfo, ImageViewInfo, TopLevel};
use gbm::{BufferObject, BufferObjectFlags, Device as GBMDevice};
use wayland_backend::protocol::WEnum;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;
use wayland_protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1;
use wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use crate::region::Region;

/// Struct to store wayland connection and globals list.
/// # Example usage
///
/// ```ignore
/// use libwayshot::WayshotConnection;
/// let wayshot_connection = WayshotConnection::new()?;
/// let image_buffer = wayshot_connection.screenshot_all()?;
/// ```

#[derive(Debug)]
pub struct ExtBase<T> {
    pub toplevels: Vec<TopLevel>,
    pub img_copy_manager: Option<ExtImageCopyCaptureManagerV1>,
    pub output_image_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    pub shm: Option<WlShm>,
    pub qh: Option<QueueHandle<T>>,
    pub event_queue: Option<EventQueue<T>>,
}

#[derive(Debug)]
pub struct WayshotConnection {
    pub conn: Connection,
    pub globals: GlobalList,
    pub output_infos: Vec<OutputInfo>,
    dmabuf_state: Option<DMABUFState>,
    pub ext_image: Option<ExtBase<Self>>,
}

impl WayshotConnection {
    pub fn new() -> Result<
        Self,
    > {
        // Try to use ext_image protocol first
        match Self::create_connection(None, true) {
            Ok(connection) => {
                tracing::debug!("Successfully created connection with ext_image protocol");
                Ok(connection)
            }
            Err(err) => {
                tracing::debug!(
                    "ext_image protocol not available ({}), falling back to wlr-screencopy",
                    err
                );
                // Fall back to wlr_screencopy
                Self::create_connection(None, false)
            }
        }
    }

    /// Recommended if you already have a [`wayland_client::Connection`].
    /// Internal function that handles connection creation with protocol selection
    fn create_connection(
        connection: Option<Connection>,
        use_ext_image: bool,
    ) -> Result<Self, WayshotError> {
        let conn = if let Some(conn) = connection {
            conn
        } else {
            Connection::connect_to_env()?
        };

        let (globals, mut event_queue) = registry_queue_init::<WayshotConnection>(&conn)?;

        let mut initial_state = Self {
            conn,
            globals,
            output_infos: Vec::new(),
            dmabuf_state: None,
            ext_image: if use_ext_image {
                Some(ExtBase {
                    toplevels: Vec::new(),
                    img_copy_manager: None,
                    output_image_manager: None,
                    shm: None,
                    qh: None,
                    event_queue: None,
                })
            } else {
                None
            },
        };

        // Refresh outputs
        initial_state.refresh_outputs()?;

        // If using ext_image protocol, initialize the specific components
        if use_ext_image {
            let qh = event_queue.handle();

            // Bind to ext_image specific globals
            match initial_state
                .globals
                .bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())
            {
                Ok(image_manager) => {
                    match initial_state
                        .globals
                        .bind::<ExtOutputImageCaptureSourceManagerV1, _, _>(&qh, 1..=1, ())
                    {
                        Ok(output_image_manager) => {
                            match initial_state.globals.bind::<WlShm, _, _>(&qh, 1..=2, ()) {
                                Ok(shm) => {
                                    // Try to bind to toplevel list, but doen't fail if not available
                                    let _ = initial_state
                                        .globals
                                        .bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ());

                                    // Process events to ensure all bound globals are initialized
                                    event_queue.blocking_dispatch(&mut initial_state)?;

                                    // Store the globals we fetched
                                    if let Some(ext_image) = initial_state.ext_image.as_mut() {
                                        ext_image.img_copy_manager = Some(image_manager);
                                        ext_image.output_image_manager = Some(output_image_manager);
                                        ext_image.qh = Some(qh);
                                        ext_image.shm = Some(shm);
                                        ext_image.event_queue = Some(event_queue);
                                    }
                                }
                                Err(_) => {
                                    return Err(WayshotError::ProtocolNotFound(
                                        "WlShm not found".to_string(),
                                    ));
                                }
                            }
                        }
                        Err(_) => {
                            return Err(WayshotError::ProtocolNotFound(
                                "ExtOutputImageCaptureSourceManagerV1 not found".to_string(),
                            ));
                        }
                    }
                }
                Err(_) => {
                    return Err(WayshotError::ProtocolNotFound(
                        "ExtImageCopyCaptureManagerV1 not found".to_string(),
                    ));
                }
            }
        }

        Ok(initial_state)
    }

    ///Create a WayshotConnection struct having DMA-BUF support
    /// Using this connection is required to make use of the dmabuf functions
    ///# Parameters
    /// - conn: a Wayland connection
    /// - device_path: string pointing to the DRI device that is to be used for creating the DMA-BUFs on. For example: "/dev/dri/renderD128"
    pub fn from_connection_with_dmabuf(conn: Connection, device_path: &str) -> Result<Self> {
        let (globals, evq) = registry_queue_init::<WayshotState>(&conn)?;
        let linux_dmabuf =
            globals.bind(&evq.handle(), 4..=ZwpLinuxDmabufV1::interface().version, ())?;
        let gpu = dispatch::Card::open(device_path);
        // init a GBM device
        let gbm = GBMDevice::new(gpu).unwrap();
        let mut initial_state = Self {
            conn,
            globals,
            output_infos: Vec::new(),
            dmabuf_state: Some(DMABUFState {
                linux_dmabuf,
                gbmdev: gbm,
            }),
            ext_image: None,
        };

        initial_state.refresh_outputs()?;

        Ok(initial_state)
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
                tracing::error!(
                    "Failed to create ZxdgOutputManagerV1 version 3. Does your compositor implement ZxdgOutputManagerV1?"
                );
                panic!("{:#?}", e);
            }
        };

        // Fetch all outputs; when their names arrive, add them to the list
        let _ = self.conn.display().get_registry(&qh, ());
        event_queue.roundtrip(&mut state)?;

        // We loop over each output and request its position data.
        let xdg_outputs: Vec<ZxdgOutputV1> = state
            .outputs
            .iter()
            .enumerate()
            .map(|(index, output)| zxdg_output_manager.get_xdg_output(&output.output, &qh, index))
            .collect();

        event_queue.roundtrip(&mut state)?;

        for xdg_output in xdg_outputs {
            xdg_output.destroy();
        }

        if state.outputs.is_empty() {
            tracing::error!("Compositor did not advertise any wl_output devices!");
            return Err(WayshotError::NoOutputs);
        }
        tracing::trace!("Outputs detected: {:#?}", state.outputs);
        self.output_infos = state.outputs;

        Ok(())
    }

    /// Fetch all accessible wayland outputs.
    pub fn get_all_outputs(&self) -> &[OutputInfo] {
        self.output_infos.as_slice()
    }

    /// print the displays' info
    pub fn print_displays_info(&self) {
        for OutputInfo {
            physical_size: Size { width, height },
            logical_region:
                LogicalRegion {
                    inner:
                        region::Region {
                            position: region::Position { x, y },
                            size:
                                Size {
                                    width: logical_width,
                                    height: logical_height,
                                },
                        },
                },
            name,
            description,
            scale,
            ..
        } in self.get_all_outputs()
        {
            println!("{name}");
            println!("description: {description}");
            println!("    Size: {width},{height}");
            println!("    LogicSize: {logical_width}, {logical_height}");
            println!("    Position: {x}, {y}");
			println!("    Scale: {scale}");
        }
    }

    /// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
    ///  Data will be written to fd.
    pub fn capture_output_frame_shm_fd<T: AsFd>(
        &self,
        cursor_overlay: i32,
        output: &WlOutput,
        fd: T,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(FrameFormat, FrameGuard)> {
        let (state, event_queue, frame, frame_format) =
            self.capture_output_frame_get_state_shm(cursor_overlay, output, capture_region)?;
        let frame_guard =
            self.capture_output_frame_inner(state, event_queue, frame, frame_format, fd)?;

        Ok((frame_format, frame_guard))
    }

    fn capture_output_frame_shm_from_file(
        &self,
        cursor_overlay: bool,
        output: &WlOutput,
        file: &File,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(FrameFormat, FrameGuard)> {
        let (state, event_queue, frame, frame_format) =
            self.capture_output_frame_get_state_shm(cursor_overlay as i32, output, capture_region)?;

        file.set_len(frame_format.byte_size())?;

        let frame_guard =
            self.capture_output_frame_inner(state, event_queue, frame, frame_format, file)?;

        Ok((frame_format, frame_guard))
    }
    /// # Safety
    ///
    /// Helper function/wrapper that uses the OpenGL extension OES_EGL_image to convert the EGLImage obtained from [`WayshotConnection::capture_output_frame_eglimage`]
    /// into a OpenGL texture.
    /// - The caller is supposed to setup everything required for the texture binding. An example call may look like:
    /// ```no_run, ignore
    /// gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);
    /// gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    /// wayshot_conn
    ///     .bind_output_frame_to_gl_texture(
    ///         true,
    ///        &wayshot_conn.get_all_outputs()[0].wl_output,
    ///        None)
    ///```
    /// # Parameters
    /// - `cursor_overlay`: A boolean flag indicating whether the cursor should be included in the capture.
    /// - `output`: Reference to the `WlOutput` from which the frame is to be captured.
    /// - `capture_region`: Optional region specifying a sub-area of the output to capture. If `None`, the entire output is captured.
    /// # Returns
    /// - If the function was found and called, an OK(()), note that this does not necessarily mean that binding was successful, only that the function was called.
    ///   The caller may check for any OpenGL errors using the standard routes.
    /// - If the function was not found, [`WayshotError::EGLImageToTexProcNotFoundError`] is returned
    pub unsafe fn bind_output_frame_to_gl_texture(
        &self,
        cursor_overlay: bool,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<()> {
        let egl = khronos_egl::Instance::new(egl::Static);
        let eglimage_guard =
            self.capture_output_frame_eglimage(&egl, cursor_overlay, output, capture_region)?;
        unsafe {
            let gl_egl_image_texture_target_2d_oes: unsafe extern "system" fn(
                target: gl::types::GLenum,
                image: gl::types::GLeglImageOES,
            ) -> () =
                std::mem::transmute(match egl.get_proc_address("glEGLImageTargetTexture2DOES") {
                    Some(f) => {
                        tracing::debug!("glEGLImageTargetTexture2DOES found at address {:#?}", f);
                        f
                    }
                    None => {
                        tracing::error!("glEGLImageTargetTexture2DOES not found");
                        return Err(WayshotError::EGLImageToTexProcNotFoundError);
                    }
                });

            gl_egl_image_texture_target_2d_oes(gl::TEXTURE_2D, eglimage_guard.image.as_ptr());
            tracing::trace!("glEGLImageTargetTexture2DOES called");
            Ok(())
        }
    }

    /// Obtain a screencapture in the form of a EGLImage.
    /// The display on which this image is created is obtained from the Wayland Connection.
    /// Uses the dma-buf provisions of the wlr-screencopy copy protocol to avoid VRAM->RAM copies
    /// It returns the captured frame as an `EGLImage`, wrapped in an `EGLImageGuard`
    /// for safe handling and cleanup.
    /// # Parameters
    /// - `egl_instance`: Reference to an egl API instance obtained from the khronos_egl crate, which is used to create the `EGLImage`.
    /// - `cursor_overlay`: A boolean flag indicating whether the cursor should be included in the capture.
    /// - `output`: Reference to the `WlOutput` from which the frame is to be captured.
    /// - `capture_region`: Optional region specifying a sub-area of the output to capture. If `None`, the entire output is captured.
    ///
    /// # Returns
    /// If successful, an EGLImageGuard which contains a pointer 'image' to the created EGLImage
    /// On error, the EGL [error code](https://registry.khronos.org/EGL/sdk/docs/man/html/eglGetError.xhtml) is returned via this crates Error type
    pub fn capture_output_frame_eglimage<'a, T: khronos_egl::api::EGL1_5>(
        &self,
        egl_instance: &'a Instance<T>,
        cursor_overlay: bool,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<EGLImageGuard<'a, T>> {
        let egl_display = unsafe {
            match egl_instance.get_display(self.conn.display().id().as_ptr() as *mut c_void) {
                Some(disp) => disp,
                None => return Err(egl_instance.get_error().unwrap().into()),
            }
        };
        tracing::trace!("eglDisplay obtained from Wayland connection's display");

        egl_instance.initialize(egl_display)?;
        self.capture_output_frame_eglimage_on_display(
            egl_instance,
            egl_display,
            cursor_overlay,
            output,
            capture_region,
        )
    }

    /// Obtain a screencapture in the form of a EGLImage on the given EGLDisplay.
    ///
    /// Uses the dma-buf provisions of the wlr-screencopy copy protocol to avoid VRAM->RAM copies
    /// It returns the captured frame as an `EGLImage`, wrapped in an `EGLImageGuard`
    /// for safe handling and cleanup.
    /// # Parameters
    /// - `egl_instance`: Reference to an `EGL1_5` instance, which is used to create the `EGLImage`.
    /// - `egl_display`: The `EGLDisplay` on which the image should be created.
    /// - `cursor_overlay`: A boolean flag indicating whether the cursor should be included in the capture.
    /// - `output`: Reference to the `WlOutput` from which the frame is to be captured.
    /// - `capture_region`: Optional region specifying a sub-area of the output to capture. If `None`, the entire output is captured.
    ///
    /// # Returns
    /// If successful, an EGLImageGuard which contains a pointer 'image' to the created EGLImage
    /// On error, the EGL [error code](https://registry.khronos.org/EGL/sdk/docs/man/html/eglGetError.xhtml) is returned via this crates Error type
    pub fn capture_output_frame_eglimage_on_display<'a, T: khronos_egl::api::EGL1_5>(
        &self,
        egl_instance: &'a Instance<T>,
        egl_display: egl::Display,
        cursor_overlay: bool,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<EGLImageGuard<'a, T>> {
        type Attrib = egl::Attrib;
        let (frame_format, _guard, bo) =
            self.capture_output_frame_dmabuf(cursor_overlay, output, capture_region)?;
        let modifier: u64 = bo.modifier().into();
        let image_attribs = [
            egl::WIDTH as Attrib,
            frame_format.size.width as Attrib,
            egl::HEIGHT as Attrib,
            frame_format.size.height as Attrib,
            0x3271, //EGL_LINUX_DRM_FOURCC_EXT
            bo.format() as Attrib,
            0x3272, //EGL_DMA_BUF_PLANE0_FD_EXT
            bo.fd_for_plane(0).unwrap().into_raw_fd() as Attrib,
            0x3273, //EGL_DMA_BUF_PLANE0_OFFSET_EXT
            bo.offset(0) as Attrib,
            0x3274, //EGL_DMA_BUF_PLANE0_PITCH_EXT
            bo.stride_for_plane(0) as Attrib,
            0x3443, //EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT
            (modifier as u32) as Attrib,
            0x3444, //EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT
            (modifier >> 32) as Attrib,
            egl::ATTRIB_NONE as Attrib,
        ];
        tracing::debug!(
            "Calling eglCreateImage with attributes: {:#?}",
            image_attribs
        );
        unsafe {
            match egl_instance.create_image(
                egl_display,
                khronos_egl::Context::from_ptr(egl::NO_CONTEXT),
                0x3270, // EGL_LINUX_DMA_BUF_EXT
                khronos_egl::ClientBuffer::from_ptr(std::ptr::null_mut()), //NULL
                &image_attribs,
            ) {
                Ok(image) => Ok(EGLImageGuard {
                    image,
                    egl_instance,
                    egl_display,
                }),
                Err(e) => {
                    tracing::error!("eglCreateImage call failed with error {e}");
                    Err(e.into())
                }
            }
        }
    }

    /// Obtain a screencapture in the form of a WlBuffer backed by a GBM Bufferobject on the GPU.
    /// Uses the dma-buf provisions of the wlr-screencopy copy protocol to avoid VRAM->RAM copies
    /// The captured frame is returned as a tuple containing the frame format, a guard to manage
    /// the WlBuffer's cleanup on drop, and the underlying `BufferObject`.
    /// - `cursor_overlay`: A boolean flag indicating whether the cursor should be included in the capture.
    /// - `output`: Reference to the `WlOutput` from which the frame is to be captured.
    /// - `capture_region`: Optional region specifying a sub-area of the output to capture. If `None`, the entire output is captured.
    ///# Returns
    /// On success, returns a tuple containing the frame format,
    ///   a guard to manage the frame's lifecycle, and the GPU-backed `BufferObject`.
    /// # Errors
    /// - Returns `NoDMAStateError` if the DMA-BUF state is not initialized a the time of initialization of this struct.
    pub fn capture_output_frame_dmabuf(
        &self,
        cursor_overlay: bool,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(DMAFrameFormat, DMAFrameGuard, BufferObject<()>)> {
        match &self.dmabuf_state {
            Some(dmabuf_state) => {
                let (state, event_queue, frame, frame_format) = self
                    .capture_output_frame_get_state_dmabuf(
                        cursor_overlay as i32,
                        output,
                        capture_region,
                    )?;
                let gbm = &dmabuf_state.gbmdev;
                let bo = gbm.create_buffer_object::<()>(
                    frame_format.size.width,
                    frame_format.size.height,
                    gbm::Format::try_from(frame_format.format)?,
                    BufferObjectFlags::RENDERING | BufferObjectFlags::LINEAR,
                )?;

                let stride = bo.stride();
                let modifier: u64 = bo.modifier().into();
                tracing::debug!(
                    "Created GBM Buffer object with input frame format {:#?}, stride {:#?} and modifier {:#?} ",
                    frame_format,
                    stride,
                    modifier
                );
                let frame_guard = self.capture_output_frame_inner_dmabuf(
                    state,
                    event_queue,
                    frame,
                    frame_format,
                    stride,
                    modifier,
                    bo.fd_for_plane(0).unwrap(),
                )?;

                Ok((frame_format, frame_guard, bo))
            }
            None => Err(WayshotError::NoDMAStateError),
        }
    }

    // This API is exposed to provide users with access to window manager (WM)
    // information. For instance, enabling Vulkan in wlroots alters the display
    // format. Consequently, using PipeWire to capture streams without knowing
    // the current format can lead to color distortion. This function attempts
    // a trial screenshot to determine the screen's properties.
    pub fn capture_output_frame_get_state_shm(
        &self,
        cursor_overlay: i32,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(
        CaptureFrameState,
        EventQueue<CaptureFrameState>,
        ZwlrScreencopyFrameV1,
        FrameFormat,
    )> {
        let mut state = CaptureFrameState {
            formats: Vec::new(),
            dmabuf_formats: Vec::new(),
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
                tracing::error!(
                    "Failed to create screencopy manager. Does your compositor implement ZwlrScreencopy?"
                );
                tracing::error!("err: {e}");
                return Err(WayshotError::ProtocolNotFound(
                    "ZwlrScreencopy Manager not found".to_string(),
                ));
            }
        };

        tracing::debug!("Capturing output(shm buffer)...");
        let frame = if let Some(embedded_region) = capture_region {
            screencopy_manager.capture_output_region(
                cursor_overlay,
                output,
                embedded_region.inner.position.x,
                embedded_region.inner.position.y,
                embedded_region.inner.size.width as i32,
                embedded_region.inner.size.height as i32,
                &qh,
                (),
            )
        } else {
            screencopy_manager.capture_output(cursor_overlay, output, &qh, ())
        };

        // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
        // event is fired, aka the capture from the compositor is successful.
        while !state.buffer_done.load(Ordering::SeqCst) {
            event_queue.blocking_dispatch(&mut state)?;
        }

        tracing::trace!(
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
                        | wl_shm::Format::Bgr888
                )
            })
            .copied()
            // Check if frame format exists.
            .ok_or_else(|| {
                tracing::error!("No suitable frame format found");
                WayshotError::NoSupportedBufferFormat
            })?;
        tracing::trace!("Selected frame buffer format: {:#?}", frame_format);

        Ok((state, event_queue, frame, frame_format))
    }

    fn capture_output_frame_get_state_dmabuf(
        &self,
        cursor_overlay: i32,
        output: &WlOutput,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(
        CaptureFrameState,
        EventQueue<CaptureFrameState>,
        ZwlrScreencopyFrameV1,
        DMAFrameFormat,
    )> {
        let mut state = CaptureFrameState {
            formats: Vec::new(),
            dmabuf_formats: Vec::new(),
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
                tracing::error!(
                    "Failed to create screencopy manager. Does your compositor implement ZwlrScreencopy?"
                );
                tracing::error!("err: {e}");
                return Err(WayshotError::ProtocolNotFound(
                    "ZwlrScreencopy Manager not found".to_string(),
                ));
            }
        };

        tracing::debug!("Capturing output for DMA-BUF API...");
        let frame = if let Some(embedded_region) = capture_region {
            screencopy_manager.capture_output_region(
                cursor_overlay,
                output,
                embedded_region.inner.position.x,
                embedded_region.inner.position.y,
                embedded_region.inner.size.width as i32,
                embedded_region.inner.size.height as i32,
                &qh,
                (),
            )
        } else {
            screencopy_manager.capture_output(cursor_overlay, output, &qh, ())
        };

        // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
        // event is fired, aka the capture from the compositor is successful.
        while !state.buffer_done.load(Ordering::SeqCst) {
            event_queue.blocking_dispatch(&mut state)?;
        }

        tracing::trace!(
            "Received compositor frame buffer formats: {:#?}",
            state.formats
        );
        // TODO select appropriate format if there is more than one
        let frame_format = state.dmabuf_formats[0];
        tracing::trace!("Selected frame buffer format: {:#?}", frame_format);

        Ok((state, event_queue, frame, frame_format))
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_output_frame_inner_dmabuf(
        &self,
        mut state: CaptureFrameState,
        mut event_queue: EventQueue<CaptureFrameState>,
        frame: ZwlrScreencopyFrameV1,
        frame_format: DMAFrameFormat,
        stride: u32,
        modifier: u64,
        fd: OwnedFd,
    ) -> Result<DMAFrameGuard> {
        match &self.dmabuf_state {
            Some(dmabuf_state) => {
                println!("The program screenshoted via dmabuf");
                // Connecting to wayland environment.
                let qh = event_queue.handle();

                let linux_dmabuf = &dmabuf_state.linux_dmabuf;
                let dma_width = frame_format.size.width;
                let dma_height = frame_format.size.height;

                let dma_params = linux_dmabuf.create_params(&qh, ());

                dma_params.add(
                    fd.as_fd(),
                    0,
                    0,
                    stride,
                    (modifier >> 32) as u32,
                    (modifier & 0xffffffff) as u32,
                );
                tracing::trace!("Called  ZwpLinuxBufferParamsV1::create_params ");
                let dmabuf_wlbuf = dma_params.create_immed(
                    dma_width as i32,
                    dma_height as i32,
                    frame_format.format,
                    zwp_linux_buffer_params_v1::Flags::empty(),
                    &qh,
                    (),
                );
                tracing::trace!("Called  ZwpLinuxBufferParamsV1::create_immed to create WlBuffer ");
                // Copy the pixel data advertised by the compositor into the buffer we just created.
                frame.copy(&dmabuf_wlbuf);
                tracing::debug!("wlr-screencopy copy() with dmabuf complete");

                // On copy the Ready / Failed events are fired by the frame object, so here we check for them.
                loop {
                    // Basically reads, if frame state is not None then...
                    if let Some(state) = state.state {
                        match state {
                            FrameState::Failed(_) => {
                                tracing::error!("Frame copy failed");
                                return Err(WayshotError::FramecopyFailed);
                            }
                            FrameState::Succeeded => {
                                tracing::trace!("Frame copy finished");

                                return Ok(DMAFrameGuard {
                                    buffer: dmabuf_wlbuf,
                                });
                            }
                            FrameState::Pending => {
                                // If still pending, continue the event loop to wait for status change
                            }
                        }
                    }

                    event_queue.blocking_dispatch(&mut state)?;
                }
            }
            None => Err(WayshotError::NoDMAStateError),
        }
    }

    fn capture_output_frame_inner<T: AsFd>(
        &self,
        mut state: CaptureFrameState,
        mut event_queue: EventQueue<CaptureFrameState>,
        frame: ZwlrScreencopyFrameV1,
        frame_format: FrameFormat,
        fd: T,
    ) -> Result<FrameGuard> {
        // Connecting to wayland environment.
        println!("The program screenshoted via wlshm");
        let qh = event_queue.handle();

        // Instantiate shm global.
        let shm = self.globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        let shm_pool = shm.create_pool(
            fd.as_fd(),
            frame_format
                .byte_size()
                .try_into()
                .map_err(|_| WayshotError::BufferTooSmall)?,
            &qh,
            (),
        );
        let buffer = shm_pool.create_buffer(
            0,
            frame_format.size.width as i32,
            frame_format.size.height as i32,
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
                    FrameState::Failed(_) => {
                        tracing::error!("Frame copy failed");
                        return Err(WayshotError::FramecopyFailed);
                    }
                    FrameState::Succeeded => {
                        tracing::trace!("Frame copy finished");
                        return Ok(FrameGuard { buffer, shm_pool });
                    }
                    FrameState::Pending => {
                        // If still pending, continue the event loop to wait for status change
                    }
                }
            }

            event_queue.blocking_dispatch(&mut state)?;
        }
    }

    /// Get a FrameCopy instance with screenshot pixel data for any wl_output object.
    #[tracing::instrument(skip_all, fields(output = format!("{output_info}"), region = capture_region.map(|r| format!("{:}", r)).unwrap_or("fullscreen".to_string())))]
    fn capture_frame_copy(
        &self,
        cursor_overlay: bool,
        output_info: &OutputInfo,
        capture_region: Option<EmbeddedRegion>,
    ) -> Result<(FrameCopy, FrameGuard)> {
        // Create an in memory file and return it's file descriptor.
        let fd = create_shm_fd()?;
        // Create a writeable memory map backed by a mem_file.
        let mem_file = File::from(fd);

        let (frame_format, frame_guard) = self.capture_output_frame_shm_from_file(
            cursor_overlay,
            &output_info.output,
            &mem_file,
            capture_region,
        )?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file)? };
        let data = &mut *frame_mmap;
        let frame_color_type = match create_converter(frame_format.format) {
            Some(converter) => converter.convert_inplace(data),
            _ => {
                tracing::error!("Unsupported buffer format: {:?}", frame_format.format);
                tracing::error!(
                    "You can send a feature request for the above format to the mailing list for wayshot over at https://sr.ht/~shinyzenith/wayshot."
                );
                return Err(WayshotError::NoSupportedBufferFormat);
            }
        };
        let rotated_physical_size = match output_info.transform {
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
                Size {
                    width: frame_format.size.height,
                    height: frame_format.size.width,
                }
            }
            _ => frame_format.size,
        };
        let frame_copy = FrameCopy {
            frame_format,
            frame_color_type,
            frame_data: FrameData::Mmap(frame_mmap),
            transform: output_info.transform,
            logical_region: capture_region
                .map(|capture_region| capture_region.logical())
                .unwrap_or(output_info.logical_region),
            physical_size: rotated_physical_size,
        };
        tracing::debug!("Created frame copy: {:#?}", frame_copy);
        Ok((frame_copy, frame_guard))
    }

    pub fn capture_frame_copies(
        &self,
        output_capture_regions: &[(OutputInfo, Option<EmbeddedRegion>)],
        cursor_overlay: bool,
    ) -> Result<Vec<(FrameCopy, FrameGuard, OutputInfo)>> {
        output_capture_regions
            .iter()
            .map(|(output_info, capture_region)| {
                self.capture_frame_copy(cursor_overlay, output_info, *capture_region)
                    .map(|(frame_copy, frame_guard)| (frame_copy, frame_guard, output_info.clone()))
            })
            .collect()
    }

    /// Create a layer shell surface for each output,
    /// render the screen captures on them and use the callback to select a region from them
    fn overlay_frames_and_select_region<F>(
        &self,
        frames: &[(FrameCopy, FrameGuard, OutputInfo)],
        callback: F,
    ) -> Result<LogicalRegion>
    where
        F: Fn(&WayshotConnection) -> Result<LogicalRegion, WayshotError>,
    {
        let mut state = XdgShellState::new();
        let mut event_queue: EventQueue<XdgShellState> =
            self.conn.new_event_queue::<XdgShellState>();
        let qh = event_queue.handle();

        let compositor = match self.globals.bind::<WlCompositor, _, _>(&qh, 3..=3, ()) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!(
                    "Failed to create compositor. Does your compositor implement WlCompositor?"
                );
                tracing::error!("err: {e}");
                return Err(WayshotError::ProtocolNotFound(
                    "WlCompositor not found".to_string(),
                ));
            }
        };

        // Use XDG shell instead of layer shell
        let xdg_wm_base = match self
            .globals
            .bind::<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase, _, _>(
            &qh,
            1..=1,
            (),
        ) {
            Ok(x) => x,
            Err(e) => {
                tracing::error!(
                    "Failed to create xdg_wm_base. Does your compositor implement XdgWmBase?"
                );
                tracing::error!("err: {e}");
                return Err(WayshotError::ProtocolNotFound(
                    "XdgWmBase not found".to_string(),
                ));
            }
        };

        let viewporter = self.globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();
        if viewporter.is_none() {
            tracing::info!(
                "Compositor does not support wp_viewporter, display scaling may be inaccurate."
            );
        }

        // Use a vector to store XDG surfaces instead of layer shell surfaces
        let mut xdg_surfaces = Vec::with_capacity(frames.len());

        for (frame_copy, frame_guard, output_info) in frames {
            tracing::span!(
                tracing::Level::DEBUG,
                "overlay_frames::surface",
                output = format!("{output_info}")
            )
            .in_scope(|| -> Result<()> {
                let surface = compositor.create_surface(&qh, ());

                // Create XDG surface and toplevel instead of layer shell surface
                let xdg_surface =
                    xdg_wm_base.get_xdg_surface(&surface, &qh, output_info.output.clone());
                let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());

                // Configure the toplevel to be fullscreen on the specific output
                xdg_toplevel.set_fullscreen(Some(&output_info.output));
                xdg_toplevel.set_title("wayshot-overlay".to_string());
                xdg_toplevel.set_app_id("wayshot".to_string());

                debug!("Committing surface creation changes.");
                surface.commit();

                debug!("Waiting for XDG surface to be configured.");
                while !state.configured_surfaces.contains(&xdg_surface) {
                    event_queue.blocking_dispatch(&mut state)?;
                }

                surface.set_buffer_transform(output_info.transform);
                surface.attach(Some(&frame_guard.buffer), 0, 0);

                if let Some(viewporter) = viewporter.as_ref() {
                    let viewport = viewporter.get_viewport(&surface, &qh, ());
                    viewport.set_destination(
                        output_info.logical_region.inner.size.width as i32,
                        output_info.logical_region.inner.size.height as i32,
                    );
                }

                debug!("Committing surface with attached buffer.");
                surface.commit();
                xdg_surfaces.push((surface, xdg_surface, xdg_toplevel));
                event_queue.blocking_dispatch(&mut state)?;

                Ok(())
            })?;
        }

        let callback_result = callback(self);

        debug!("Unmapping and destroying XDG shell surfaces.");
        for (surface, xdg_surface, xdg_toplevel) in xdg_surfaces.iter() {
            surface.attach(None, 0, 0);
            surface.commit(); // unmap surface by committing a null buffer
            xdg_toplevel.destroy();
            xdg_surface.destroy();
        }
        event_queue.roundtrip(&mut state)?;

        callback_result
    }

    /// Take a screenshot from the specified region.
    #[tracing::instrument(skip_all, fields(max_scale = tracing::field::Empty))]
    fn screenshot_region_capturer(
        &self,
        region_capturer: RegionCapturer,
        cursor_overlay: bool,
    ) -> Result<DynamicImage> {
        let outputs_capture_regions: Vec<(OutputInfo, Option<EmbeddedRegion>)> =
            match region_capturer {
                RegionCapturer::Outputs(ref outputs) => outputs
                    .iter()
                    .map(|output_info| (output_info.clone(), None))
                    .collect(),
                RegionCapturer::Region(capture_region) => self
                    .get_all_outputs()
                    .iter()
                    .filter_map(|output_info| {
                        tracing::span!(
                            tracing::Level::DEBUG,
                            "filter_map",
                            output = format!(
                                "{output_info} at {region}",
                                output_info = format!("{output_info}"),
                                region = LogicalRegion::from(output_info),
                            ),
                            capture_region = format!("{}", capture_region),
                        )
                        .in_scope(|| {
                            if let Some(relative_region) =
                                EmbeddedRegion::new(capture_region, output_info.into())
                            {
                                tracing::debug!("Intersection found: {}", relative_region);
                                Some((output_info.clone(), Some(relative_region)))
                            } else {
                                tracing::debug!("No intersection found");
                                None
                            }
                        })
                    })
                    .collect(),
                RegionCapturer::Freeze(_) => self
                    .get_all_outputs()
                    .iter()
                    .map(|output_info| (output_info.clone(), None))
                    .collect(),
            };

        let frames = self.capture_frame_copies(&outputs_capture_regions, cursor_overlay)?;

        let capture_region: LogicalRegion = match region_capturer {
            RegionCapturer::Outputs(outputs) => outputs.as_slice().try_into()?,
            RegionCapturer::Region(region) => region,
            RegionCapturer::Freeze(callback) => {
                self.overlay_frames_and_select_region(&frames, callback)?
            }
        };

        // TODO When freeze was used, we can still further remove the outputs
        // that don't intersect with the capture region.

        thread::scope(|scope| {
            let max_scale = outputs_capture_regions
                .iter()
                .map(|(output_info, _)| output_info.scale as f64)
                .fold(1.0, f64::max);

            tracing::Span::current().record("max_scale", max_scale);

            let rotate_join_handles = frames
                .into_iter()
                .map(|(frame_copy, _, _)| {
                    scope.spawn(move || {
                        let image = (&frame_copy).try_into()?;
                        Ok((
                            image_util::rotate_image_buffer(
                                image,
                                frame_copy.transform,
                                frame_copy.logical_region.inner.size,
                                max_scale,
                            ),
                            frame_copy,
                        ))
                    })
                })
                .collect::<Vec<_>>();

            rotate_join_handles
                .into_iter()
                .flat_map(|join_handle| join_handle.join())
                .fold(
                    None,
                    |composite_image: Option<Result<_>>, image: Result<_>| {
                        // Default to a transparent image.
                        let composite_image = composite_image.unwrap_or_else(|| {
                            Ok(DynamicImage::new_rgba8(
                                (capture_region.inner.size.width as f64 * max_scale) as u32,
                                (capture_region.inner.size.height as f64 * max_scale) as u32,
                            ))
                        });

                        Some(|| -> Result<_> {
                            let mut composite_image = composite_image?;
                            let (image, frame_copy) = image?;
                            let (x, y) = (
                                ((frame_copy.logical_region.inner.position.x as f64
                                    - capture_region.inner.position.x as f64)
                                    * max_scale) as i64,
                                ((frame_copy.logical_region.inner.position.y as f64
                                    - capture_region.inner.position.y as f64)
                                    * max_scale) as i64,
                            );
                            tracing::span!(
                                tracing::Level::DEBUG,
                                "replace",
                                frame_copy_region = format!("{}", frame_copy.logical_region),
                                capture_region = format!("{}", capture_region),
                                x = x,
                                y = y,
                            )
                            .in_scope(|| {
                                tracing::debug!("Replacing parts of the final image");
                                replace(&mut composite_image, &image, x, y);
                            });

                            Ok(composite_image)
                        }())
                    },
                )
                .ok_or_else(|| {
                    tracing::error!("Provided capture region doesn't intersect with any outputs!");
                    WayshotError::NoOutputs
                })?
        })
    }

    /// Take a screenshot from the specified region.
    pub fn screenshot(
        &self,
        capture_region: LogicalRegion,
        cursor_overlay: bool,
    ) -> Result<DynamicImage> {
        self.screenshot_region_capturer(RegionCapturer::Region(capture_region), cursor_overlay)
    }

    /// Take a screenshot, overlay the screenshot, run the callback, and then
    /// unfreeze the screenshot and return the selected region.
    pub fn screenshot_freeze<F>(&self, callback: F, cursor_overlay: bool) -> Result<DynamicImage>
    where
        F: Fn(&WayshotConnection) -> Result<LogicalRegion> + 'static,
    {
        self.screenshot_region_capturer(RegionCapturer::Freeze(Box::new(callback)), cursor_overlay)
    }

    /// Take a screenshot from one output
    pub fn screenshot_single_output(
        &self,
        output_info: &OutputInfo,
        cursor_overlay: bool,
    ) -> Result<DynamicImage> {
        let (frame_copy, _) = self.capture_frame_copy(cursor_overlay, output_info, None)?;
        (&frame_copy).try_into()
    }

    /// Take a screenshot from all of the specified outputs.
    pub fn screenshot_outputs(
        &self,
        outputs: &[OutputInfo],
        cursor_overlay: bool,
    ) -> Result<DynamicImage> {
        if outputs.is_empty() {
            return Err(WayshotError::NoOutputs);
        }

        self.screenshot_region_capturer(RegionCapturer::Outputs(outputs.to_owned()), cursor_overlay)
    }

    /// Take a screenshot from all accessible outputs.
    pub fn screenshot_all(&self, cursor_overlay: bool) -> Result<DynamicImage> {
        self.screenshot_outputs(self.get_all_outputs(), cursor_overlay)
    }
}

use wayland_client::protocol::{
    wl_shm::Format,
};

impl WayshotConnection {
    /// get all outputs and their info
    pub fn vector_of_Outputs(&self) -> &Vec<OutputInfo> {
        &self.output_infos
    }

    pub(crate) fn reset_event_queue(&mut self, event_queue: EventQueue<Self>) {
        self.ext_image
            .as_mut()
            .expect("ext_image should be initialized")
            .event_queue = Some(event_queue);
    }
}

impl WayshotConnection {
    /// Capture a single output
    pub fn ext_capture_single_output(
        &mut self,
        option: CaptureOption,
        output: OutputInfo,
    ) -> std::result::Result<ImageViewInfo, WayshotError> {
        let mem_fd = ext_image_protocols::ext_create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);
        let ext_image_protocols::CaptureOutputData {
            width,
            height,
            frame_format,
            ..
        } = self.ext_capture_output_inner(
            output.clone(),
            option,
            mem_file.as_fd(),
            Some(&mem_file),
        )?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        let converter = create_converter(frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        // Create a full screen region representing the entire output
        let region = output.logical_region.inner.clone();

        Ok(ImageViewInfo {
            data: frame_mmap.deref().into(),
            width,
            height,
            color_type,
            region,
        })
    }

    fn ext_capture_output_inner<T: AsFd>(
        &mut self,
        OutputInfo {
            output,
            logical_region:
                LogicalRegion {
                    inner:
                        Region {
                            position: screen_position,
                            size:
                                Size {
                                    width: real_width,
                                    height: real_height,
                                },
                        },
                },
            ..
        }: OutputInfo,
        option: CaptureOption,
        fd: T,
        file: Option<&File>,
    ) -> std::result::Result<crate::ext_image_protocols::CaptureOutputData, WayshotError> {
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
        let info = Arc::new(RwLock::new(FrameInfo::default()));
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

        let Size { width, height } = info.size();
        let WEnum::Value(frame_format) = info.format() else {
            return Err(WayshotError::NotSupportFormat);
        };
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
                        FailureReason::Stopped => {
                            return Err(WayshotError::CaptureFailed("Stopped".to_owned()));
                        }

                        FailureReason::BufferConstraints => {
                            return Err(WayshotError::CaptureFailed(
                                "BufferConstraints".to_owned(),
                            ));
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

        Ok(crate::ext_image_protocols::CaptureOutputData {
            output,
            buffer,
            width,
            height,
            frame_bytes,
            stride,
            frame_format,
            real_width: real_width as u32,
            real_height: real_height as u32,
            transform,
            screen_position,
        })
    }

    pub fn ext_capture_area2<F>(
        &mut self,
        option: CaptureOption,
        callback: F,
    ) -> std::result::Result<ImageViewInfo, WayshotError>
    where
        F: AreaSelectCallback,
    {
        let outputs = self.vector_of_Outputs().clone();

        let mut data_list = vec![];
        for data in outputs.into_iter() {
            let mem_fd = crate::ext_image_protocols::ext_create_shm_fd().unwrap();
            let mem_file = File::from(mem_fd);
            let data =
                self.ext_capture_output_inner(data, option, mem_file.as_fd(), Some(&mem_file))?;
            data_list.push(crate::ext_image_protocols::AreaShotInfo { data, mem_file })
        }

        let mut state = XdgShellState::new();
        let mut event_queue: EventQueue<XdgShellState> = self.conn.new_event_queue();
        let globals = &self.globals;
        let qh = event_queue.handle();

        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 3..=3, ())?;
        let xdg_wm_base = globals.bind::<XdgWmBase, _, _>(&qh, 1..=1, ())?;
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ())?;

        let mut xdg_surfaces: Vec<(WlSurface, XdgSurface, XdgToplevel)> =
            Vec::with_capacity(data_list.len());
        for crate::ext_image_protocols::AreaShotInfo { data, .. } in data_list.iter() {
            let crate::ext_image_protocols::CaptureOutputData {
                output,
                buffer,
                real_width,
                real_height,
                transform,
                ..
            } = data;
            let surface = compositor.create_surface(&qh, ());

            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, output.clone());
            let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());

            // Configure the toplevel to be fullscreen on the specific output
            xdg_toplevel.set_fullscreen(Some(output));
            xdg_toplevel.set_title("wayshot-overlay".to_string());
            xdg_toplevel.set_app_id("wayshot".to_string());

            debug!("Committing surface creation changes.");
            surface.commit();

            debug!("Waiting for layer surface to be configured.");
            while !state.configured_surfaces.contains(&xdg_surface) {
                event_queue.blocking_dispatch(&mut state)?;
            }

            surface.set_buffer_transform(*transform);
            // surface.set_buffer_scale(output_info.scale());
            surface.attach(Some(buffer), 0, 0);

            let viewport = viewporter.get_viewport(&surface, &qh, ());
            viewport.set_destination(*real_width as i32, *real_height as i32);

            debug!("Committing surface with attached buffer.");
            surface.commit();
            xdg_surfaces.push((surface, xdg_surface, xdg_toplevel));
            event_queue.blocking_dispatch(&mut state)?;
        }

        let region_re = callback.Screenshot(self);

        debug!("Unmapping and destroying layer shell surfaces.");
        for (surface, xdg_surface, xdg_toplevel) in xdg_surfaces.iter() {
            surface.attach(None, 0, 0);
            surface.commit(); // unmap surface by committing a null buffer
            xdg_toplevel.destroy();
            xdg_surface.destroy();
        }
        event_queue.roundtrip(&mut state)?;
        let region = region_re?;

        let shotdata = data_list
            .iter()
            .find(|data| data.in_this_screen(region))
            .ok_or(WayshotError::CaptureFailed("not in region".to_owned()))?;
        let area = shotdata.clip_area(region).expect("should have");
        let mut frame_mmap = unsafe { MmapMut::map_mut(&shotdata.mem_file).unwrap() };

        let converter = crate::convert::create_converter(shotdata.data.frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageViewInfo {
            data: frame_mmap.deref().into(),
            width: shotdata.data.width,
            height: shotdata.data.height,
            color_type,
            region: area,
        })
    }
}












impl WayshotConnection {
    /// Creates a StreamingCaptureContext for efficient continuous capture of an output
    ///
    /// This method initializes all necessary resources for capturing frames from a specific output
    /// and returns a context that can be reused for multiple captures. This is much more efficient
    /// for streaming use cases than creating new resources for each capture.
    ///
    /// # Parameters
    /// - `option`: The capture options to use (e.g., whether to include cursor)
    /// - `output`: The output to capture from
    ///
    /// # Returns
    /// A `StreamingCaptureContext` that can be used with `capture_frame_with_context`
    pub fn create_streaming_context(
        &mut self,
        option: CaptureOption,
        output: OutputInfo,
    ) -> Result<crate::ext_image_protocols::StreamingCaptureContext, WayshotError> {
        // Create resources that will be reused across multiple captures
        let mem_fd = crate::ext_image_protocols::ext_create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);

        // Take ownership of components rather than borrowing self in multiple ways
        let mut event_queue = self
            .ext_image
            .as_mut()
            .expect("ext_image should be initialized")
            .event_queue
            .take()
            .expect("Control your self");
        let qh = {
            let ext_image = self
                .ext_image
                .as_ref()
                .expect("ext_image should be initialized");
            ext_image.qh.as_ref().expect("Should init").clone()
        };

        let img_manager = {
            let ext_image = self
                .ext_image
                .as_ref()
                .expect("ext_image should be initialized");
            ext_image
                .output_image_manager
                .as_ref()
                .expect("Should init")
        };

        let capture_manager = {
            let ext_image = self
                .ext_image
                .as_ref()
                .expect("ext_image should be initialized");
            ext_image.img_copy_manager.as_ref().expect("Should init")
        };

        // Create source and session - these will be reused
        let source = img_manager.create_source(&output.output, &qh, ());
        let info = Arc::new(RwLock::new(FrameInfo::default()));
        let session = capture_manager.create_session(&source, option.into(), &qh, info.clone());

        // Dispatch events to get buffer info - pass &mut *self to avoid borrowing issues
        {
            let mut_ref = &mut *self as *mut WayshotConnection;
            let result = event_queue.blocking_dispatch(unsafe { &mut *mut_ref });
            if let Err(e) = result {
                self.reset_event_queue(event_queue);
                // DispatchError is now properly converted to WayshotError
                return Err(e.into());
            }
        }

        // Create frame info
        let frame_info = info.read().unwrap();
        let Size { width, height } = frame_info.size();
        let WEnum::Value(frame_format) = frame_info.format() else {
            self.reset_event_queue(event_queue);
            return Err(WayshotError::NotSupportFormat);
        };

        if !matches!(
            frame_format,
            Format::Xbgr2101010
                | Format::Abgr2101010
                | Format::Argb8888
                | Format::Xrgb8888
                | Format::Xbgr8888
        ) {
            self.reset_event_queue(event_queue);
            return Err(WayshotError::NotSupportFormat);
        }

        let frame_bytes = 4 * height * width;
        let stride = 4 * width;

        // Set up the memory file
        mem_file.set_len(frame_bytes as u64).unwrap();

        // Create buffer resources
        let shm = {
            let ext_image = self
                .ext_image
                .as_ref()
                .expect("ext_image should be initialized");
            ext_image.shm.as_ref().expect("Should init")
        };

        let shm_pool = shm.create_pool(mem_file.as_fd(), (width * height * 4) as i32, &qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            frame_format,
            &qh,
            (),
        );

        // Reset event queue before returning
        self.reset_event_queue(event_queue);

        // Return context with all resources
        Ok(crate::ext_image_protocols::StreamingCaptureContext {
            source: Some(source),
            session: Some(session),
            frame: None, // Will be created for each capture
            buffer: Some(buffer),
            shm_pool: Some(shm_pool),
            mem_file: Some(mem_file),
            width,
            height,
            stride,
            frame_format,
            output,
            option,
        })
    }

    /// Capture a single frame using an existing StreamingCaptureContext
    ///
    /// This method reuses the resources from the provided context to efficiently
    /// capture a new frame without recreating protocol objects each time.
    ///
    /// # Parameters
    /// - `context`: The StreamingCaptureContext created with `create_streaming_context`
    ///
    /// # Returns
    /// The captured frame as an ImageViewInfo
    pub fn capture_frame_with_context(
        &mut self,
        context: &mut crate::ext_image_protocols::StreamingCaptureContext,
    ) -> Result<ImageViewInfo, WayshotError> {
        // Take ownership of components rather than borrowing self in multiple ways
        let mut event_queue = self
            .ext_image
            .as_mut()
            .expect("ext_image should be initialized")
            .event_queue
            .take()
            .expect("Control your self");
        let qh = {
            let ext_image = self
                .ext_image
                .as_ref()
                .expect("ext_image should be initialized");
            ext_image.qh.as_ref().expect("Should init").clone()
        };

        // Use existing resources from the context
        let session = context
            .session
            .as_ref()
            .expect("Session should be initialized in context");
        let buffer = context
            .buffer
            .as_ref()
            .expect("Buffer should be initialized in context");

        // Create a capture info for this frame
        let capture_info = CaptureInfo::new();

        // Create a new frame for this capture
        let frame = session.create_frame(&qh, capture_info.clone());

        // Attach buffer and capture
        frame.attach_buffer(buffer);
        frame.capture();

        // Wait for completion using a raw pointer to avoid borrow conflicts
        loop {
            {
                let mut_ref = &mut *self as *mut WayshotConnection;
                let result = event_queue.blocking_dispatch(unsafe { &mut *mut_ref });
                if let Err(e) = result {
                    self.reset_event_queue(event_queue);
                    // DispatchError is now properly converted to WayshotError
                    return Err(e.into());
                }
            }

            let info = capture_info.read().unwrap();
            match info.state() {
                FrameState::Succeeded => {
                    break;
                }
                FrameState::Failed(info) => {
                    self.reset_event_queue(event_queue);
                    match info {
                        Some(WEnum::Value(reason)) => match reason {
                            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason::Stopped => {
                                return Err(WayshotError::CaptureFailed("Stopped".to_owned()));
                            }
                            wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason::BufferConstraints => {
                                return Err(WayshotError::CaptureFailed("BufferConstraints".to_owned()));
                            }
                            _ => {
                                return Err(WayshotError::CaptureFailed("Unknown".to_owned()));
                            }
                        },
                        Some(WEnum::Unknown(code)) => {
                            return Err(WayshotError::CaptureFailed(format!(
                                "Unknown reason, code : {code}"
                            )));
                        }
                        None => {
                            return Err(WayshotError::CaptureFailed("No failure reason provided".to_owned()));
                        }
                    }
                }
                FrameState::Pending => {}
            }
        }

        self.reset_event_queue(event_queue);

        // Get image data from memory file
        let mem_file = context
            .mem_file
            .as_ref()
            .expect("Memory file should be initialized in context");
        let mut frame_mmap = unsafe { memmap2::MmapMut::map_mut(mem_file).unwrap() };

        // Process the image data
        let converter = crate::convert::create_converter(context.frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        // Create the full screen region representing the output
        let region = context.output.logical_region.inner.clone();

        // Store the frame in the context for proper cleanup later
        context.frame = Some(frame);

        Ok(ImageViewInfo {
            data: frame_mmap.deref().into(),
            width: context.width,
            height: context.height,
            color_type,
            region,
        })
    }

    /// Release resources associated with a StreamingCaptureContext
    ///
    /// This method explicitly releases Wayland protocol resources held by the context.
    /// While Rust's Drop implementations would eventually handle this, explicitly
    /// releasing resources can be helpful in some cases.
    ///
    /// # Parameters
    /// - `context`: The StreamingCaptureContext to release
    pub fn release_streaming_context(
        &mut self,
        context: &mut crate::ext_image_protocols::StreamingCaptureContext,
    ) {
        // Release frame if it exists
        if let Some(frame) = context.frame.take() {
            frame.destroy();
        }

        // Release session if it exists
        if let Some(session) = context.session.take() {
            session.destroy();
        }

        // Release source if it exists
        if let Some(source) = context.source.take() {
            source.destroy();
        }

        // Buffer and pool will be dropped automatically
        context.buffer = None;
        context.shm_pool = None;
        context.mem_file = None;
    }
}
