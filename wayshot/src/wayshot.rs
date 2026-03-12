use std::io::{self, BufWriter, Write};

use clap::Parser;
use eyre::Result;
use libwayshot::WayshotConnection;

mod cli;
#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "color_picker")]
mod color_picker;
mod config;
#[cfg(feature = "logger")]
mod logger;
#[cfg(feature = "notifications")]
mod notification;
mod screenshot;
mod settings;
mod utils;

use config::Config;
use settings::{AppSettings, Command};

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
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
            connection.print_displays_info();
            Ok(())
        }
        Command::ListToplevels => {
            for tl in connection.get_all_toplevels().iter().filter(|t| t.active) {
                writeln!(writer, "{}", tl.id_and_title())?;
            }
            writer.flush()?;
            Ok(())
        }
        #[cfg(feature = "color_picker")]
        Command::ColorPicker => color_picker::pick(&connection, settings.freeze),
        Command::Screenshot(mode) => {
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

                    if settings.stdout_print {
                        writer.write_all(&encoded)?;
                    }

                    #[cfg(feature = "clipboard")]
                    if settings.clipboard {
                        clipboard::copy_to_clipboard(encoded)?;
                    }

                    #[cfg(feature = "notifications")]
                    if settings.notifications {
                        notification::send_success(&shot_result);
                    }
                    // Silence unused warning when the notifications feature is disabled.
                    #[cfg(not(feature = "notifications"))]
                    drop(shot_result);

                    Ok(())
                }
                Err(e) => {
                    #[cfg(feature = "notifications")]
                    if settings.notifications {
                        notification::send_failure(&e);
                    }
                    Err(e)
                }
            }
        }
    }
}
