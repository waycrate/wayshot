use std::{
    process::exit,
    sync::atomic::{AtomicBool, Ordering},
};
use wayland_client::{
    delegate_noop,
    globals::GlobalListContents,
    protocol::{
        wl_buffer::WlBuffer, wl_output, wl_output::WlOutput, wl_registry, wl_registry::WlRegistry,
        wl_shm::WlShm, wl_shm_pool::WlShmPool,
    },
    Connection, Dispatch, QueueHandle, WEnum,
    WEnum::Value,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1, zxdg_output_v1::ZxdgOutputV1,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1, zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use crate::{
    output::{OutputInfo, OutputPositioning, WlOutputMode},
    screencopy::FrameFormat,
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
        {
            if interface == "wl_output" {
                if version >= 4 {
                    let output = wl_registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
                    state.outputs.push(OutputInfo {
                        wl_output: output,
                        name: "".to_string(),
                        description: String::new(),
                        transform: wl_output::Transform::Normal,
                        dimensions: OutputPositioning {
                            x: 0,
                            y: 0,
                            width: 0,
                            height: 0,
                        },
                        mode: WlOutputMode {
                            width: 0,
                            height: 0,
                        },
                    });
                } else {
                    tracing::error!("Ignoring a wl_output with version < 4.");
                }
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
        let output: &mut OutputInfo = state
            .outputs
            .iter_mut()
            .find(|x| x.wl_output == *wl_output)
            .unwrap();

        match event {
            wl_output::Event::Name { name } => {
                output.name = name;
            }
            wl_output::Event::Description { description } => {
                output.description = description;
            }
            wl_output::Event::Mode { width, height, .. } => {
                output.mode = WlOutputMode { width, height };
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                output.transform = transform;
            }
            _ => (),
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
        let output_info = state.outputs.get_mut(*index).unwrap();

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                output_info.dimensions.x = x;
                output_info.dimensions.y = y;
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                output_info.dimensions.width = width;
                output_info.dimensions.height = height;
            }
            zxdg_output_v1::Event::Done => {}
            zxdg_output_v1::Event::Name { .. } => {}
            zxdg_output_v1::Event::Description { .. } => {}
            _ => {}
        };
    }
}

/// State of the frame after attempting to copy it's data to a wl_buffer.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FrameState {
    /// Compositor returned a failed event on calling `frame.copy`.
    Failed,
    /// Compositor sent a Ready event on calling `frame.copy`.
    Finished,
}

pub struct CaptureFrameState {
    pub formats: Vec<FrameFormat>,
    pub state: Option<FrameState>,
    pub buffer_done: AtomicBool,
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
                    frame.formats.push(FrameFormat {
                        format: f,
                        width,
                        height,
                        stride,
                    })
                } else {
                    tracing::debug!("Received Buffer event with unidentified format");
                    exit(1);
                }
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                // If the frame is successfully copied, a “flags” and a “ready” events are sent. Otherwise, a “failed” event is sent.
                // This is useful when we call .copy on the frame object.
                frame.state.replace(FrameState::Finished);
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                frame.state.replace(FrameState::Failed);
            }
            zwlr_screencopy_frame_v1::Event::Damage { .. } => {}
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf { .. } => {}
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

// TODO: Create a xdg-shell surface, check for the enter event, grab the output from it.

pub struct WayshotState {}

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
