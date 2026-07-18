use std::{os::fd::AsFd, sync::atomic::Ordering};

use gbm::{BufferObject, BufferObjectFlags};
use wayland_client::{
    EventQueue, Proxy,
    globals::registry_queue_init,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
    },
};
use wayland_protocols::{
    ext::{
        image_capture_source::v1::client::{
            ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
            ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        },
        image_copy_capture::v1::client::{
            ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
            ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
        },
    },
    wp::linux_dmabuf::zv1::client::{
        zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
    },
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::{
    Error, Result, Size, WayshotConnection, WayshotTarget,
    dispatch::{CaptureFrameState, DMABUFState, FrameState, WayshotState},
};

/// It is a unit to do screencast. It storages used information for screencast
/// You should use it and related api to do screencast
#[derive(Debug)]
pub struct WayshotScreenCast {
    buffer: wl_buffer::WlBuffer,
    origin_size: Size<i32>,
    current_size: Size<i32>,
    cursor_overlay: bool,
    target: WayshotTarget,
    shm_pool: Option<WlShmPool>,
    shm_format: Option<wl_shm::Format>,
    bo: Option<BufferObject<()>>,
    #[cfg(feature = "egl")]
    egl_display: Option<crate::egl::EglDisplay>,
    image_copy_manager: Option<ExtImageCopyCaptureManagerV1>,
    foreign_manager: Option<ExtForeignToplevelImageCaptureSourceManagerV1>,
    output_manager: Option<ExtOutputImageCaptureSourceManagerV1>,
    wlr_screencopy: Option<ZwlrScreencopyManagerV1>,
    event_queue: EventQueue<CaptureFrameState>,
}

impl Drop for WayshotScreenCast {
    fn drop(&mut self) {
        if let Some(pool) = self.shm_pool.take() {
            pool.destroy()
        }
        self.buffer.destroy();
    }
}

impl WayshotScreenCast {
    /// Get the current_size of the screen or toplevel
    pub fn current_size(&self) -> Size<i32> {
        self.current_size
    }

    /// Get the buffer object
    pub fn dmabuf_bo(&self) -> Option<&BufferObject<()>> {
        self.bo.as_ref()
    }

    /// Get the buffer of the unit
    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }

    fn screencast_wlr(&mut self) -> Result<()> {
        let mut state = CaptureFrameState::new(false);
        let qh = self.event_queue.handle();
        let screencopy_manager = self.wlr_screencopy.as_ref().ok_or(Error::Unsupported(
            "wlr_screencopy is not support".to_owned(),
        ))?;

        tracing::debug!("Capturing output(shm buffer)...");
        let WayshotTarget::Screen(output) = &self.target else {
            unreachable!()
        };

        let frame = screencopy_manager.capture_output(self.cursor_overlay as i32, output, &qh, ());
        // Empty internal event buffer until buffer_done is set to true which is when the Buffer done
        // event is fired, aka the capture from the compositor is successful.
        while !state.buffer_done.load(Ordering::SeqCst) {
            self.event_queue.blocking_dispatch(&mut state)?;
        }
        if let Some(shm_format) = &self.shm_format {
            let Some(frame_format) = state
                .formats
                .iter()
                .find(|f| f.format == *shm_format)
                .copied()
            else {
                return Err(Error::NoSupportedBufferFormat);
            };

            self.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        } else {
            let Some(frame_format) = state.formats.first() else {
                return Err(Error::NoSupportedBufferFormat);
            };
            self.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        }
        frame.copy(&self.buffer);
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
                        break;
                    }
                }
            }

            self.event_queue.blocking_dispatch(&mut state)?;
        }

        #[cfg(feature = "egl")]
        if let (Some(egl_display), Some(bo)) = (self.egl_display.as_ref(), self.dmabuf_bo()) {
            if state.dmabuf_formats.is_empty() {
                return Err(Error::NoDMAStateError);
            }
            let frame_format = state.dmabuf_formats[0];
            crate::egl::create_egl_image_and_bind_to_gl_texture(*egl_display, bo, &frame_format)?;
        }
        Ok(())
    }
    /// Do screencast once
    /// Please check the result to see you should update the status
    ///
    /// if with [Error::FramecopyFailedWithReason], you need to update the status, for example,
    /// send the param_changes to pipewire
    pub fn screencast(&mut self) -> Result<()> {
        if self.target.is_screen() && self.image_copy_manager.is_none() {
            return self.screencast_wlr();
        }
        let state = CaptureFrameState::new(false);
        let qh = self.event_queue.handle();
        let options = if self.cursor_overlay {
            Options::PaintCursors
        } else {
            Options::empty()
        };
        let image_copy_manager = self.image_copy_manager.as_ref().ok_or(Error::Unsupported(
            "image_copy_manager is not supported".to_owned(),
        ))?;
        let (session, source) = match &self.target {
            WayshotTarget::Screen(output) => {
                let source = self
                    .output_manager
                    .as_ref()
                    .ok_or(Error::Unsupported(
                        "output_manager is not supported".to_owned(),
                    ))?
                    .create_source(output, &qh, ());

                (
                    image_copy_manager.create_session(&source, options, &qh, ()),
                    source,
                )
            }
            WayshotTarget::Toplevel(toplevel) => {
                let source = self
                    .foreign_manager
                    .as_ref()
                    .ok_or(Error::Unsupported(
                        "foreign_manager is not supported".to_owned(),
                    ))?
                    .create_source(toplevel, &qh, ());

                (
                    image_copy_manager.create_session(&source, options, &qh, ()),
                    source,
                )
            }
        };
        let frame = session.create_frame(&qh, ());
        let result = self.screencast_ext_frame(frame, state);
        session.destroy();
        source.destroy();
        result
    }

    fn screencast_ext_frame(
        &mut self,
        frame: ExtImageCopyCaptureFrameV1,
        mut state: CaptureFrameState,
    ) -> Result<()> {
        while !state.session_done {
            self.event_queue.blocking_dispatch(&mut state)?;
        }
        if let Some(shm_format) = &self.shm_format {
            let Some(frame_format) = state
                .formats
                .iter()
                .find(|f| f.format == *shm_format)
                .copied()
            else {
                return Err(Error::NoSupportedBufferFormat);
            };

            self.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        } else {
            let Some(frame_format) = state.formats.first() else {
                return Err(Error::NoSupportedBufferFormat);
            };
            self.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        }
        frame.attach_buffer(&self.buffer);
        frame.damage_buffer(0, 0, self.origin_size.width, self.origin_size.height);
        frame.capture();
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
                        break;
                    }
                }
            }

            self.event_queue.blocking_dispatch(&mut state)?;
        }

        #[cfg(feature = "egl")]
        if let (Some(egl_display), Some(bo)) = (self.egl_display.as_ref(), self.dmabuf_bo()) {
            if state.dmabuf_formats.is_empty() {
                return Err(Error::NoDMAStateError);
            }
            let frame_format = state.dmabuf_formats[0];
            crate::egl::create_egl_image_and_bind_to_gl_texture(*egl_display, bo, &frame_format)?;
        }

        Ok(())
    }
}

impl WayshotConnection {
    #[allow(clippy::type_complexity)]
    fn screencast_init(
        &self,
    ) -> Result<(
        EventQueue<CaptureFrameState>,
        Option<ExtImageCopyCaptureManagerV1>,
        Option<ExtForeignToplevelImageCaptureSourceManagerV1>,
        Option<ExtOutputImageCaptureSourceManagerV1>,
        Option<ZwlrScreencopyManagerV1>,
    )> {
        let event_queue = self.conn.new_event_queue::<CaptureFrameState>();
        let screencopy_manager = self.registers.screencopy_manager.clone();

        // Bind managers
        let manager = self.registers.image_copy_capture_manager.clone();
        let toplevel_source_manager = self.registers.toplevel_source_manager.clone();
        let output_source_manager = self.registers.output_image_management.clone();
        Ok((
            event_queue,
            manager,
            toplevel_source_manager,
            output_source_manager,
            screencopy_manager,
        ))
    }
    /// This will run once to get the device provided by ext-image-copy. If you did not init the
    /// dmabuf at the first, you can try to use this way to init one
    pub fn try_init_dmabuf(&mut self, target: WayshotTarget) -> Result<()> {
        self.find_dmabuf = true;
        if self.dmabuf_state.is_some() {
            return Ok(());
        }
        let (mut state, _) = self.capture_target_frame_get_state(&target, false, None)?;
        let (globals, evq) = registry_queue_init::<WayshotState>(&self.conn)?;
        let Some(gbm) = state.gbm.take() else {
            return Err(Error::NoDMAStateError);
        };
        let linux_dmabuf =
            globals.bind(&evq.handle(), 4..=ZwpLinuxDmabufV1::interface().version, ())?;
        self.dmabuf_state = Some(DMABUFState {
            linux_dmabuf,
            gbmdev: gbm,
        });
        Ok(())
    }

    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    /// Same with create_screencast_with_shm, but now it is with dmabuf
    /// And bind the a `EglDisplay`, to support the egl
    #[cfg(feature = "egl")]
    pub fn create_screencast_with_egl(
        &self,
        target: WayshotTarget,
        cursor_overlay: bool,
        egl_display: crate::egl::EglDisplay,
    ) -> Result<WayshotScreenCast> {
        let mut cast = self.create_screencast_with_dmabuf(target, cursor_overlay)?;
        cast.egl_display = Some(egl_display);
        Ok(cast)
    }
    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    /// Same with create_screencast_with_shm, but now it is with dmabuf
    pub fn create_screencast_with_dmabuf(
        &self,
        target: WayshotTarget,
        cursor_overlay: bool,
    ) -> Result<WayshotScreenCast> {
        let Some(dmabuf_state) = &self.dmabuf_state else {
            return Err(Error::NoDMAStateError);
        };
        let (state, _) = self.capture_target_frame_get_state(&target, cursor_overlay, None)?;
        if state.dmabuf_formats.is_empty() {
            return Err(Error::NoSupportedBufferFormat);
        }
        let (event_queue, image_copy_manager, foreign_manager, output_manager, wlr_screencopy) =
            self.screencast_init()?;

        let frame_format = state.dmabuf_formats[0];
        tracing::trace!("Selected frame buffer format: {:#?}", frame_format);
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

        let fd = bo.fd_for_plane(0)?;
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
        let buffer = dma_params.create_immed(
            dma_width as i32,
            dma_height as i32,
            frame_format.format,
            zwp_linux_buffer_params_v1::Flags::empty(),
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
            shm_pool: None,
            shm_format: None,
            bo: Some(bo),
            #[cfg(feature = "egl")]
            egl_display: None,
            image_copy_manager,
            foreign_manager,
            output_manager,
            event_queue,
            wlr_screencopy,
        })
    }
    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    pub fn create_screencast_with_shm<T: AsFd>(
        &self,
        target: WayshotTarget,
        cursor_overlay: bool,
        shm_format: wl_shm::Format,
        fd: T,
    ) -> Result<WayshotScreenCast> {
        let (event_queue, image_copy_manager, foreign_manager, output_manager, wlr_screencopy) =
            self.screencast_init()?;
        let (state, _) = self.capture_target_frame_get_state(&target, cursor_overlay, None)?;
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
            shm_pool: Some(shm_pool),
            shm_format: Some(shm_format),
            bo: None,
            #[cfg(feature = "egl")]
            egl_display: None,
            image_copy_manager,
            foreign_manager,
            output_manager,
            event_queue,
            wlr_screencopy,
        })
    }
}
