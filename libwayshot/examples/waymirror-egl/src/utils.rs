use crate::error::{Result, WaylandEGLStateError};
use gl::types::{GLenum, GLint, GLuint};
use std::{ffi::CString, ptr};

pub fn load_shader(shader_type: GLenum, src: String) -> Result<GLuint> {
    unsafe {
        let shader: GLuint = gl::CreateShader(shader_type);

        if shader == 0 {
            return Err(WaylandEGLStateError::GLShaderCompileFailed);
        }

        let src_c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &src_c_str.as_ptr(), ptr::null());

        //gl::ShaderSource(shader, 1, &src_c_str.as_ptr(), std::ptr::null());

        gl::CompileShader(shader);

        let mut status: GLint = 1;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status as *mut i32);

        if status > 0 {
            tracing::event!(tracing::Level::INFO, "Shader compile successfull!",);
        } else {
            return Err(WaylandEGLStateError::GLShaderCompileFailed);
        }

        Ok(shader)
    }
}
