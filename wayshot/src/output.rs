use std::{process::exit, sync::Arc, sync::Mutex};
use wayland_client::{
    delegate_noop,
    globals::GlobalList,
    protocol::{wl_output, wl_output::WlOutput, wl_registry, wl_registry::WlRegistry},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1, zxdg_output_v1::ZxdgOutputV1,
};

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: WlOutput,
    pub name: String,
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
        _: &mut Self,
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
                    let _ = wl_registry.bind::<wl_output::WlOutput, _, _>(name, 4, qh, ());
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
        /* > The name event is sent after binding the output object. This event
         * is only sent once per output object, and the name does not change
         * over the lifetime of the wl_output global. */
        if let wl_output::Event::Name { name } = event {
            state.outputs.push(OutputInfo {
                wl_output: wl_output.clone(),
                name,
                dimensions: OutputPositioning {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
            });
        }
    }
}

delegate_noop!(OutputCaptureState: ignore ZxdgOutputManagerV1);

impl Dispatch<ZxdgOutputV1, Arc<Mutex<OutputPositioning>>> for OutputCaptureState {
    fn event(
        _: &mut Self,
        _: &ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        outpos: &Arc<Mutex<OutputPositioning>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Ok(mut output_positioning) = outpos.lock() {
            match event {
                zxdg_output_v1::Event::LogicalPosition { x, y } => {
                    output_positioning.x = x;
                    output_positioning.y = y;
                    log::debug!("Logical position event fired!");
                }
                zxdg_output_v1::Event::LogicalSize { width, height } => {
                    output_positioning.width = width;
                    output_positioning.height = height;
                    log::debug!("Logical size event fired!");
                }
                _ => {}
            };
        }
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

    // We loop over each output and get its position data.
    let mut data: Vec<OutputInfo> = Vec::new();
    for mut output in state.outputs.clone() {
        let output_position: Arc<Mutex<OutputPositioning>> =
            Arc::new(Mutex::new(OutputPositioning {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }));

        let xdg_output =
            zxdg_output_manager.get_xdg_output(&output.wl_output, &qh, output_position.clone());
        event_queue.roundtrip(&mut state).unwrap();
        xdg_output.destroy();

        // Set the output dimensions
        output.dimensions = output_position.lock().unwrap().clone();
        data.push(output);
    }
    if data.is_empty() {
        log::error!("Compositor did not advertise any wl_output devices!");
        exit(1);
    }
    log::debug!("Outputs detected: {:#?}", data);
    data
}

/// Get a wl_output object from the output name.
pub fn get_wloutput(name: String, outputs: Vec<OutputInfo>) -> WlOutput {
    for output in outputs {
        if output.name == name {
            return output.wl_output;
        }
    }
    log::error!("Error: No output of name \"{}\" was found", name);
    exit(1);
}
