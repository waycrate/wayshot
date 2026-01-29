use config::Config;
use std::{
    env,
    io::{self, BufWriter, Cursor, Write},
};

use clap::Parser;
use eyre::{Result, bail};
use libwayshot::WayshotConnection;

mod cli;
mod config;
mod utils;

use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use libwaysip::WaySip;
use utils::{ShotResult, send_notification, waysip_to_region};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

use crate::utils::EncodingFormat;
use rustix::runtime::{self, Fork};

fn select_output<T>(outputs: &[T]) -> Option<usize>
where
    T: ToString + std::fmt::Display,
{
    let Ok(selection) = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose Screen")
        .default(0)
        .items(outputs)
        .interact()
    else {
        return None;
    };
    Some(selection)
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let config_path = cli.config.unwrap_or(Config::get_default_path());
    let config = Config::load(&config_path).unwrap_or_default();
    let base = config.base.unwrap_or_default();
    let file = config.file.unwrap_or_default();
    let encoding_config = config.encoding.unwrap_or_default();
    let jxl_config = encoding_config.jxl.unwrap_or_default();
    let png_config = encoding_config.png.unwrap_or_default();
    let notifications_enabled = base.notifications.unwrap_or(true);

    let log_level = cli.log_level.unwrap_or(base.get_log_level());
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(io::stderr)
        .init();

    let cursor = match cli.cursor {
        true => cli.cursor,
        _ => base.cursor.unwrap_or_default(),
    };
    let clipboard = match cli.clipboard {
        true => cli.clipboard,
        _ => base.clipboard.unwrap_or_default(),
    };

    let input_encoding = cli
        .file
        .as_ref()
        .and_then(|pathbuf| pathbuf.try_into().ok());
    let encoding = cli
        .encoding
        .or(input_encoding)
        .unwrap_or(file.encoding.unwrap_or_default());

    if let Some(ie) = input_encoding
        && ie != encoding
    {
        tracing::warn!(
            "The encoding requested '{encoding}' does not match the output file's encoding '{ie}'. Still using the requested encoding however.",
        );
    }

    let file_name_format = cli.file_name_format.unwrap_or(
        file.name_format
            .unwrap_or("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
    );
    let mut stdout_print = base.stdout.unwrap_or_default();
    let file = cli
        .file
        .and_then(|pathbuf| {
            if pathbuf.to_string_lossy() == "-" {
                stdout_print = true;
                None
            } else {
                Some(utils::get_full_file_name(
                    &pathbuf,
                    &file_name_format,
                    encoding,
                ))
            }
        })
        .or_else(|| {
            if base.file.unwrap_or_default() {
                let dir = file
                    .path
                    .unwrap_or_else(|| env::current_dir().unwrap_or_default());
                Some(utils::get_full_file_name(&dir, &file_name_format, encoding))
            } else {
                None
            }
        });

    let output = cli.output.or(base.output);

    let wayshot_conn = WayshotConnection::new()?;

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    if cli.list_outputs {
        let valid_outputs = wayshot_conn.get_all_outputs();
        for output in valid_outputs {
            writeln!(writer, "{}", output.name)?;
        }

        writer.flush()?;

        return Ok(());
    }

    if cli.list_outputs_info {
        wayshot_conn.print_displays_info();
        return Ok(());
    }

    if cli.list_toplevels {
        let toplevels = wayshot_conn.get_all_toplevels();
        for tl in toplevels.iter().filter(|t| t.active) {
            writeln!(writer, "{}", tl.id_and_title())?;
        }
        writer.flush()?;
        return Ok(());
    }
    if cli.color {
        let image = wayshot_conn
            .screenshot_freeze(
                |w_conn| {
                    let info = WaySip::new()
                        .with_connection(w_conn.conn.clone())
                        .with_selection_type(libwaysip::SelectionType::Point)
                        .get()
                        .map_err(|e| libwayshot::Error::FreezeCallbackError(e.to_string()))?
                        .ok_or(libwayshot::Error::FreezeCallbackError(
                            "Failed to capture the area".to_string(),
                        ))?;
                    waysip_to_region(
                        libwaysip::Size {
                            width: 1,
                            height: 1,
                        },
                        info.left_top_point(),
                    )
                },
                false,
            )?
            .to_rgba8();
        let pixel = image.get_pixel(0, 0);
        let [r, g, b, a] = pixel.0;
        let r_f = r as f32 / 255.;
        let g_f = g as f32 / 255.;
        let b_f = b as f32 / 255.;
        let a_f = a as f32 / 255.;
        println!("RGBA       : R:{r}, G:{g}, B{b}, A{a}");
        println!("RGBA(float): R:{r_f:.2}, G:{g_f:.2}, B:{b_f:.2}, A:{a_f:.2}");
        println!("16hex      : #{:02x}{:02x}{:02x}{:02x}", r, g, b, a);
        return Ok(());
    }

    let result = (|| -> Result<(image::DynamicImage, ShotResult)> {
        if cli.geometry {
            Ok((
                wayshot_conn.screenshot_freeze(
                    |w_conn| {
                        let info = WaySip::new()
                            .with_connection(w_conn.conn.clone())
                            .with_selection_type(libwaysip::SelectionType::Area)
                            .get()
                            .map_err(|e| libwayshot::Error::FreezeCallbackError(e.to_string()))?
                            .ok_or(libwayshot::Error::FreezeCallbackError(
                                "Failed to capture the area".to_string(),
                            ))?;
                        waysip_to_region(info.size(), info.left_top_point())
                    },
                    cursor,
                )?,
                ShotResult::Area,
            ))
        } else if let Some(ref name) = cli.toplevel {
            let toplevels = wayshot_conn.get_all_toplevels();
            let maybe = toplevels
                .iter()
                .filter(|t| t.active)
                .find(|t| t.id_and_title() == *name);
            if let Some(toplevel) = maybe {
                Ok((
                    wayshot_conn.screenshot_toplevel(toplevel, cursor)?,
                    ShotResult::Toplevel { name: name.clone() },
                ))
            } else {
                bail!("No toplevel window matched '{name}'")
            }
        } else if cli.choose_toplevel {
            let toplevels = wayshot_conn.get_all_toplevels();
            let active: Vec<_> = toplevels.iter().filter(|t| t.active).collect();
            if active.is_empty() {
                bail!("No active toplevel windows found!");
            }
            let names: Vec<String> = active.iter().map(|t| t.id_and_title()).collect();
            if let Some(idx) = select_output(&names) {
                Ok((
                    wayshot_conn.screenshot_toplevel(active[idx], cursor)?,
                    ShotResult::Toplevel {
                        name: names[idx].clone(),
                    },
                ))
            } else {
                bail!("No toplevel window selected!");
            }
        } else if let Some(output_name) = output {
            let outputs = wayshot_conn.get_all_outputs();
            if let Some(output) = outputs.iter().find(|output| output.name == output_name) {
                Ok((
                    wayshot_conn.screenshot_single_output(output, cursor)?,
                    ShotResult::Output {
                        name: output_name.clone(),
                    },
                ))
            } else {
                bail!("No output found!");
            }
        } else if cli.choose_output {
            let outputs = wayshot_conn.get_all_outputs();
            let output_names: Vec<&str> = outputs
                .iter()
                .map(|display| display.name.as_str())
                .collect();
            if let Some(index) = select_output(&output_names) {
                Ok((
                    wayshot_conn.screenshot_single_output(&outputs[index], cursor)?,
                    ShotResult::Output {
                        name: output_names[index].to_string(),
                    },
                ))
            } else {
                bail!("No output found!");
            }
        } else {
            Ok((wayshot_conn.screenshot_all(cursor)?, ShotResult::All))
        }
    })();

    match result {
        Ok((image_buffer, shot_result)) => {
            let mut image_buf: Option<Cursor<Vec<u8>>> = None;

            if let Some(f) = file {
                if encoding == EncodingFormat::Jxl {
                    if let Err(e) = utils::encode_to_jxl(
                        &image_buffer,
                        &f,
                        jxl_config.get_lossless(),
                        jxl_config.get_distance(),
                        jxl_config.get_encoder_speed(),
                    ) {
                        tracing::error!("Failed to encode to JXL: {}", e);
                    }
                } else if encoding == EncodingFormat::Png {
                    if cli.optimize {
                        if let Err(e) = utils::encode_to_png_optimized(
                            &image_buffer,
                            &f,
                            png_config.get_compression(),
                            png_config.get_filter(),
                        ) {
                            tracing::error!("Failed to encode to PNG: {}", e);
                        }
                    } else if let Err(e) = utils::encode_to_png(
                        &image_buffer,
                        &f,
                        png_config.get_compression(),
                        png_config.get_filter(),
                    ) {
                        tracing::error!("Failed to encode to PNG: {}", e);
                    }
                } else {
                    image_buffer.save(f)?;
                }
            }

            if stdout_print {
                let buffer = if encoding == EncodingFormat::Jxl {
                    let data = utils::encode_to_jxl_bytes(
                        &image_buffer,
                        jxl_config.get_lossless(),
                        jxl_config.get_distance(),
                        jxl_config.get_encoder_speed(),
                    )
                    .map_err(|e| eyre::eyre!("Failed to encode JXL: {}", e))?;
                    Cursor::new(data)
                } else if encoding == EncodingFormat::Png {
                    let data = if cli.optimize {
                        utils::encode_to_png_bytes_optimized(
                            &image_buffer,
                            png_config.get_compression(),
                            png_config.get_filter(),
                        )
                        .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?
                    } else {
                        utils::encode_to_png_bytes(
                            &image_buffer,
                            png_config.get_compression(),
                            png_config.get_filter(),
                        )
                        .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?
                    };
                    Cursor::new(data)
                } else {
                    let mut buffer = Cursor::new(Vec::new());
                    image_buffer.write_to(&mut buffer, encoding.into())?;
                    buffer
                };
                writer.write_all(buffer.get_ref())?;
                image_buf = Some(buffer);
            }

            if clipboard {
                clipboard_daemonize(match image_buf {
                    Some(buf) => buf,
                    None => {
                        if encoding == EncodingFormat::Jxl {
                            let data = utils::encode_to_jxl_bytes(
                                &image_buffer,
                                jxl_config.get_lossless(),
                                jxl_config.get_distance(),
                                jxl_config.get_encoder_speed(),
                            )
                            .map_err(|e| eyre::eyre!("Failed to encode JXL: {}", e))?;
                            Cursor::new(data)
                        } else if encoding == EncodingFormat::Png {
                            let data = if cli.optimize {
                                utils::encode_to_png_bytes_optimized(
                                    &image_buffer,
                                    png_config.get_compression(),
                                    png_config.get_filter(),
                                )
                                .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?
                            } else {
                                utils::encode_to_png_bytes(
                                    &image_buffer,
                                    png_config.get_compression(),
                                    png_config.get_filter(),
                                )
                                .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?
                            };
                            Cursor::new(data)
                        } else {
                            let mut buffer = Cursor::new(Vec::new());
                            image_buffer.write_to(&mut buffer, encoding.into())?;
                            buffer
                        }
                    }
                })?;
            }

            if notifications_enabled {
                send_notification(Ok(shot_result));
            }

            Ok(())
        }
        Err(e) => {
            if notifications_enabled {
                send_notification(Err(&e));
            }
            Err(e)
        }
    }
}

/// Daemonize and copy the given buffer containing the encoded image to the clipboard
fn clipboard_daemonize(buffer: Cursor<Vec<u8>>) -> Result<()> {
    let mut opts = Options::new();
    match unsafe { runtime::kernel_fork() } {
        // Having the image persistently available on the clipboard requires a wayshot process to be alive.
        // Fork the process with a child detached from the main process and have the parent exit
        Ok(Fork::ParentOf(_)) => {
            return Ok(());
        }
        Ok(Fork::Child(_)) => {
            opts.foreground(true); // Offer the image till something else is available on the clipboard
            opts.copy(
                Source::Bytes(buffer.into_inner().into()),
                MimeType::Autodetect,
            )?;
        }
        Err(e) => {
            tracing::warn!(
                "Fork failed with error: {e}, couldn't offer image on the clipboard persistently.
                 Use a clipboard manager to record screenshot."
            );
            opts.copy(
                Source::Bytes(buffer.into_inner().into()),
                MimeType::Autodetect,
            )?;
        }
    }
    Ok(())
}
