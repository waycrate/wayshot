use std::{cell::RefCell, process::exit, rc::Rc};
use wayland_client::{protocol::wl_output, protocol::wl_output::WlOutput, Display, GlobalManager};
use wayland_protocols::unstable::xdg_output::v1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1,
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

pub fn get_all_outputs(display: Display) -> Vec<OutputInfo> {
    // Connecting to wayland environment.
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    // Instantiating the global manager.
    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

    let mut data: Vec<OutputInfo> = Vec::new();
    let outputs: Rc<RefCell<Vec<OutputInfo>>> = Rc::new(RefCell::new(Vec::new()));

    // Bind to xdg_output global.
    let zxdg_output_manager = match globals.instantiate_exact::<ZxdgOutputManagerV1>(3) {
        Ok(x) => x,
        Err(e) => {
            log::error!("Failed to create ZxdgOutputManagerV1 version 3. Does your compositor implement ZxdgOutputManagerV1?");
            panic!("{:#?}", e);
        }
    };

    // Fetch all outputs and it's name.
    globals
        .instantiate_exact::<WlOutput>(4)
        .expect("Failed to bind to wl_output global.")
        .quick_assign({
            let outputs = outputs.clone();
            move |output, event, _| {
                if let wl_output::Event::Name { name } = event {
                    outputs.borrow_mut().push(OutputInfo {
                        wl_output: output.detach(),
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
        });
    event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

    // We loop over each output and get it's position data.
    for mut output in outputs.borrow().iter().cloned() {
        let output_position: Rc<RefCell<OutputPositioning>> =
            Rc::new(RefCell::new(OutputPositioning {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }));

        // Callback to set X, Y, Width, and Height.
        zxdg_output_manager
            .get_xdg_output(&output.wl_output)
            .quick_assign({
                let output_position = output_position.clone();
                move |_, event, _| {
                    match event {
                        zxdg_output_v1::Event::LogicalPosition { x, y } => {
                            output_position.borrow_mut().x = x;
                            output_position.borrow_mut().y = y;
                            log::debug!("Logical position event fired!");
                        }
                        zxdg_output_v1::Event::LogicalSize { width, height } => {
                            output_position.borrow_mut().width = width;
                            output_position.borrow_mut().height = height;
                            log::debug!("Logical size event fired!");
                        }
                        _ => {}
                    };
                }
            });

        // Exhaust the internal buffer queue until we get our required data.
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .unwrap();

        // Set the output dimensions
        output.dimensions = output_position.take();
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
