use config::Config;
use std::{
    env,
    io::{self, BufWriter, Cursor, Write},
};

use clap::Parser;
use eyre::{Result, bail};

mod cli;
mod config;
mod ext_wayshot;
mod utils;

use ext_wayshot::*;

use dialoguer::{FuzzySelect, theme::ColorfulTheme};

use wl_clipboard_rs::copy::{MimeType, Options, Source};

use rustix::runtime::{self, Fork};

fn select_output<T>(outputs: &[T]) -> Option<usize>
where
    T: ToString,
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

    // Create WayshotConnection (will automatically use ext_image protocol if available)
    let connection_result = libwayshot::WayshotConnection::new();

    match connection_result {
        Ok(mut state) => {
            // If we have a connection, check if it has ext_image capability
            let has_ext_image = state.ext_image.is_some();

            if has_ext_image {
                tracing::info!("Using ext_image protocol");

                let stdout = io::stdout();
                let mut writer = BufWriter::new(stdout.lock());

                if cli.list_outputs {
                    let valid_outputs = state.get_all_outputs();
                    for output in valid_outputs {
                        writeln!(writer, "{}", output.name)?;
                    }
                    writer.flush()?;
                    return Ok(());
                }

                if cli.list_outputs_info {
                    state.print_displays_info();
                    return Ok(());
                }

                // EXT protocol logic for -g, -t, -o, --color
                let image_result = if cli.color {
                    // ext_capture_color does not return a DynamicImage, so handle separately
                    match ext_capture_color(&mut state) {
                        Ok(res) => {
                            notify_result(Ok(res));
                            return Ok(());
                        }
                        Err(e) => {
                            tracing::error!("Failed to capture color: {}", e);
                            notify_result(Err(e));
                            return Ok(());
                        }
                    }
                } else if cli.geometry {
                    ext_capture_area(&mut state, stdout_print, cursor)
                } else if cli.toplevel {
                    ext_capture_toplevel(&mut state, stdout_print, cursor)
                        .map(|(img, name)| (img, WayshotResult::ToplevelCaptured { name }))
                } else if output.as_ref().is_some() || cli.choose_output {
                    ext_capture_output(&mut state, output.clone(), stdout_print, cursor)
                        .map(|(img, name)| (img, WayshotResult::OutputCaptured { name }))
                } else {
                    // If no flag is provided, default to output selection (choose_output = true)
                    ext_capture_output(&mut state, None, stdout_print, cursor)
                        .map(|(img, name)| (img, WayshotResult::OutputCaptured { name }))
                };

                match image_result {
                    Ok((image_buffer_opt, result_variant)) => {
                        // If image_buffer_opt is None, it means stdout was used and we're done
                        if let Some(image_buffer) = image_buffer_opt {
                            if let Some(f) = file.as_ref() {
                                if let Err(e) = image_buffer.save(f) {
                                    tracing::error!("Failed to save file '{}': {}", f.display(), e);
                                    notify_result(Err(
                                        ext_wayshot::WayshotImageWriteError::ImageError(e),
                                    ));
                                } else {
                                    notify_result(Ok(result_variant.clone()));
                                }
                            } else {
                                notify_result(Ok(result_variant));
                            }

                            // This again depends on the Compositor present,
                            // Compositors such as Cosmic doesn't have Ext/wlr data parsing protocol present
                            // so Clipboard doesn't work yet for Cosmic or any such Compositors.
                            // However Stdout shouldn't be affected in any manner 

                            if clipboard {
                                let mut buffer = Cursor::new(Vec::new());
                                image_buffer.write_to(&mut buffer, encoding.into())?;
                                clipboard_daemonize(buffer)?;
                            }
                        } else {
                            // Image was written to stdout, only handle clipboard if needed
                            if clipboard {
                                tracing::warn!(
                                    "Clipboard functionality not available when using stdout output"
                                );
                            }
                            // Only notify if not using stdout
                            if !stdout_print {
                                notify_result(Ok(result_variant));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to capture output: {}", e);
                        notify_result(Err(e));
                    }
                }

                return Ok(());
            } else {
                tracing::info!("ext_image protocol not available, using wlr_screencopy");

                let stdout = io::stdout();
                let mut writer = BufWriter::new(stdout.lock());

                if cli.list_outputs {
                    let valid_outputs = state.get_all_outputs();
                    for output in valid_outputs {
                        writeln!(writer, "{}", output.name)?;
                    }
                    writer.flush()?;
                    return Ok(());
                }

                if cli.list_outputs_info {
                    state.print_displays_info();
                    return Ok(());
                }

                let image_buffer = if cli.geometry {
                    state.screenshot_freeze(
                        |w_conn| {
                            let info = libwaysip::get_area(
                                Some(libwaysip::WaysipConnection {
                                    connection: &w_conn.conn,
                                    globals: &w_conn.globals,
                                }),
                                libwaysip::SelectionType::Area,
                            )
                            .map_err(|e| {
                                libwayshot::WayshotError::FreezeCallbackError(e.to_string())
                            })?
                            .ok_or(
                                libwayshot::WayshotError::FreezeCallbackError(
                                    "Failed to capture the area".to_string(),
                                ),
                            )?;
                            utils::waysip_to_region(info.size(), info.left_top_point())
                        },
                        cursor,
                    )?
                } else if let Some(output_name) = output {
                    let outputs = state.get_all_outputs();
                    if let Some(output) = outputs.iter().find(|output| output.name == output_name) {
                        state.screenshot_single_output(output, cursor)?
                    } else {
                        bail!("No output found!");
                    }
                } else if cli.choose_output {
                    let outputs = state.get_all_outputs();
                    let output_names: Vec<&str> = outputs
                        .iter()
                        .map(|display| display.name.as_str())
                        .collect();
                    if let Some(index) = select_output(&output_names) {
                        state.screenshot_single_output(&outputs[index], cursor)?
                    } else {
                        bail!("No output found!");
                    }
                } else {
                    state.screenshot_all(cursor)?
                };

                let mut image_buf: Option<Cursor<Vec<u8>>> = None;
                if let Some(f) = file
                    && let Err(e) = image_buffer.save(&f)
                {
                    tracing::error!("Failed to save file '{}': {}", f.display(), e);
                    // TODO: Optionally, notify the user or handle the error as needed
                }

                if stdout_print {
                    let mut buffer = Cursor::new(Vec::new());
                    image_buffer.write_to(&mut buffer, encoding.into())?;
                    writer.write_all(buffer.get_ref())?;
                    image_buf = Some(buffer);
                }

                if clipboard {
                    clipboard_daemonize(match image_buf {
                        Some(buf) => buf,
                        None => {
                            let mut buffer = Cursor::new(Vec::new());
                            image_buffer.write_to(&mut buffer, encoding.into())?;
                            buffer
                        }
                    })?;
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create WayshotConnection: {}", e);
            bail!("Could not establish connection to Wayland compositor");
        }
    }

    Ok(())
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
