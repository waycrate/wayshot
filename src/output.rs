use smithay_client_toolkit::reexports::client::{
    protocol::wl_output, protocol::wl_output::WlOutput, Display, GlobalManager,
};
use std::{cell::RefCell, process::exit, rc::Rc};

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub wl_output: *mut WlOutput,
    pub name: String,
}

pub fn get_all_outputs(display: Display) -> Vec<OutputInfo> {
    // Connecting to wayland environment.
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let outputs: Rc<RefCell<Vec<OutputInfo>>> = Rc::new(RefCell::new(Vec::new()));

    // Instantiating the global manager.
    let globals = GlobalManager::new(&attached_display);
    event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

    globals
        .instantiate_exact::<WlOutput>(4)
        .expect("Failed to bind to wl_output global.")
        .quick_assign({
            let outputs = outputs.clone();
            move |output, event, _| {
                if let wl_output::Event::Name { name } = event {
                    outputs.borrow_mut().push(OutputInfo {
                        wl_output: &mut output.detach(),
                        name,
                    });
                }
            }
        });
    event_queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();
    let x = outputs.borrow().to_vec();
    x
}

/// Get a wl_output object from the output name.
pub fn get_wloutput(name: String, outputs: Vec<OutputInfo>) -> &'static WlOutput {
    for output in outputs {
        if output.name == name {
            unsafe {
                return &*output.wl_output;
            }
        }
    }
    log::error!("Error: No output of name \"{}\" was found", name);
    exit(1);
}
