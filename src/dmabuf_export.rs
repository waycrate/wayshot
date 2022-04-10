use smithay_client_toolkit::reexports::{
    client::{protocol::wl_output::WlOutput, Display, GlobalManager, Main},
    protocols::wlr::unstable::export_dmabuf::v1::client::{
        zwlr_export_dmabuf_frame_v1, zwlr_export_dmabuf_frame_v1::ZwlrExportDmabufFrameV1,
        zwlr_export_dmabuf_manager_v1::ZwlrExportDmabufManagerV1,
    },
};

use std::{
    cell::RefCell,
    os::unix::io::RawFd,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DmaBufFrameFormat {
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub buffer_flags: u32,
    pub flags: zwlr_export_dmabuf_frame_v1::Flags,
    pub format: u32,
    pub mod_high: u32,
    pub mod_low: u32,
    pub num_objects: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DmaBufObject {
    index: u32,
    fd: RawFd,
    size: u32,
    offset: u32,
    stride: u32,
    plane_index: u32,
}

pub fn capture_output_frame(
    display: Display,
    cursor_overlay: i32,
    output: WlOutput,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connecting to wayland environment.
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    // Instantiating the global manager.
    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| unreachable!())?;

    let frame_canel: Rc<AtomicBool> = Rc::new(AtomicBool::new(false));
    let frame_format: Rc<RefCell<Option<DmaBufFrameFormat>>> = Rc::new(RefCell::new(None));
    let frame_object: Rc<RefCell<Option<DmaBufObject>>> = Rc::new(RefCell::new(None));
    let frame_ready: Rc<AtomicBool> = Rc::new(AtomicBool::new(false));

    // Instantiating the dmabuf manager.
    let dmabuf_manager =
        if let Ok(manager) = globals.instantiate_exact::<ZwlrExportDmabufManagerV1>(1) {
            manager
        } else {
            panic!("Global manager failed to instantiate dmabuf_manager");
        };

    // Capture output.
    let dmabuf_frame: Main<ZwlrExportDmabufFrameV1> =
        dmabuf_manager.capture_output(cursor_overlay, &output);

    // Assigning callbacks to the frame.
    dmabuf_frame.quick_assign({
        let frame_ready = frame_ready.clone();
        let frame_cancel = frame_canel.clone();
        let frame_format = frame_format.clone();
        let frame_object = frame_object.clone();

        move |_, event, _| match event {
            zwlr_export_dmabuf_frame_v1::Event::Frame {
                width,
                height,
                offset_x,
                offset_y,
                buffer_flags,
                flags,
                format,
                mod_high,
                mod_low,
                num_objects,
            } => {
                frame_format.borrow_mut().replace(DmaBufFrameFormat {
                    width,
                    height,
                    offset_x,
                    offset_y,
                    buffer_flags,
                    flags,
                    format,
                    mod_high,
                    mod_low,
                    num_objects,
                });
            }

            zwlr_export_dmabuf_frame_v1::Event::Object {
                index,
                fd,
                size,
                offset,
                stride,
                plane_index,
            } => {
                frame_object.borrow_mut().replace(DmaBufObject {
                    index,
                    fd,
                    size,
                    offset,
                    stride,
                    plane_index,
                });
            }
            zwlr_export_dmabuf_frame_v1::Event::Ready { .. } => {
                frame_ready.store(true, Ordering::SeqCst);
            }
            zwlr_export_dmabuf_frame_v1::Event::Cancel { reason } => match reason {
                zwlr_export_dmabuf_frame_v1::CancelReason::Permanent => {
                    frame_cancel.store(true, Ordering::SeqCst);
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    });

    while !frame_ready.load(Ordering::SeqCst) {
        event_queue.dispatch(&mut (), |_, _, _| unreachable!())?;
    }
    println!("Finished running dmabuf_capture_output");

    Ok(())
}
