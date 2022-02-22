use smithay_client_toolkit::{
    environment,
    environment::Environment,
    output::{with_output_info, OutputHandler, OutputInfo, XdgOutputHandler},
    reexports::{
        client::{protocol::wl_output::WlOutput, Display},
        protocols::unstable::xdg_output::v1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1,
    },
};

struct App {
    outputs: OutputHandler,
    xdg_output: XdgOutputHandler,
}

environment! {App,
    singles = [
        ZxdgOutputManagerV1 => xdg_output,
    ],
    multis = [
        WlOutput => outputs,
    ]
}

pub fn get_valid_outputs(display: Display) -> Vec<(WlOutput, OutputInfo)> {
    let mut queue = display.create_event_queue();
    let attached_display = display.attach(queue.token());

    let (outputs, xdg_output) = XdgOutputHandler::new_output_handlers();
    let mut valid_outputs: Vec<(WlOutput, OutputInfo)> = Vec::new();

    let env = Environment::new(
        &attached_display,
        &mut &mut queue,
        App {
            outputs,
            xdg_output,
        },
    )
    .unwrap();

    queue.sync_roundtrip(&mut (), |_, _, _| {}).unwrap();

    for output in env.get_all_outputs() {
        with_output_info(&output, |info| {
            if info.obsolete == false {
                valid_outputs.push((output.clone(), info.clone()));
            } else {
                output.release();
            }
        });
    }
    valid_outputs
}
