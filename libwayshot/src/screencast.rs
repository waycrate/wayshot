use std::os::fd::AsFd;

use wayland_client::{
    EventQueue,
    protocol::{
        wl_buffer,
        wl_shm::{self, WlShm},
    },
};

use crate::{
    EmbeddedRegion, Error, Result, Size, WayshotConnection, WayshotFrame, WayshotTarget,
    dispatch::{CaptureFrameState, FrameState},
    screencopy::FrameFormat,
};

pub struct WayshotScreenCast {
    buffer: wl_buffer::WlBuffer,
    frame: WayshotFrame,
    event_queue: EventQueue<CaptureFrameState>,
    state: CaptureFrameState,
    frame_format: FrameFormat,
}

impl WayshotScreenCast {
    fn reset(&mut self) {
        self.state.state = None;
        self.state.session_done = false;
    }
    pub fn capture(&mut self) -> Result<()> {
        self.reset();
        let Size { width, height } = self.frame_format.size;
        match &self.frame {
            WayshotFrame::ExtImageCopy(frame) => {
                frame.attach_buffer(&self.buffer);
                frame.damage_buffer(0, 0, width as i32, height as i32);
                frame.capture();
            }
            WayshotFrame::WlrScreenshot(frame) => {
                frame.copy(&self.buffer);
            }
        }
        loop {
            // Basically reads, if frame state is not None then...
            if let Some(state) = self.state.state {
                match state {
                    FrameState::Failed | FrameState::FailedWithReason(_) => {
                        tracing::error!("Frame copy failed");
                        return Err(Error::FramecopyFailed);
                    }
                    FrameState::Finished => {
                        tracing::trace!("Frame copy finished");
                        return Ok(());
                    }
                }
            }

            self.event_queue.blocking_dispatch(&mut self.state)?;
        }
    }
}

impl WayshotConnection {
    pub fn create_screencast_with_format<T: AsFd>(
        &self,
        frame_format: wl_shm::Format,
        capture_region: Option<EmbeddedRegion>,
        target: WayshotTarget,
        cursor_overlay: bool,
        fd: T,
    ) -> Result<WayshotScreenCast> {
        let (state, event_queue, frame) =
            self.capture_target_frame_get_state(cursor_overlay, &target, capture_region)?;
        let Some(frame_format) = state
            .formats
            .iter()
            .find(|f| f.format == frame_format)
            .copied()
        else {
            return Err(Error::NoSupportedBufferFormat);
        };
        let qh = event_queue.handle();

        // Instantiate shm global.
        let shm = self.globals.bind::<WlShm, _, _>(&qh, 1..=1, ())?;
        let shm_pool = shm.create_pool(
            fd.as_fd(),
            frame_format
                .byte_size()
                .try_into()
                .map_err(|_| Error::BufferTooSmall)?,
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
        shm_pool.destroy();

        Ok(WayshotScreenCast {
            buffer,
            frame,
            event_queue,
            state,
            frame_format,
        })
    }
}
