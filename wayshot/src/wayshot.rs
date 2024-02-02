use std::{
    io::{stdout, BufWriter, Cursor, Write},
    process::{exit, Command},
};

use eyre::Result;
use libwayshot::{region::LogicalRegion, WayshotConnection};

mod clap;
mod utils;

use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use tracing::Level;

use crate::utils::EncodingFormat;

fn select_ouput<T>(ouputs: &[T]) -> Option<usize>
where
    T: ToString,
{
    let Ok(selection) = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose Screen")
        .default(0)
        .items(ouputs)
        .interact()
    else {
        return None;
    };
    Some(selection)
}

fn main() -> Result<()> {
    let args = clap::set_flags().get_matches();
    let level = if args.get_flag("debug") {
        Level::TRACE
    } else {
        Level::INFO
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();

    let extension = if let Some(extension) = args.get_one::<String>("extension") {
        let ext = extension.trim().to_lowercase();
        tracing::debug!("Using custom extension: {:#?}", ext);

        match ext.as_str() {
            "jpeg" | "jpg" => EncodingFormat::Jpg,
            "png" => EncodingFormat::Png,
            "ppm" => EncodingFormat::Ppm,
            "qoi" => EncodingFormat::Qoi,
            _ => {
                tracing::error!("Invalid extension provided.\nValid extensions:\n1) jpeg\n2) jpg\n3) png\n4) ppm\n5) qoi");
                exit(1);
            }
        }
    } else {
        EncodingFormat::Png
    };

    let mut file_is_stdout: bool = false;
    let mut file_path: Option<String> = None;

    if args.get_flag("stdout") {
        file_is_stdout = true;
    } else if let Some(filepath) = args.get_one::<String>("file") {
        file_path = Some(filepath.trim().to_string());
    } else {
        file_path = Some(utils::get_default_file_name(extension));
    }

    let wayshot_conn = WayshotConnection::new()?;

    if args.get_flag("listoutputs") {
        let valid_outputs = wayshot_conn.get_all_outputs();
        for output in valid_outputs {
            tracing::info!("{:#?}", output.name);
        }
        exit(1);
    }

    let mut cursor_overlay = false;
    if args.get_flag("cursor") {
        cursor_overlay = true;
    }

    let image_buffer = if let Some(slurp_region) = args.get_one::<String>("slurp") {
        let slurp_region = slurp_region.clone();
        wayshot_conn.screenshot_freeze(
            Box::new(move || {
                || -> Result<LogicalRegion> {
                    let slurp_output = Command::new("slurp")
                        .args(slurp_region.split(" "))
                        .output()?
                        .stdout;

                    utils::parse_geometry(&String::from_utf8(slurp_output)?)
                }()
                .map_err(|_| libwayshot::Error::FreezeCallbackError)
            }),
            cursor_overlay,
        )?
    } else if let Some(output_name) = args.get_one::<String>("output") {
        let outputs = wayshot_conn.get_all_outputs();
        if let Some(output) = outputs.iter().find(|output| &output.name == output_name) {
            wayshot_conn.screenshot_single_output(output, cursor_overlay)?
        } else {
            tracing::error!("No output found!\n");
            exit(1);
        }
    } else if args.get_flag("chooseoutput") {
        let outputs = wayshot_conn.get_all_outputs();
        let output_names: Vec<String> = outputs
            .iter()
            .map(|display| display.name.to_string())
            .collect();
        if let Some(index) = select_ouput(&output_names) {
            wayshot_conn.screenshot_single_output(&outputs[index], cursor_overlay)?
        } else {
            tracing::error!("No output found!\n");
            exit(1);
        }
    } else {
        wayshot_conn.screenshot_all(cursor_overlay)?
    };

    if file_is_stdout {
        let stdout = stdout();
        let mut buffer = Cursor::new(Vec::new());

        let mut writer = BufWriter::new(stdout.lock());
        image_buffer.write_to(&mut buffer, extension)?;

        writer.write_all(buffer.get_ref())?;
    } else {
        image_buffer.save(file_path.unwrap())?;
    }

    Ok(())
}
