use crate::error::{Result, WaylandEGLStateError};
use crate::utils::load_shader;

use libwayshot::screencast::WayshotScreenCast;
use libwayshot::{WayshotConnection, WayshotTarget};

use gl::types::GLuint;
use r_egl_wayland::EGL_INSTALCE;
use r_egl_wayland::{WayEglTrait, r_egl as egl};
use std::ffi::c_void;
use std::time::{Duration, Instant};
use wayland_client::EventQueue;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_seat;
use wayland_client::{
    Connection, Proxy,
    protocol::{wl_compositor, wl_surface::WlSurface},
};
use wayland_egl::WlEglSurface;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_wm_base};
#[derive(Debug)]
pub struct WaylandEGLState {
    pub width: i32,
    pub height: i32,
    pub running: bool,

    pub wl_surface: WlSurface,

    pub egl_window: WlEglSurface,
    pub egl_display: egl::Display,
    pub egl_surface: egl::Surface,
    pub egl_context: egl::Context,

    pub gl_program: GLuint,
    pub gl_texture: GLuint,

    pub xdg_surface: xdg_surface::XdgSurface,

    wayshot: WayshotConnection,
    cast: WayshotScreenCast,
    pub instant: Instant,
}

fn init_cast(
    connection: &libwayshot::WayshotConnection,
    target: WayshotTarget,
    gl_texture: GLuint,
    egl_display: egl::Display,
) -> WayshotScreenCast {
    unsafe {
        gl::BindTexture(gl::TEXTURE_2D, gl_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    }
    connection
        .create_screencast_with_egl(target, true, None, egl_display)
        .unwrap()
}

impl WaylandEGLState {
    #[tracing::instrument]
    pub fn new() -> Result<(Self, EventQueue<Self>), WaylandEGLStateError> {
        let server_connection = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<Self>(&server_connection)?;
        let qhandle = event_queue.handle();
        let compositor = globals
            .bind::<wl_compositor::WlCompositor, _, _>(&qhandle, 3..=3, ())
            .unwrap();
        let wl_surface = compositor.create_surface(&qhandle, ());

        globals
            .bind::<wl_seat::WlSeat, _, _>(&qhandle, 1..=1, ())
            .unwrap();

        let wm_base = globals
            .bind::<xdg_wm_base::XdgWmBase, _, _>(&qhandle, 2..=6, ())
            .unwrap();
        let xdg_surface = wm_base.get_xdg_surface(&wl_surface, &qhandle, ());

        let toplevel = xdg_surface.get_toplevel(&qhandle, ());
        toplevel.set_title("Waymirror-EGL".into());
        wl_surface.commit();

        // Init gl
        gl_loader::init_gl();
        gl::load_with(|s| gl_loader::get_proc_address(s) as *const _);

        let width = 1920;
        let height = 1080;
        let egl_window = WlEglSurface::new(wl_surface.id(), width, height)?;
        let wl_display = server_connection.display();
        let egl_display = EGL_INSTALCE.get_display_wl(&wl_display).unwrap();

        EGL_INSTALCE.initialize(egl_display)?;

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

        let config = EGL_INSTALCE
            .choose_first_config(egl_display, &attributes)?
            .expect("unable to find an appropriate EGL configuration");
        let egl_surface = unsafe {
            EGL_INSTALCE.create_window_surface(
                egl_display,
                config,
                egl_window.ptr() as egl::NativeWindowType,
                None,
            )?
        };

        let context_attributes = [egl::CONTEXT_CLIENT_VERSION, 2, egl::NONE];
        let egl_context =
            EGL_INSTALCE.create_context(egl_display, config, None, &context_attributes)?;

        EGL_INSTALCE.make_current(
            egl_display,
            Some(egl_surface),
            Some(egl_surface),
            Some(egl_context),
        )?;

        let wayshot = WayshotConnection::from_connection_with_dmabuf(
            server_connection,
            "/dev/dri/renderD128",
        )
        .unwrap();
        let target = WayshotTarget::Screen(wayshot.get_all_outputs()[0].wl_output.clone());
        let cast = init_cast(&wayshot, target, 0, egl_display);
        Ok((
            Self {
                width: 1920,
                height: 1080,
                running: true,
                wl_surface,

                egl_window,
                egl_display,
                egl_surface,
                egl_context,
                gl_program: 0,
                gl_texture: 0,

                xdg_surface,
                wayshot,
                instant: Instant::now()
                    .checked_add(Duration::from_millis(10))
                    .unwrap(),
                cast,
            },
            event_queue,
        ))
    }

    pub fn deinit(&self) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            gl::DeleteProgram(self.gl_program);
        }

        EGL_INSTALCE.destroy_surface(self.egl_display, self.egl_surface)?;
        EGL_INSTALCE.destroy_context(self.egl_display, self.egl_context)?;

        self.xdg_surface.destroy();
        self.wl_surface.destroy();

        Ok(())
    }

    pub fn init_program(&mut self) -> Result<()> {
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
                std::ptr::null::<c_void>(),
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
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null::<c_void>(),
            );
        }
    }

    pub fn cast(&mut self) {
        let _ = self.wayshot.screencast(&mut self.cast);
    }
}
