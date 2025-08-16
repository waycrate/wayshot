use std::{
    collections::HashSet,
    os::fd::{AsFd, BorrowedFd},
    sync::atomic::{AtomicBool, Ordering},
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    WEnum::{self, Value},
    delegate_noop,
    globals::GlobalListContents,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::WlShm,
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;

use wayland_protocols::{
    ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::FailureReason,
    wp::{
        linux_dmabuf::zv1::client::{
            zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1},
            zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
        },
        viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    },
    xdg::xdg_output::zv1::client::{
        zxdg_output_manager_v1::ZxdgOutputManagerV1,
        zxdg_output_v1::{self, ZxdgOutputV1},
    },
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::{
    output::OutputInfo,
    region::{LogicalRegion, Position, Size},
    screencopy::{DMAFrameFormat, FrameFormat},
};

#[derive(Debug)]
pub struct OutputCaptureState {
    pub outputs: Vec<OutputInfo>,
}

impl Dispatch<WlRegistry, ()> for OutputCaptureState {
    #[tracing::instrument(skip(wl_registry, qh), ret, level = "trace")]
    fn event(
        state: &mut Self,
        wl_registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        /* > The name event is sent after binding the output object. This event
         * is only sent once per output object, and the name does not change
         * over the lifetime of the wl_output global. */

        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
            && interface == "wl_output"
        {
            if version >= 4 {
                let output = wl_registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
                state.outputs.push(OutputInfo {
                    output,
                    name: "".to_string(),
                    description: String::new(),
                    transform: wl_output::Transform::Normal,
                    physical_size: Size::default(),
                    logical_region: LogicalRegion::default(),
                    scale: 1,
                });
            } else {
                tracing::error!("Ignoring a wl_output with version < 4.");
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for OutputCaptureState {
    #[tracing::instrument(skip(wl_output), ret, level = "trace")]
    fn event(
        state: &mut Self,
        wl_output: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let output: &mut OutputInfo =
            match state.outputs.iter_mut().find(|x| x.output == *wl_output) {
                Some(output) => output,
                _ => {
                    tracing::error!(
                        "Received event for an output that is not registered: {event:#?}"
                    );
                    return;
                }
            };

        match event {
            wl_output::Event::Name { name } => {
                output.name = name;
            }
            wl_output::Event::Description { description } => {
                output.description = description;
            }
            wl_output::Event::Mode { width, height, .. } => {
                output.physical_size = Size {
                    width: width as u32,
                    height: height as u32,
                };
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                output.transform = transform;
            }
            wl_output::Event::Scale { factor } => {
                output.scale = factor;
            }
            wl_output::Event::Done => {}
            _ => {}
        }
    }
}

delegate_noop!(OutputCaptureState: ignore ZxdgOutputManagerV1);

impl Dispatch<ZxdgOutputV1, usize> for OutputCaptureState {
    #[tracing::instrument(ret, level = "trace")]
    fn event(
        state: &mut Self,
        _: &ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let output_info = match state.outputs.get_mut(*index) {
            Some(output_info) => output_info,
            _ => {
                tracing::error!(
                    "Received event for output index {index} that is not registered: {event:#?}"
                );
                return;
            }
        };

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                output_info.logical_region.inner.position = Position { x, y };
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                output_info.logical_region.inner.size = Size {
                    width: width as u32,
                    height: height as u32,
                };
            }
            zxdg_output_v1::Event::Done => {}
            zxdg_output_v1::Event::Name { .. } => {}
            zxdg_output_v1::Event::Description { .. } => {}
            _ => {}
        };
    }
}

/// State of the frame after attempting to copy its data to a buffer.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FrameState {
    /// Compositor returned a failed event on calling `frame.copy`.
    Failed(Option<WEnum<FailureReason>>),
    /// Compositor sent a Ready event on calling `frame.copy`.
    Succeeded,
    /// Capture is still pending (not yet failed or succeeded).
    Pending,
}

pub struct CaptureFrameState {
    pub formats: Vec<FrameFormat>,
    pub dmabuf_formats: Vec<DMAFrameFormat>,
    pub state: Option<FrameState>,
    pub buffer_done: AtomicBool,
}

impl Dispatch<ZwpLinuxDmabufV1, ()> for CaptureFrameState {
    fn event(
        _frame: &mut Self,
        _proxy: &ZwpLinuxDmabufV1,
        _event: zwp_linux_dmabuf_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpLinuxBufferParamsV1, ()> for CaptureFrameState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpLinuxBufferParamsV1,
        _event: zwp_linux_buffer_params_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureFrameState {
    #[tracing::instrument(skip(frame), ret, level = "trace")]
    fn event(
        frame: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                if let Value(f) = format {
                    tracing::debug!("Received Buffer event with format: {f:?}");
                    frame.formats.push(FrameFormat {
                        format: f,
                        size: Size { width, height },
                        stride,
                    })
                } else {
                    tracing::debug!("Received Buffer event with unidentified format");
                }
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                // If the frame is successfully copied, a “flags” and a “ready” events are sent. Otherwise, a “failed” event is sent.
                // This is useful when we call .copy on the frame object.
                frame.state.replace(FrameState::Succeeded);
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                frame.state.replace(FrameState::Failed(None));
            }
            zwlr_screencopy_frame_v1::Event::Damage { .. } => {}
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf {
                format,
                width,
                height,
            } => {
                tracing::debug!(
                    "Received wlr-screencopy linux_dmabuf event with format: {format} and size {width}x{height}"
                );
                frame.dmabuf_formats.push(DMAFrameFormat {
                    format,
                    size: Size { width, height },
                });
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                frame.buffer_done.store(true, Ordering::SeqCst);
            }
            _ => {}
        };
    }
}

delegate_noop!(CaptureFrameState: ignore WlShm);
delegate_noop!(CaptureFrameState: ignore WlShmPool);
delegate_noop!(CaptureFrameState: ignore WlBuffer);
delegate_noop!(CaptureFrameState: ignore ZwlrScreencopyManagerV1);

pub struct WayshotState {}
delegate_noop!(WayshotState: ignore ZwpLinuxDmabufV1);
impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for WayshotState {
    fn event(
        _: &mut WayshotState,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<WayshotState>,
    ) {
    }
}

pub(crate) struct Card(std::fs::File);

/// Implementing [`AsFd`] is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling [`File::as_fd()`] on the inner
/// [`File`].
impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl drm::Device for Card {}
/// Simple helper methods for opening a `Card`.
impl Card {
    pub fn open(path: &str) -> Self {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);
        Card(options.open(path).unwrap())
    }
}
#[derive(Debug)]
pub(crate) struct DMABUFState {
    pub linux_dmabuf: ZwpLinuxDmabufV1,
    pub gbmdev: gbm::Device<Card>,
}

pub struct LayerShellState {
    pub configured_outputs: HashSet<WlOutput>,
}

impl LayerShellState {
    pub(crate) fn new() -> Self {
        Self {
            configured_outputs: HashSet::new(),
        }
    }
}

// Here there is another problem for Cosmic Based Compositor
// Currently LayerShell works flawlessly for wlroots based compositors such as sway or Treeland,
// However when it comes to compositors like Cosmic, this doesn't seem to work properly..
// The Layer won't appear on Cosmic similar to other compositors even while Cosmic does indeed support wlr_shell && wlr_layer_surface.

// To Fix this, I have previosly implemented XDG based layer shell and Surface which seemingly works fine for Cosmic.
// However another dillema here is while the previous scenario,
// the wlr protocols didn't work, but now this works even while having waysip still uses the very same wlr protocols.

// Aakash and I have deemed this again another one of the quirks of Cosmic Alpha build
// So I have reverted it back to wlr-based protocols
// but If anyone believes on converting waysip to xdg based protocols, do let me know I can gladly add Back the xdg protocols code for wayshot again.

delegate_noop!(LayerShellState: ignore WlCompositor);
delegate_noop!(LayerShellState: ignore WlShm);
delegate_noop!(LayerShellState: ignore WlShmPool);
delegate_noop!(LayerShellState: ignore WlBuffer);
delegate_noop!(LayerShellState: ignore ZwlrLayerShellV1);
delegate_noop!(LayerShellState: ignore WlSurface);
delegate_noop!(LayerShellState: ignore WpViewport);
delegate_noop!(LayerShellState: ignore WpViewporter);

impl wayland_client::Dispatch<ZwlrLayerSurfaceV1, WlOutput> for LayerShellState {
    // No need to instrument here, span from lib.rs is automatically used.
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        data: &WlOutput,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
			wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Configure {
				serial,
				width: _,
				height: _,
			} => {
				tracing::debug!("Acking configure");
				state.configured_outputs.insert(data.clone());
				proxy.ack_configure(serial);
				tracing::trace!("Acked configure");
			}
			wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::Event::Closed => {
				tracing::debug!("Closed")
			}
			_ => {}
		}
    }
}

use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

use wayland_client::event_created_child;

use crate::WayshotConnection;
use crate::ext_image_protocols::{CaptureInfo, TopLevel};

delegate_noop!(WayshotConnection: ignore ExtImageCaptureSourceV1);
delegate_noop!(WayshotConnection: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(WayshotConnection: ignore ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(WayshotConnection: ignore WlShm);
delegate_noop!(WayshotConnection: ignore ZxdgOutputManagerV1);
delegate_noop!(WayshotConnection: ignore ExtImageCopyCaptureManagerV1);
delegate_noop!(WayshotConnection: ignore WlBuffer);
delegate_noop!(WayshotConnection: ignore WlShmPool);

impl Dispatch<WlRegistry, GlobalListContents> for WayshotConnection {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for WayshotConnection {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            state
                .ext_image
                .as_mut()
                .expect("ext_image should be initialized")
                .toplevels
                .push(TopLevel::new(toplevel));
        }
    }
    event_created_child!(WayshotConnection, ExtForeignToplevelHandleV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for WayshotConnection {
    fn event(
        state: &mut Self,
        toplevel: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Use ext_image.toplevels for all fields
        let toplevels = match state.ext_image.as_mut() {
            Some(ext_image) => &mut ext_image.toplevels,
            None => return,
        };
        match event {
            ext_foreign_toplevel_handle_v1::Event::Title { title } => {
                if let Some(current_info) = toplevels
                    .iter_mut()
                    .find(|my_toplevel| my_toplevel.handle == *toplevel)
                {
                    current_info.title = title;
                }
            }
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(current_info) = toplevels
                    .iter_mut()
                    .find(|my_toplevel| my_toplevel.handle == *toplevel)
                {
                    current_info.app_id = app_id;
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                if let Some(current_info) = toplevels
                    .iter_mut()
                    .find(|my_toplevel| my_toplevel.handle == *toplevel)
                {
                    current_info.identifier = identifier;
                }
            }
            ext_foreign_toplevel_handle_v1::Event::Closed => {
                if let Some(current_info) = toplevels
                    .iter_mut()
                    .find(|my_toplevel| my_toplevel.handle == *toplevel)
                {
                    current_info.active = false;
                }
            }
            _ => {}
        }
    }
}

use std::sync::{Arc, RwLock};

impl Dispatch<ExtImageCopyCaptureFrameV1, Arc<RwLock<CaptureInfo>>> for WayshotConnection {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as Proxy>::Event,
        data: &Arc<RwLock<CaptureInfo>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut data = data.write().unwrap();
        match event {
            ext_image_copy_capture_frame_v1::Event::Ready => {
                data.state = FrameState::Succeeded;
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                data.state = FrameState::Failed(Some(reason))
            }
            ext_image_copy_capture_frame_v1::Event::Transform {
                transform: WEnum::Value(transform),
            } => {
                data.transform = transform;
            }
            _ => {}
        }
    }
}

//Currently the below Dispatch function does work
//however for certain few compositors which doesn't display the format properly
//such as Cosmic, this protocol will fail,
//Currently its under assumption that Cosmic still being Alpha is the source of problem
//Giving benefit of the Doubt that the code works correctly as it works in sway.

// I will try to better explain this later in the blog I will write After my exams, (I will maybe link it here)

//To Try making this work on Cosmic simply Comment the few code statements below.

impl Dispatch<ExtImageCopyCaptureSessionV1, Arc<RwLock<FrameFormat>>> for WayshotConnection {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as Proxy>::Event,
        data: &Arc<RwLock<FrameFormat>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut frame_info = data.write().unwrap();
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                frame_info.size = Size { width, height };
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat {
                format: WEnum::Value(fmt),
            } => {
                tracing::trace!("Compositor supports shm format: {fmt:?}");
                if frame_info.format == wayland_client::protocol::wl_shm::Format::Xbgr8888 {
                    //<-- This
                    //frame_info.format = wayland_client::protocol::wl_shm::Format::Xbgr8888; // <-- Uncomment this
                    frame_info.format = fmt; // <-- This
                } // <-- This
            }
            ext_image_copy_capture_session_v1::Event::Done => {}
            _ => {}
        }
    }
}
