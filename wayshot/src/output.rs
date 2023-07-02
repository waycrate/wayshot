use std::process::exit;
use wayland_client::{
    delegate_noop,
    globals::GlobalList,
    protocol::{wl_output, wl_output::WlOutput, wl_registry, wl_registry::WlRegistry},
    Connection, Dispatch, QueueHandle, WEnum,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1, zxdg_output_v1::ZxdgOutputV1,
};

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
    pub transform: wl_output::Transform,
    pub dimensions: OutputPositioning,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct OutputPositioning {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

struct OutputCaptureState {
    outputs: Vec<OutputInfo>,
}

impl Dispatch<WlRegistry, ()> for OutputCaptureState {
    fn event(
        state: &mut Self,
        wl_registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        /* > The name event is sent after binding the output object. This event
         * is only sent once per output object, and the name does not change
         * over the lifetime of the wl_output global. */

        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "wl_output" {
                if version >= 4 {
                    let output = wl_registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
                    state.outputs.push(OutputInfo {
                        wl_output: output,
                        name: "".to_string(),
                        transform: wl_output::Transform::Normal,
                        dimensions: OutputPositioning {
                            x: 0,
                            y: 0,
                            width: 0,
                            height: 0,
                        },
                    });
                } else {
                    log::error!("Ignoring a wl_output with version < 4.");
                }
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for OutputCaptureState {
    fn event(
        state: &mut Self,
        wl_output: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let output: &mut OutputInfo = state
            .outputs
            .iter_mut()
            .find(|x| x.wl_output == *wl_output)
            .unwrap();

        match event {
            wl_output::Event::Name { name } => {
                output.name = name;
            }
            wl_output::Event::Geometry {
                transform: WEnum::Value(transform),
                ..
            } => {
                output.transform = transform;
            }
            _ => (),
        }
    }
}

delegate_noop!(OutputCaptureState: ignore ZxdgOutputManagerV1);

impl Dispatch<ZxdgOutputV1, usize> for OutputCaptureState {
    fn event(
        state: &mut Self,
        _: &ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        index: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let output_info = state.outputs.get_mut(*index).unwrap();

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                output_info.dimensions.x = x;
                output_info.dimensions.y = y;
                log::debug!("Logical position event fired!");
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                output_info.dimensions.width = width;
                output_info.dimensions.height = height;
                log::debug!("Logical size event fired!");
            }
            _ => {}
        };
    }
}

pub fn get_all_outputs(globals: &mut GlobalList, conn: &mut Connection) -> Vec<OutputInfo> {
    // Connecting to wayland environment.
    let mut state = OutputCaptureState {
        outputs: Vec::new(),
    };
    let mut event_queue = conn.new_event_queue::<OutputCaptureState>();
    let qh = event_queue.handle();

    // Bind to xdg_output global.
    let zxdg_output_manager = match globals.bind::<ZxdgOutputManagerV1, _, _>(&qh, 3..=3, ()) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to create ZxdgOutputManagerV1 version 3. Does your compositor implement ZxdgOutputManagerV1?");
            panic!("{:#?}", e);
        }
    };

    // Fetch all outputs; when their names arrive, add them to the list
    let _ = conn.display().get_registry(&qh, ());
    event_queue.roundtrip(&mut state).unwrap();
    event_queue.roundtrip(&mut state).unwrap();

    let mut xdg_outputs: Vec<ZxdgOutputV1> = Vec::new();

    // We loop over each output and request its position data.
    for (index, output) in state.outputs.clone().iter().enumerate() {
        let xdg_output = zxdg_output_manager.get_xdg_output(&output.wl_output, &qh, index);
        xdg_outputs.push(xdg_output);
    }

    event_queue.roundtrip(&mut state).unwrap();

    for xdg_output in xdg_outputs {
        xdg_output.destroy();
    }

    if state.outputs.is_empty() {
        log::error!("Compositor did not advertise any wl_output devices!");
        exit(1);
    }
    log::debug!("Outputs detected: {:#?}", state.outputs);
    state.outputs
}
