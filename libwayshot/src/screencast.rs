use std::os::fd::{AsFd, IntoRawFd};

use gbm::{BufferObject, BufferObjectFlags};
use wayland_client::{
    Proxy,
    globals::registry_queue_init,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
    },
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{
    zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
};

use crate::{
    EmbeddedRegion, Error, Result, Size, WayshotConnection, WayshotFrame, WayshotTarget,
    dispatch::{DMABUFState, FrameState, WayshotState},
};
use r_egl_wayland::{EGL_INSTALCE, WayEglTrait, r_egl as egl};

/// It is a unit to do screencast. It storages used information for screencast
/// You should use it and related api to do screencast
#[derive(Debug)]
pub struct WayshotScreenCast {
    buffer: wl_buffer::WlBuffer,
    origin_size: Size<i32>,
    current_size: Size<i32>,
    cursor_overlay: bool,
    target: WayshotTarget,
    capture_region: Option<EmbeddedRegion>,
    shm_pool: Option<WlShmPool>,
    shm_format: Option<wl_shm::Format>,
    bo: Option<BufferObject<()>>,
    egl_display: Option<egl::Display>,
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
}

impl WayshotConnection {
    /// This will run once to get the device provided by ext-image-copy. If you did not init the
    /// dmabuf at the first, you can try to use this way to init one
    pub fn try_init_dmabuf(&mut self, target: WayshotTarget) -> Result<()> {
        if self.dmabuf_state.is_some() {
            return Ok(());
        }
        let (mut state, _, _) = self.capture_target_frame_get_state(false, &target, None)?;
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

    pub fn create_screencast_with_egl(
        &self,
        capture_region: Option<EmbeddedRegion>,
        target: WayshotTarget,
        cursor_overlay: bool,
    ) -> Result<WayshotScreenCast> {
        let Some(dmabuf_state) = &self.dmabuf_state else {
            return Err(Error::NoDMAStateError);
        };

        let egl_display = EGL_INSTALCE.get_display_wl(&self.conn.display())?;

        tracing::trace!("eglDisplay obtained from Wayland connection's display");

        EGL_INSTALCE.initialize(egl_display)?;
        let (state, event_queue, _) =
            self.capture_target_frame_get_state(cursor_overlay, &target, capture_region)?;
        if state.dmabuf_formats.is_empty() {
            return Err(Error::NoSupportedBufferFormat);
        }
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
            capture_region,
            shm_pool: None,
            shm_format: None,
            bo: Some(bo),
            egl_display: Some(egl_display),
        })
    }
    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    /// Same with create_screencast_with_shm, but now it is with dmabuf
    pub fn create_screencast_with_dmabuf(
        &self,
        capture_region: Option<EmbeddedRegion>,
        target: WayshotTarget,
        cursor_overlay: bool,
    ) -> Result<WayshotScreenCast> {
        let Some(dmabuf_state) = &self.dmabuf_state else {
            return Err(Error::NoDMAStateError);
        };
        let (state, event_queue, _) =
            self.capture_target_frame_get_state(cursor_overlay, &target, capture_region)?;
        if state.dmabuf_formats.is_empty() {
            return Err(Error::NoSupportedBufferFormat);
        }
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
            capture_region,
            shm_pool: None,
            shm_format: None,
            bo: Some(bo),
            egl_display: None,
        })
    }
    /// This will save a screencast status for you
    /// We suggest you to use this api to do screencast
    pub fn create_screencast_with_shm<T: AsFd>(
        &self,
        shm_format: wl_shm::Format,
        capture_region: Option<EmbeddedRegion>,
        target: WayshotTarget,
        cursor_overlay: bool,
        fd: T,
    ) -> Result<WayshotScreenCast> {
        let (state, event_queue, _) =
            self.capture_target_frame_get_state(cursor_overlay, &target, capture_region)?;
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
            shm_pool: Some(shm_pool),
            shm_format: Some(shm_format),
            bo: None,
            egl_display: None,
        })
    }

    /// Do screencast once
    /// Please check the result to see you should update the status
    ///
    /// if with [Error::FramecopyFailedWithReason], you need to update the status, for example,
    /// send the param_changes to pipewire
    pub fn screencast(&self, cast: &mut WayshotScreenCast) -> Result<()> {
        let (mut state, mut event_queue, frame) = self.capture_target_frame_get_state(
            cast.cursor_overlay,
            &cast.target,
            cast.capture_region,
        )?;

        if let Some(shm_format) = &cast.shm_format {
            let Some(frame_format) = state
                .formats
                .iter()
                .find(|f| f.format == *shm_format)
                .copied()
            else {
                return Err(Error::NoSupportedBufferFormat);
            };

            cast.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        } else {
            let Some(frame_format) = state.formats.first() else {
                return Err(Error::NoSupportedBufferFormat);
            };
            cast.current_size = Size {
                width: frame_format.size.width as i32,
                height: frame_format.size.height as i32,
            };
        }
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
                        break;
                    }
                }
            }

            event_queue.blocking_dispatch(&mut state)?;
        }

        if let (Some(egl_display), Some(bo)) = (cast.egl_display, cast.dmabuf_bo()) {
            type Attrib = egl::Attrib;
            if state.dmabuf_formats.is_empty() {
                return Err(Error::NoDMAStateError);
            }
            let frame_format = state.dmabuf_formats[0];
            let modifier: u64 = bo.modifier().into();
            let image_attribs = [
                egl::WIDTH as Attrib,
                frame_format.size.width as Attrib,
                egl::HEIGHT as Attrib,
                frame_format.size.height as Attrib,
                egl::LINUX_DRM_FOURCC_EXT as Attrib,
                bo.format() as Attrib,
                egl::DMA_BUF_PLANE0_FD_EXT as Attrib,
                bo.fd_for_plane(0)?.into_raw_fd() as Attrib,
                egl::DMA_BUF_PLANE0_OFFSET_EXT as Attrib,
                bo.offset(0) as Attrib,
                egl::DMA_BUF_PLANE0_PITCH_EXT as Attrib,
                bo.stride_for_plane(0) as Attrib,
                egl::DMA_BUF_PLANE0_MODIFIER_LO_EXT as Attrib,
                (modifier as u32) as Attrib,
                egl::DMA_BUF_PLANE0_MODIFIER_HI_EXT as Attrib,
                (modifier >> 32) as Attrib,
                egl::ATTRIB_NONE as Attrib,
            ];
            unsafe {
                let image = EGL_INSTALCE.create_image(
                    egl_display,
                    egl::Context::from_ptr(egl::NO_CONTEXT),
                    egl::LINUX_DMA_BUF_EXT as u32,
                    egl::ClientBuffer::from_ptr(std::ptr::null_mut()), //NULL
                    &image_attribs,
                )?;
                let gl_egl_image_texture_target_2d_oes: unsafe extern "system" fn(
                    target: gl::types::GLenum,
                    image: gl::types::GLeglImageOES,
                )
                    -> () = std::mem::transmute(
                    match EGL_INSTALCE.get_proc_address("glEGLImageTargetTexture2DOES") {
                        Some(f) => {
                            tracing::debug!(
                                "glEGLImageTargetTexture2DOES found at address {:#?}",
                                f
                            );
                            f
                        }
                        None => {
                            tracing::error!("glEGLImageTargetTexture2DOES not found");
                            return Err(Error::EGLImageToTexProcNotFoundError);
                        }
                    },
                );

                gl_egl_image_texture_target_2d_oes(gl::TEXTURE_2D, image.as_ptr());
            }
        }

        Ok(())
    }
}
