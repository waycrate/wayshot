use crate::error::{Result, WaylandEGLStateError};
use crate::utils::load_shader;

use libwayshot::WayshotConnection;

use gl::types::GLuint;
use khronos_egl::{self as egl};
use std::{ffi::c_void, rc::Rc};
use wayland_client::{
    protocol::{wl_compositor, wl_display::WlDisplay, wl_surface::WlSurface},
    ConnectError, Connection, Proxy,
};
use wayland_egl::WlEglSurface;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

#[derive(Debug)]
pub struct WaylandEGLState {
    pub width: i32,
    pub height: i32,
    pub running: bool,
    pub title: String,

    pub wl_connection: Connection,
    pub wl_display: WlDisplay,
    pub wl_surface: Option<WlSurface>,

    pub egl: egl::Instance<egl::Static>,
    pub egl_window: Option<Rc<WlEglSurface>>,
    pub egl_display: Option<egl::Display>,
    pub egl_surface: Option<egl::Surface>,
    pub egl_context: Option<egl::Context>,
    pub egl_image: Option<egl::Image>,

    pub gl_program: GLuint,
    pub gl_texture: GLuint,

    pub xdg_wm_base: Option<xdg_wm_base::XdgWmBase>,
    pub xdg_surface: Option<xdg_surface::XdgSurface>,
    pub xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub wl_compositor: Option<wl_compositor::WlCompositor>,

    wayshot: WayshotConnection,
}

impl WaylandEGLState {
    #[tracing::instrument]
    pub fn new() -> Result<Self, ConnectError> {
        let server_connection = Connection::connect_to_env()?;

        Ok(Self {
            width: 1920,
            height: 1080,
            running: true,
            title: "Waymirror-EGL".into(),

            wl_connection: server_connection.clone(),
            wl_display: server_connection.display(),
            wl_surface: None,

            egl: khronos_egl::Instance::new(egl::Static),
            egl_window: None,
            egl_display: None,
            egl_surface: None,
            egl_context: None,
            egl_image: None,
            gl_program: 0,
            gl_texture: 0,

            xdg_wm_base: None,
            xdg_surface: None,
            xdg_toplevel: None,
            wl_compositor: None,
            wayshot: WayshotConnection::from_connection_with_dmabuf(
                server_connection,
                "/dev/dri/renderD128",
            )
            .unwrap(),
        })
    }

    pub fn deinit(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            gl::DeleteProgram(self.gl_program);
        }

        self.egl
            .destroy_surface(self.egl_display.unwrap(), self.egl_surface.unwrap())?;
        self.egl
            .destroy_context(self.egl_display.unwrap(), self.egl_context.unwrap())?;

        self.xdg_surface.clone().unwrap().destroy();
        self.wl_surface.clone().unwrap().destroy();

        Ok(())
    }

    pub fn init_egl(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Init gl
        gl_loader::init_gl();
        gl::load_with(|s| gl_loader::get_proc_address(s) as *const _);

        self.egl_window = Some(Rc::new(WlEglSurface::new(
            self.wl_surface.clone().unwrap().id(),
            self.width,
            self.height,
        )?));

        self.egl_display = Some(
            unsafe {
                self.egl
                    .get_display(self.wl_display.id().as_ptr() as *mut c_void)
            }
            .unwrap(),
        );

        self.egl.initialize(self.egl_display.unwrap())?;

        let attributes = [
            egl::SURFACE_TYPE,
            egl::WINDOW_BIT,
            egl::RENDERABLE_TYPE,
            egl::OPENGL_ES2_BIT,
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::NONE,
        ];

        let config = self
            .egl
            .choose_first_config(self.egl_display.unwrap(), &attributes)?
            .expect("unable to find an appropriate EGL configuration");
        self.egl_surface = Some(unsafe {
            self.egl.create_window_surface(
                self.egl_display.unwrap(),
                config,
                self.egl_window.clone().unwrap().ptr() as egl::NativeWindowType,
                None,
            )?
        });

        let context_attributes = [egl::CONTEXT_CLIENT_VERSION, 2, egl::NONE];
        self.egl_context = Some(self.egl.create_context(
            self.egl_display.unwrap(),
            config,
            None,
            &context_attributes,
        )?);

        self.egl.make_current(
            self.egl_display.unwrap(),
            self.egl_surface,
            self.egl_surface,
            self.egl_context,
        )?;

        self.init_program()?;

        Ok(())
    }

    fn init_program(&mut self) -> Result<()> {
        let vert_shader = load_shader(
            gl::VERTEX_SHADER,
            include_str!("./shaders/vert.glsl").into(),
        )
        .unwrap();

        let frag_shader = load_shader(
            gl::FRAGMENT_SHADER,
            include_str!("./shaders/frag.glsl").into(),
        )
        .unwrap();

        unsafe {
            self.gl_program = gl::CreateProgram();
        }

        if self.gl_program == 0 {
            tracing::event!(tracing::Level::ERROR, "glCreateProgramFailed!");
            return Err(WaylandEGLStateError::GLCreateProgramFailed);
        }

        unsafe {
            gl::AttachShader(self.gl_program, vert_shader);
            gl::AttachShader(self.gl_program, frag_shader);

            gl::LinkProgram(self.gl_program);
        }

        let mut linked: gl::types::GLint = 1;
        unsafe { gl::GetProgramiv(self.gl_program, gl::LINK_STATUS, &mut linked as *mut i32) }

        if linked > 0 {
            tracing::event!(tracing::Level::INFO, "Successfully linked the program!");
        } else {
            return Err(WaylandEGLStateError::GLLinkProgramFailed);
        }

        let vertices: [gl::types::GLfloat; 20] = [
            // positions             // texture coords
            1.0, 1.0, 0.0, 1.0, 0.0, // top right
            1.0, -1.0, 0.0, 1.0, 1.0, // bottom right
            -1.0, -1.0, 0.0, 0.0, 1.0, // bottom left
            -1.0, 1.0, 0.0, 0.0, 0.0, // top left
        ];
        let indices: [gl::types::GLint; 6] = [
            0, 1, 3, // first Triangle
            1, 2, 3, // second Triangle
        ];
        let mut vbo: GLuint = 0;
        let mut vao: GLuint = 0;
        let mut ebo: GLuint = 0;

        unsafe {
            gl::GenTextures(1, &mut self.gl_texture);

            self.dmabuf_to_texture();

            gl::GenVertexArrays(1, &mut vao as *mut u32);
            gl::GenBuffers(1, &mut vbo as *mut u32);
            gl::GenBuffers(1, &mut ebo as *mut u32);
            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<gl::types::GLfloat>())
                    as gl::types::GLsizeiptr,
                &vertices[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * std::mem::size_of::<gl::types::GLfloat>())
                    as gl::types::GLsizeiptr,
                &indices[0] as *const i32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                5 * std::mem::size_of::<gl::types::GLfloat>() as gl::types::GLint,
                0 as *const c_void,
            );
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                5 * std::mem::size_of::<gl::types::GLfloat>() as gl::types::GLint,
                (3 * std::mem::size_of::<gl::types::GLfloat>()) as *const c_void,
            );
            gl::EnableVertexAttribArray(1);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
        Ok(())
    }

    pub fn draw(&mut self) {
        unsafe {
            gl::ClearColor(1.0, 1.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            // gl::DeleteTextures(1, &mut self.gl_texture);

            gl::UseProgram(self.gl_program);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const c_void);
        }
    }

    pub fn dmabuf_to_texture(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.gl_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);

            self.wayshot
                .bind_output_frame_to_gl_texture(
                    true,
                    &self.wayshot.get_all_outputs()[0].wl_output,
                    None,
                )
                .unwrap();
        }
    }

    pub fn validate_globals(&self) -> Result<()> {
        if self.xdg_wm_base.is_none() {
            return Err(WaylandEGLStateError::XdgWmBaseMissing);
        } else if self.wl_compositor.is_none() {
            return Err(WaylandEGLStateError::WlCompositorMissing);
        }

        Ok(())
    }
}
