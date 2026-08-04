use std::io::{self, BufWriter, IsTerminal, Write};
use std::time::Duration;

use clap::Parser;
use eyre::Result;
use libwayshot::{LogicalRegion, OutputInfo, Size, WayshotConnection, region};

mod cli;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "color_picker")]
mod color_picker;
mod config;
mod listing;
#[cfg(feature = "logger")]
mod logger;
#[cfg(feature = "notifications")]
mod notification;
mod screenshot;
mod settings;
mod utils;

use crate::listing::{DisplayInfo, PositionInfo, SizeInfo, ToplevelInfo};
use config::Config;
use settings::{AppSettings, Command};

/// print the displays' info
pub fn print_displays_info(conn: &WayshotConnection) {
    for OutputInfo {
        physical_size: Size { width, height },
        logical_region:
            LogicalRegion {
                inner:
                    region::Region {
                        position: region::Position { x, y },
                        size:
                            Size {
                                width: logical_width,
                                height: logical_height,
                            },
                    },
            },
        name,
        description,
        ..
    } in conn.get_all_outputs()
    {
        println!("{name}");
        println!("description: {description}");
        println!("    Size: {width},{height}");
        println!("    LogicSize: {logical_width}, {logical_height}");
        println!("    Position: {x}, {y}");
    }
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    #[cfg(feature = "completions")]
    if let Some(shell) = cli.completions {
        utils::print_completions(shell);
        return Ok(());
    }

    let config_path = cli.config.clone().unwrap_or(Config::get_default_path());
    let config = Config::load(&config_path).unwrap_or_default();

    #[cfg(feature = "logger")]
    logger::setup(&cli, &config);

    let settings = AppSettings::resolve(&cli, &config);

    let connection = WayshotConnection::new()?;
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    match settings.command {
        Command::ListOutputs => {
            for output in connection.get_all_outputs() {
                writeln!(writer, "{}", output.name)?;
            }
            writer.flush()?;
            Ok(())
        }
        Command::ListOutputsInfo => {
            print_displays_info(&connection);
            Ok(())
        }
        Command::ListOutputsJson => {
            let outputs: Vec<DisplayInfo> = connection
                .get_all_outputs()
                .iter()
                .map(|output| DisplayInfo {
                    name: output.name.clone(),
                    description: output.description.clone(),
                    size: SizeInfo {
                        width: output.physical_size.width,
                        height: output.physical_size.height,
                    },
                    logical_size: SizeInfo {
                        width: output.logical_region.inner.size.width,
                        height: output.logical_region.inner.size.height,
                    },
                    position: PositionInfo {
                        x: output.logical_region.inner.position.x,
                        y: output.logical_region.inner.position.y,
                    },
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&outputs)?);
            Ok(())
        }
        Command::ListToplevels => {
            for tl in connection.get_all_toplevels().iter().filter(|t| t.active) {
                writeln!(writer, "{}", tl.id_title_identifier())?;
            }
            writer.flush()?;
            Ok(())
        }
        Command::ListToplevelsJson => {
            let toplevels: Vec<ToplevelInfo> = connection
                .get_all_toplevels()
                .iter()
                .filter(|t| t.active)
                .map(|tl| ToplevelInfo {
                    title: tl.title.clone(),
                    app_id: tl.app_id.clone(),
                    identifier: tl.identifier.clone(),
                })
                .collect();
            writeln!(writer, "{}", serde_json::to_string_pretty(&toplevels)?)?;
            writer.flush()?;
            Ok(())
        }
        #[cfg(feature = "color_picker")]
        Command::ColorPicker(format) => color_picker::pick(&connection, settings.freeze, format),
        Command::Screenshot(mode) => {
            if let Some(ms) = settings.delay {
                std::thread::sleep(Duration::from_millis(ms as u64));
            }
            let result = screenshot::capture(&connection, &mode, settings.cursor, settings.freeze);
            match result {
                Ok((image_buffer, shot_result)) => {
                    let encoded = utils::encode_image(
                        &image_buffer,
                        settings.encoding,
                        &settings.jxl,
                        &settings.png,
                    )
                    .map_err(|e| eyre::eyre!("Failed to encode image: {e}"))?;

                    if let Some(ref f) = settings.file {
                        std::fs::write(f, &encoded)?;
                    }

                    if settings.stdout_print || !stdout.is_terminal() {
                        writer.write_all(&encoded)?;
                    }

                    #[cfg(feature = "clipboard")]
                    if settings.clipboard {
                        clipboard::copy_to_clipboard(encoded, settings.encoding)?;
                    }

                    #[cfg(feature = "notifications")]
                    if settings.notifications {
                        notification::send_success(
                            &shot_result,
                            settings.file.as_deref(),
                            &settings.notification,
                        );
                    }
                    // Silence unused warning when the notifications feature is disabled.
                    #[cfg(not(feature = "notifications"))]
                    drop(shot_result);

                    Ok(())
                }
                Err(e) => {
                    #[cfg(feature = "notifications")]
                    if settings.notifications {
                        notification::send_failure(&e, &settings.notification);
                    }
                    Err(e)
                }
            }
        }
    }
}
