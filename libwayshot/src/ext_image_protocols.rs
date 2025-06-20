use wayland_client::{EventQueue, WEnum};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1, FailureReason},
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use tracing::debug;

use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};

use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop, event_created_child,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::WlBuffer,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
    },
};

use wayland_client::protocol::{wl_compositor::WlCompositor, wl_surface::WlSurface};

use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

use std::sync::{Arc, OnceLock, RwLock};

use std::ops::Sub;

use wayland_protocols::{
    ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::Options,
    wp::viewporter::client::wp_viewporter::WpViewporter,
};

use image::ColorType;
use memmap2::MmapMut;

use std::os::fd::{AsFd, AsRawFd};

use std::{
    fs::File,
    time::{SystemTime, UNIX_EPOCH},
};

use std::{ops::Deref, os::fd::OwnedFd};

use std::io;
use thiserror::Error;
use wayland_client::{
    ConnectError, DispatchError,
    globals::{BindError, GlobalError},
};

use crate::region::{LogicalRegion, Position, Region, Size};

/// This main state of HaruhiShot, We use it to do screen copy
#[derive(Debug, Default)]
pub struct HaruhiShotState {
    toplevels: Vec<TopLevel>,
    output_infos: Vec<WlOutputInfo>,
    img_copy_manager: OnceLock<ExtImageCopyCaptureManagerV1>,
    output_image_manager: OnceLock<ExtOutputImageCaptureSourceManagerV1>,
    shm: OnceLock<WlShm>,
    qh: OnceLock<QueueHandle<Self>>,
    event_queue: Option<EventQueue<Self>>,
    conn: OnceLock<Connection>,
    globals: OnceLock<GlobalList>,
}

/// Image view means what part to use
/// When use the project, every time you will get a picture of the full screen,
/// and when you do area screenshot, This lib will also provide you with the view of the selected
/// part
#[derive(Debug, Clone)]
pub struct ImageViewInfo {
    pub info: ImageInfo,
    pub region: Region,
}

/// The data of the image, for the whole screen
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub color_type: ColorType,
}

#[allow(unused)]
#[derive(Debug, Clone)]
struct CaptureOutputData {
    output: WlOutput,
    buffer: WlBuffer,
    real_width: u32,
    real_height: u32,
    width: u32,
    height: u32,
    frame_bytes: u32,
    stride: u32,
    transform: wl_output::Transform,
    frame_format: Format,
    screen_position: Position,
}

/// This describe the error happens during screenshot
#[derive(Error, Debug)]
pub enum HaruhiError {
    #[error("Init Failed connection")]
    InitFailedConnection(#[from] ConnectError),
    #[error("Init Failed Global")]
    InitFailedGlobal(#[from] GlobalError),
    #[error("Dispatch Error")]
    DispatchError(#[from] DispatchError),
    #[error("Error during queue")]
    BindError(#[from] BindError),
    #[error("Error in write image in shm")]
    ShmError(#[from] io::Error),
    #[error("Not Support format")]
    NotSupportFormat,
    #[error("Capture Failed")]
    CaptureFailed(String),
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) title: String,
}

impl TopLevel {
    pub(crate) fn new(handle: ExtForeignToplevelHandleV1) -> Self {
        Self {
            handle,
            title: "".to_string(),
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn handle(&self) -> &ExtForeignToplevelHandleV1 {
        &self.handle
    }
}

#[derive(Debug, Default)]
pub(crate) struct FrameInfo {
    buffer_size: OnceLock<Size>,
    shm_format: OnceLock<WEnum<Format>>,
}

impl FrameInfo {
    pub(crate) fn size(&self) -> Size {
        self.buffer_size.get().cloned().expect("not inited")
    }

    pub(crate) fn format(&self) -> WEnum<Format> {
        self.shm_format.get().cloned().expect("Not inited")
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CaptureState {
    Failed(WEnum<FailureReason>),
    Succeeded,
    Pending,
}

pub(crate) struct CaptureInfo {
    transform: wl_output::Transform,
    state: CaptureState,
}

impl CaptureInfo {
    pub(crate) fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            transform: wl_output::Transform::Normal,
            state: CaptureState::Pending,
        }))
    }

    pub(crate) fn transform(&self) -> wl_output::Transform {
        self.transform
    }
    pub(crate) fn state(&self) -> CaptureState {
        self.state
    }
}

pub trait AreaSelectCallback {
    fn slurp(self, state: &HaruhiShotState) -> Result<Region, HaruhiError>;
}

impl<F> AreaSelectCallback for F
where
    F: Fn(&HaruhiShotState) -> Result<Region, HaruhiError>,
{
    fn slurp(self, state: &HaruhiShotState) -> Result<Region, HaruhiError> {
        self(state)
    }
}
impl AreaSelectCallback for Region {
    fn slurp(self, _state: &HaruhiShotState) -> Result<Region, HaruhiError> {
        Ok(self)
    }
}

use std::collections::HashSet;

#[derive(Debug)]
pub(crate) struct LayerShellState {
    pub configured_outputs: HashSet<WlOutput>,
}

impl LayerShellState {
    pub(crate) fn new() -> Self {
        Self {
            configured_outputs: HashSet::new(),
        }
    }
}

/// contain the output and their messages
#[derive(Debug, Clone)]
pub struct WlOutputInfo {
    pub(crate) output: WlOutput,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) transform: wl_output::Transform,
    pub(crate) physical_size: Size,

    pub(crate) logical_region: LogicalRegion,

    pub(crate) xdg_output: Option<ZxdgOutputV1>,
    pub(crate) scale: i32,
}

impl WlOutputInfo {
    /// The name of the output or maybe the screen?
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn new(output: WlOutput) -> Self {
        Self {
            output,
            logical_region: LogicalRegion::default(),
            physical_size: Size::default(),
            name: "".to_owned(),
            description: "".to_owned(),
            xdg_output: None,
            transform: wl_output::Transform::Normal,
            scale: 1,
        }
    }
}

/// Describe the capture option
/// Now this library provide two options
/// [CaptureOption::PaintCursors] and [CaptureOption::None]
/// It decides whether cursor will be shown
#[derive(Debug, Clone, Copy)]
pub enum CaptureOption {
    PaintCursors,
    None,
}

impl From<CaptureOption> for Options {
    fn from(val: CaptureOption) -> Self {
        match val {
            CaptureOption::None => Options::empty(),
            CaptureOption::PaintCursors => Options::PaintCursors,
        }
    }
}

struct AreaShotInfo {
    data: CaptureOutputData,
    mem_file: File,
}

impl AreaShotInfo {
    fn in_this_screen(
        &self,
        Region {
            position: point, ..
        }: Region,
    ) -> bool {
        let CaptureOutputData {
            real_width,
            real_height,
            screen_position: Position { x, y },
            ..
        } = self.data;
        if point.y < y
            || point.x < x
            || point.x > x + real_width as i32
            || point.y > y + real_height as i32
        {
            return false;
        }
        true
    }
    fn clip_area(&self, region: Region) -> Option<Region> {
        if !self.in_this_screen(region) {
            return None;
        }
        let CaptureOutputData {
            real_width,
            real_height,
            width,
            height,
            screen_position,
            ..
        } = self.data;
        let Region {
            position: point,
            size,
        } = region;
        let relative_point = point - screen_position;
        let position = Position {
            x: (relative_point.x as f64 * width as f64 / real_width as f64) as i32,
            y: (relative_point.y as f64 * height as f64 / real_height as f64) as i32,
        };

        Some(Region {
            position,
            size: Size {
                width: (size.width as f64 * width as f64 / real_width as f64) as u32,
                height: (size.height as f64 * height as f64 / real_height as f64) as u32,
            },
        })
    }
}

// Replace the layer shell imports with xdg_shell imports
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

#[derive(Debug)]
pub(crate) struct XdgShellState {
    pub configured_surfaces: HashSet<XdgSurface>,
}

impl XdgShellState {
    pub(crate) fn new() -> Self {
        Self {
            configured_surfaces: HashSet::new(),
        }
    }
}

// Replace the LayerShellState dispatch implementations with XdgShell ones
delegate_noop!(XdgShellState: ignore WlCompositor);
delegate_noop!(XdgShellState: ignore WlShm);
delegate_noop!(XdgShellState: ignore WlShmPool);
delegate_noop!(XdgShellState: ignore WlBuffer);
//delegate_noop!(XdgShellState: ignore XdgWmBase);
delegate_noop!(XdgShellState: ignore WlSurface);
delegate_noop!(XdgShellState: ignore WpViewport);
delegate_noop!(XdgShellState: ignore WpViewporter);
delegate_noop!(XdgShellState: ignore XdgToplevel);

impl Dispatch<XdgSurface, WlOutput> for XdgShellState {
    fn event(
        state: &mut Self,
        proxy: &XdgSurface,
        event: <XdgSurface as Proxy>::Event,
        _data: &WlOutput,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                tracing::debug!("Acking XDG surface configure");
                state.configured_surfaces.insert(proxy.clone());
                proxy.ack_configure(serial);
                tracing::trace!("Acked XDG surface configure");
            }
            _ => {}
        }
    }
}

// Add XdgWmBase ping handling
impl Dispatch<XdgWmBase, ()> for XdgShellState {
    fn event(
        _state: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                proxy.pong(serial);
            }
            _ => {}
        }
    }
}

impl HaruhiShotState {
    /// get all outputs and their info
    pub fn outputs(&self) -> &Vec<WlOutputInfo> {
        &self.output_infos
    }

    pub fn new() -> Result<Self, HaruhiError> {
        Self::init(None)
    }

    pub(crate) fn take_event_queue(&mut self) -> EventQueue<Self> {
        self.event_queue.take().expect("control your self")
    }

    pub(crate) fn output_image_manager(&self) -> &ExtOutputImageCaptureSourceManagerV1 {
        self.output_image_manager.get().expect("Should init")
    }

    pub(crate) fn image_copy_capture_manager(&self) -> &ExtImageCopyCaptureManagerV1 {
        self.img_copy_manager.get().expect("Should init")
    }

    pub(crate) fn qhandle(&self) -> &QueueHandle<Self> {
        self.qh.get().expect("Should init")
    }

    pub fn new_with_connection(connection: Connection) -> Result<Self, HaruhiError> {
        Self::init(Some(connection))
    }

    pub(crate) fn shm(&self) -> &WlShm {
        self.shm.get().expect("Should init")
    }

    pub(crate) fn reset_event_queue(&mut self, event_queue: EventQueue<Self>) {
        self.event_queue = Some(event_queue);
    }

    pub fn connection(&self) -> &Connection {
        self.conn.get().expect("should init")
    }

    pub fn globals(&self) -> &GlobalList {
        self.globals.get().expect("should init")
    }

    fn init(connection: Option<Connection>) -> Result<Self, HaruhiError> {
        let conn = if let Some(conn) = connection {
            conn
        } else {
            Connection::connect_to_env()?
        };

        let (globals, mut event_queue) = registry_queue_init::<HaruhiShotState>(&conn)?; // We just need the
        let display = conn.display();

        let mut state = HaruhiShotState::default();

        let qh = event_queue.handle();

        let _registry = display.get_registry(&qh, ());
        event_queue.blocking_dispatch(&mut state)?;
        let image_manager = globals.bind::<ExtImageCopyCaptureManagerV1, _, _>(&qh, 1..=1, ())?;
        let output_image_manager =
            globals.bind::<ExtOutputImageCaptureSourceManagerV1, _, _>(&qh, 1..=1, ())?;
        let shm = globals.bind::<WlShm, _, _>(&qh, 1..=2, ())?;
        globals.bind::<ExtForeignToplevelListV1, _, _>(&qh, 1..=1, ())?;
        let the_xdg_output_manager = globals.bind::<ZxdgOutputManagerV1, _, _>(&qh, 3..=3, ())?;

        for output in state.output_infos.iter_mut() {
            let xdg_the_output = the_xdg_output_manager.get_xdg_output(&output.output, &qh, ());
            output.xdg_output = Some(xdg_the_output);
        }

        event_queue.blocking_dispatch(&mut state)?;

        state.img_copy_manager.set(image_manager).unwrap();
        state
            .output_image_manager
            .set(output_image_manager)
            .unwrap();
        state.qh.set(qh).unwrap();
        state.shm.set(shm).unwrap();
        state.globals.set(globals).unwrap();
        state.conn.set(conn).unwrap();
        state.event_queue = Some(event_queue);
        Ok(state)
    }

    /// Capture a single output
    pub fn ext_capture_single_output(
        &mut self,
        option: CaptureOption,
        output: WlOutputInfo,
    ) -> Result<ImageInfo, HaruhiError> {
        let mem_fd = ext_create_shm_fd().unwrap();
        let mem_file = File::from(mem_fd);
        let CaptureOutputData {
            width,
            height,
            frame_format,
            ..
        } = self.ext_capture_output_inner(output, option, mem_file.as_fd(), Some(&mem_file))?;

        let mut frame_mmap = unsafe { MmapMut::map_mut(&mem_file).unwrap() };

        let converter = crate::convert::create_converter(frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageInfo {
            data: frame_mmap.deref().into(),
            width,
            height,
            color_type,
        })
    }

    fn ext_capture_output_inner<T: AsFd>(
        &mut self,
        WlOutputInfo {
            output,
            logical_region:
                LogicalRegion {
                    inner:
                        Region {
                            position: screen_position,
                            size:
                                Size {
                                    width: real_width,
                                    height: real_height,
                                },
                        },
                },
            //			logical_size:
            //                Size {
            //                    width: real_width,
            //                    height: real_height,
            //                },
            //            position: screen_position,
            ..
        }: WlOutputInfo,
        option: CaptureOption,
        fd: T,
        file: Option<&File>,
    ) -> Result<CaptureOutputData, HaruhiError> {
        let mut event_queue = self.take_event_queue();
        let img_manager = self.output_image_manager();
        let capture_manager = self.image_copy_capture_manager();
        let qh = self.qhandle();

        let source = img_manager.create_source(&output, qh, ());
        let info = Arc::new(RwLock::new(FrameInfo::default()));
        let session = capture_manager.create_session(&source, option.into(), qh, info.clone());

        let capture_info = CaptureInfo::new();
        let frame = session.create_frame(qh, capture_info.clone());
        event_queue.blocking_dispatch(self).unwrap();
        let qh = self.qhandle();

        let shm = self.shm();
        let info = info.read().unwrap();

        let Size { width, height } = info.size();
        let WEnum::Value(frame_format) = info.format() else {
            return Err(HaruhiError::NotSupportFormat);
        };
        if !matches!(
            frame_format,
            Format::Xbgr2101010
                | Format::Abgr2101010
                | Format::Argb8888
                | Format::Xrgb8888
                | Format::Xbgr8888
        ) {
            return Err(HaruhiError::NotSupportFormat);
        }
        let frame_bytes = 4 * height * width;
        let mem_fd = fd.as_fd();

        if let Some(file) = file {
            file.set_len(frame_bytes as u64).unwrap();
        }

        let stride = 4 * width;

        let shm_pool = shm.create_pool(mem_fd, (width * height * 4) as i32, qh, ());
        let buffer = shm_pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            frame_format,
            qh,
            (),
        );
        frame.attach_buffer(&buffer);
        frame.capture();

        let transform;
        loop {
            event_queue.blocking_dispatch(self)?;
            let info = capture_info.read().unwrap();
            match info.state() {
                CaptureState::Succeeded => {
                    transform = info.transform();
                    break;
                }
                CaptureState::Failed(info) => match info {
                    WEnum::Value(reason) => match reason {
                        FailureReason::Stopped => {
                            return Err(HaruhiError::CaptureFailed("Stopped".to_owned()));
                        }

                        FailureReason::BufferConstraints => {
                            return Err(HaruhiError::CaptureFailed("BufferConstraints".to_owned()));
                        }
                        FailureReason::Unknown | _ => {
                            return Err(HaruhiError::CaptureFailed("Unknown".to_owned()));
                        }
                    },
                    WEnum::Unknown(code) => {
                        return Err(HaruhiError::CaptureFailed(format!(
                            "Unknown reason, code : {code}"
                        )));
                    }
                },
                CaptureState::Pending => {}
            }
        }

        self.reset_event_queue(event_queue);

        Ok(CaptureOutputData {
            output,
            buffer,
            width,
            height,
            frame_bytes,
            stride,
            frame_format,
            real_width: real_width as u32,
            real_height: real_height as u32,
            transform,
            screen_position,
        })
    }

    pub fn ext_capture_area2<F>(
        &mut self,
        option: CaptureOption,
        callback: F,
    ) -> Result<ImageViewInfo, HaruhiError>
    where
        F: AreaSelectCallback,
    {
        let outputs = self.outputs().clone();

        let mut data_list = vec![];
        for data in outputs.into_iter() {
            let mem_fd = ext_create_shm_fd().unwrap();
            let mem_file = File::from(mem_fd);
            let data =
                self.ext_capture_output_inner(data, option, mem_file.as_fd(), Some(&mem_file))?;
            data_list.push(AreaShotInfo { data, mem_file })
        }

        let mut state = XdgShellState::new();
        let mut event_queue: EventQueue<XdgShellState> = self.connection().new_event_queue();
        let globals = self.globals();
        let qh = event_queue.handle();

        let compositor = globals.bind::<WlCompositor, _, _>(&qh, 3..=3, ())?;
        let xdg_wm_base = globals.bind::<XdgWmBase, _, _>(&qh, 1..=1, ())?;
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ())?;

        let mut xdg_surfaces: Vec<(WlSurface, XdgSurface, XdgToplevel)> =
            Vec::with_capacity(data_list.len());
        for AreaShotInfo { data, .. } in data_list.iter() {
            let CaptureOutputData {
                output,
                buffer,
                real_width,
                real_height,
                transform,
                ..
            } = data;
            let surface = compositor.create_surface(&qh, ());

            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, output.clone());
            let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());

            // Configure the toplevel to be fullscreen on the specific output
            xdg_toplevel.set_fullscreen(Some(output));
            xdg_toplevel.set_title("wayshot-overlay".to_string());
            xdg_toplevel.set_app_id("wayshot".to_string());

            debug!("Committing surface creation changes.");
            surface.commit();

            debug!("Waiting for layer surface to be configured.");
            while !state.configured_surfaces.contains(&xdg_surface) {
                event_queue.blocking_dispatch(&mut state)?;
            }

            surface.set_buffer_transform(*transform);
            // surface.set_buffer_scale(output_info.scale());
            surface.attach(Some(buffer), 0, 0);

            let viewport = viewporter.get_viewport(&surface, &qh, ());
            viewport.set_destination(*real_width as i32, *real_height as i32);

            debug!("Committing surface with attached buffer.");
            surface.commit();
            xdg_surfaces.push((surface, xdg_surface, xdg_toplevel));
            event_queue.blocking_dispatch(&mut state)?;
        }

        let region_re = callback.slurp(self);

        debug!("Unmapping and destroying layer shell surfaces.");
        for (surface, xdg_surface, xdg_toplevel) in xdg_surfaces.iter() {
            surface.attach(None, 0, 0);
            surface.commit(); // unmap surface by committing a null buffer
            xdg_toplevel.destroy();
            xdg_surface.destroy();
        }
        event_queue.roundtrip(&mut state)?;
        let region = region_re?;

        let shotdata = data_list
            .iter()
            .find(|data| data.in_this_screen(region))
            .ok_or(HaruhiError::CaptureFailed("not in region".to_owned()))?;
        let area = shotdata.clip_area(region).expect("should have");
        let mut frame_mmap = unsafe { MmapMut::map_mut(&shotdata.mem_file).unwrap() };

        let converter = crate::convert::create_converter(shotdata.data.frame_format).unwrap();
        let color_type = converter.convert_inplace(&mut frame_mmap);

        Ok(ImageViewInfo {
            info: ImageInfo {
                data: frame_mmap.deref().into(),
                width: shotdata.data.width,
                height: shotdata.data.height,
                color_type,
            },
            region: area,
        })
    }
}

use nix::{
    fcntl,
    sys::{memfd, mman, stat},
    unistd,
};

/// capture_output_frame.
fn ext_create_shm_fd() -> std::io::Result<OwnedFd> {
    // Only try memfd on linux and freebsd.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    loop {
        // Create a file that closes on successful execution and seal it's operations.
        match memfd::memfd_create(
            c"wayshot",
            memfd::MFdFlags::MFD_CLOEXEC | memfd::MFdFlags::MFD_ALLOW_SEALING,
        ) {
            Ok(fd) => {
                // This is only an optimization, so ignore errors.
                // F_SEAL_SRHINK = File cannot be reduced in size.
                // F_SEAL_SEAL = Prevent further calls to fcntl().
                let _ = fcntl::fcntl(
                    fd.as_fd(),
                    fcntl::F_ADD_SEALS(
                        fcntl::SealFlag::F_SEAL_SHRINK | fcntl::SealFlag::F_SEAL_SEAL,
                    ),
                );
                return Ok(fd);
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::ENOSYS) => break,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }

    // Fallback to using shm_open.
    let sys_time = SystemTime::now();
    let mut mem_file_handle = format!(
        "/wayshot-{}",
        sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
    );
    loop {
        match mman::shm_open(
            // O_CREAT = Create file if does not exist.
            // O_EXCL = Error if create and file exists.
            // O_RDWR = Open for reading and writing.
            // O_CLOEXEC = Close on successful execution.
            // S_IRUSR = Set user read permission bit .
            // S_IWUSR = Set user write permission bit.
            mem_file_handle.as_str(),
            fcntl::OFlag::O_CREAT
                | fcntl::OFlag::O_EXCL
                | fcntl::OFlag::O_RDWR
                | fcntl::OFlag::O_CLOEXEC,
            stat::Mode::S_IRUSR | stat::Mode::S_IWUSR,
        ) {
            Ok(fd) => match mman::shm_unlink(mem_file_handle.as_str()) {
                Ok(_) => return Ok(fd),
                Err(errno) => match unistd::close(fd.as_raw_fd()) {
                    Ok(_) => return Err(std::io::Error::from(errno)),
                    Err(errno) => return Err(std::io::Error::from(errno)),
                },
            },
            Err(nix::errno::Errno::EEXIST) => {
                // If a file with that handle exists then change the handle
                mem_file_handle = format!(
                    "/wayshot-{}",
                    sys_time.duration_since(UNIX_EPOCH).unwrap().subsec_nanos()
                );
                continue;
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(errno) => return Err(std::io::Error::from(errno)),
        }
    }
}

delegate_noop!(HaruhiShotState: ignore ExtImageCaptureSourceV1);
delegate_noop!(HaruhiShotState: ignore ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(HaruhiShotState: ignore WlShm);
delegate_noop!(HaruhiShotState: ignore ZxdgOutputManagerV1);
delegate_noop!(HaruhiShotState: ignore ExtImageCopyCaptureManagerV1);
delegate_noop!(HaruhiShotState: ignore WlBuffer);
delegate_noop!(HaruhiShotState: ignore WlShmPool);

impl Dispatch<WlRegistry, GlobalListContents> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: <ExtForeignToplevelListV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            state.toplevels.push(TopLevel::new(toplevel));
        }
    }
    event_created_child!(HaruhiShotState, ExtForeignToplevelHandleV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        toplevel: &ExtForeignToplevelHandleV1,
        event: <ExtForeignToplevelHandleV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let ext_foreign_toplevel_handle_v1::Event::Title { title } = event else {
            return;
        };
        let Some(current_info) = state
            .toplevels
            .iter_mut()
            .find(|my_toplevel| my_toplevel.handle == *toplevel)
        else {
            return;
        };
        current_info.title = title;
    }
}

impl Dispatch<ZxdgOutputV1, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &ZxdgOutputV1,
        event: <ZxdgOutputV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(data) =
            state
                .output_infos
                .iter_mut()
                .find(|WlOutputInfo { xdg_output, .. }| {
                    xdg_output.as_ref().expect("we need to init here") == proxy
                })
        else {
            return;
        };

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                data.logical_region.inner.position = Position { x, y }
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                data.logical_region.inner.size = Size {
                    width: width as u32,
                    height: height as u32,
                }
            }
            zxdg_output_v1::Event::Description { description } => {
                data.description = description;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, Arc<RwLock<CaptureInfo>>> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureFrameV1,
        event: <ExtImageCopyCaptureFrameV1 as Proxy>::Event,
        data: &Arc<RwLock<CaptureInfo>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut data = data.write().unwrap();
        match event {
            ext_image_copy_capture_frame_v1::Event::Ready => {
                data.state = CaptureState::Succeeded;
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                data.state = CaptureState::Failed(reason)
            }
            ext_image_copy_capture_frame_v1::Event::Transform {
                transform: WEnum::Value(transform),
            } => {
                data.transform = transform;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, Arc<RwLock<FrameInfo>>> for HaruhiShotState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureSessionV1,
        event: <ExtImageCopyCaptureSessionV1 as Proxy>::Event,
        data: &Arc<RwLock<FrameInfo>>,
        _conn: &Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let frame_info = data.write().unwrap();
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                if frame_info.buffer_size.get().is_none() {
                    frame_info.buffer_size.set(Size { width, height }).unwrap();
                }
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                if frame_info.shm_format.get().is_none() {
                    frame_info.shm_format.set(format).unwrap();
                }
            }
            ext_image_copy_capture_session_v1::Event::Done => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == WlOutput::interface().name {
                state
                    .output_infos
                    .push(WlOutputInfo::new(proxy.bind(name, version, qh, ())));
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for HaruhiShotState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let Some(data) = state
            .output_infos
            .iter_mut()
            .find(|WlOutputInfo { output, .. }| output == proxy)
        else {
            return;
        };
        match event {
            wl_output::Event::Name { name } => {
                data.name = name;
            }
            wl_output::Event::Scale { factor } => {
                data.scale = factor;
            }
            wl_output::Event::Mode { width, height, .. } => {
                data.physical_size = Size {
                    width: width as u32,
                    height: height as u32,
                };
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                data.transform = transform;
            }
            _ => {}
        }
    }
}

use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;

delegate_noop!(LayerShellState: ignore WlCompositor);
delegate_noop!(LayerShellState: ignore WlShm);
delegate_noop!(LayerShellState: ignore WlShmPool);
delegate_noop!(LayerShellState: ignore WlBuffer);
delegate_noop!(LayerShellState: ignore WlSurface);
delegate_noop!(LayerShellState: ignore WpViewport);
delegate_noop!(LayerShellState: ignore WpViewporter);
