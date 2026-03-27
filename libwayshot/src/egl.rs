//! EGL/OpenGL-based capture path: DMA-BUF → EGLImage → GL texture.
//!
//! This module is available when the `egl` feature is enabled. It provides:
//! - [`EGLImageGuard`] – owns an EGLImage created from a DMA-BUF capture
//! - [`get_egl_display_wl`] / [`initialize_egl`] – EGL display from Wayland
//! - [`create_egl_image_from_dmabuf`] – create an EGLImage from a GBM buffer
//! - [`bind_egl_image_to_gl_texture`] – bind an EGLImage to the current GL texture (OES_EGL_image)
//! - [`create_egl_image_and_bind_to_gl_texture`] – one-shot create + bind (e.g. for screencast)

use std::os::fd::IntoRawFd;

use gbm::BufferObject;
use r_egl_wayland::{EGL_INSTALCE, WayEglTrait, r_egl as egl};
use wayland_client::protocol::wl_display::WlDisplay;

use crate::error::{Error, Result};
use crate::screencopy::DMAFrameFormat;

/// EGL display type (re-exported for API use).
pub type EglDisplay = egl::Display;

/// Guard that owns an EGLImage created from a DMA-BUF. Destroys the image on drop.
pub struct EGLImageGuard {
    pub image: egl::Image,
    pub(crate) egl_display: egl::Display,
}

impl Drop for EGLImageGuard {
    fn drop(&mut self) {
        EGL_INSTALCE
            .destroy_image(self.egl_display, self.image)
            .unwrap_or_else(|e| {
                tracing::error!("EGLimage destruction had error: {e}");
            });
    }
}

/// Obtain an EGL display from the Wayland display. Call [`initialize_egl`] before use.
pub fn get_egl_display_wl(display: &WlDisplay) -> Result<egl::Display> {
    EGL_INSTALCE.get_display_wl(display).map_err(Error::from)
}

/// Initialize EGL for the given display.
pub fn initialize_egl(display: egl::Display) -> Result<()> {
    EGL_INSTALCE
        .initialize(display)
        .map(|_| ())
        .map_err(Error::from)
}

/// Create an EGLImage from a DMA-BUF (GBM buffer object). Returns a guard that destroys the image on drop.
pub fn create_egl_image_from_dmabuf(
    egl_display: egl::Display,
    bo: &BufferObject<()>,
    frame_format: &DMAFrameFormat,
) -> Result<EGLImageGuard> {
    type Attrib = egl::Attrib;
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
    tracing::debug!(
        "Calling eglCreateImage with attributes: {:#?}",
        image_attribs
    );
    unsafe {
        let image = EGL_INSTALCE
            .create_image(
                egl_display,
                egl::Context::from_ptr(egl::NO_CONTEXT),
                egl::LINUX_DMA_BUF_EXT as u32,
                egl::ClientBuffer::from_ptr(std::ptr::null_mut()),
                &image_attribs,
            )
            .map_err(|e| {
                tracing::error!("eglCreateImage call failed with error {e}");
                Error::from(e)
            })?;
        Ok(EGLImageGuard { image, egl_display })
    }
}

/// Bind the EGLImage to the current GL texture (TEXTURE_2D) via OES_EGL_image.
/// The caller must have bound the target texture before calling.
pub fn bind_egl_image_to_gl_texture(guard: &EGLImageGuard) -> Result<()> {
    let gl_egl_image_texture_target_2d_oes = unsafe {
        let f = match EGL_INSTALCE.get_proc_address("glEGLImageTargetTexture2DOES") {
            Some(f) => {
                tracing::debug!("glEGLImageTargetTexture2DOES found at address {:#?}", f);
                f
            }
            None => {
                tracing::error!("glEGLImageTargetTexture2DOES not found");
                return Err(Error::EGLImageToTexProcNotFoundError);
            }
        };
        std::mem::transmute::<
            extern "system" fn(),
            unsafe extern "system" fn(gl::types::GLenum, gl::types::GLeglImageOES) -> (),
        >(f)
    };
    unsafe {
        gl_egl_image_texture_target_2d_oes(gl::TEXTURE_2D, guard.image.as_ptr());
        tracing::trace!("glEGLImageTargetTexture2DOES called");
    }
    Ok(())
}

/// Create an EGLImage from the DMA-BUF and bind it to the current GL texture, then destroy the EGLImage.
/// Used by screencast when updating the texture each frame.
pub fn create_egl_image_and_bind_to_gl_texture(
    egl_display: egl::Display,
    bo: &BufferObject<()>,
    frame_format: &DMAFrameFormat,
) -> Result<()> {
    let guard = create_egl_image_from_dmabuf(egl_display, bo, frame_format)?;
    bind_egl_image_to_gl_texture(&guard)?;
    drop(guard);
    Ok(())
}
