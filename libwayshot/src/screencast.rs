use std::os::fd::AsFd;

use wayland_client::protocol::{
    wl_buffer,
    wl_shm::{self, WlShm},
    wl_shm_pool::WlShmPool,
};

use crate::{
    EmbeddedRegion, Error, Result, Size, WayshotConnection, WayshotFrame, WayshotTarget,
    dispatch::FrameState,
};

#[derive(Debug, Clone)]
pub struct WayshotScreenCast {
    buffer: wl_buffer::WlBuffer,
    origin_size: Size<i32>,
    current_size: Size<i32>,
    cursor_overlay: bool,
    target: WayshotTarget,
    capture_region: Option<EmbeddedRegion>,
    shm_pool: WlShmPool,
    shm_format: wl_shm::Format,
}

impl Drop for WayshotScreenCast {
    fn drop(&mut self) {
        self.shm_pool.destroy();
        self.buffer.destroy();
    }
}

impl WayshotScreenCast {
    /// Get the current_size of the screen or toplevel
    pub fn current_size(&self) -> Size<i32> {
        self.current_size
    }
}

impl WayshotConnection {
    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    pub fn create_screencast_with_format<T: AsFd>(
        &self,
        shm_format: wl_shm::Format,
        capture_region: Option<EmbeddedRegion>,
        target: WayshotTarget,
        cursor_overlay: bool,
        fd: T,
    ) -> Result<WayshotScreenCast> {
        let (state, event_queue, _) =
            self.capture_target_frame_get_state(cursor_overlay, &target, capture_region.clone())?;
        let Some(frame_format) = state
            .formats
            .iter()
            .find(|f| f.format == shm_format)
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

        let origin_size = Size {
            width: frame_format.size.width as i32,
            height: frame_format.size.height as i32,
        };
        Ok(WayshotScreenCast {
            buffer,
            origin_size,
            current_size: origin_size,
            cursor_overlay,
            target,
            capture_region,
            shm_pool,
            shm_format,
        })
    }

    /// do screencapture once
    pub fn capture_screen(&self, cast: &mut WayshotScreenCast) -> Result<()> {
        let (mut state, mut event_queue, frame) = self.capture_target_frame_get_state(
            cast.cursor_overlay,
            &cast.target,
            cast.capture_region,
        )?;

        let Some(frame_format) = state
            .formats
            .iter()
            .find(|f| f.format == cast.shm_format)
            .copied()
        else {
            return Err(Error::NoSupportedBufferFormat);
        };

        cast.current_size = Size {
            width: frame_format.size.width as i32,
            height: frame_format.size.height as i32,
        };

        match &frame {
            WayshotFrame::ExtImageCopy(frame) => {
                frame.attach_buffer(&cast.buffer);
                frame.damage_buffer(0, 0, cast.origin_size.width, cast.origin_size.height);
                frame.capture();
            }
            WayshotFrame::WlrScreenshot(frame) => {
                frame.copy(&cast.buffer);
            }
        }
        loop {
            // Basically reads, if frame state is not None then...
            if let Some(state) = state.state {
                match state {
                    FrameState::Failed => {
                        tracing::error!("Frame copy failed");
                        return Err(Error::FramecopyFailed);
                    }
                    FrameState::FailedWithReason(reason) => {
                        tracing::error!("Frame copy failed");
                        return Err(Error::FramecopyFailedWithReason(reason));
                    }
                    FrameState::Finished => {
                        tracing::trace!("Frame copy finished");
                        return Ok(());
                    }
                }
            }

            event_queue.blocking_dispatch(&mut state)?;
        }
    }
}
