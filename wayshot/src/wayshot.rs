use config::Config;
use std::{
    env,
    io::{self, BufWriter, Cursor, Write},
    path::PathBuf,
};

use clap::Parser;
use eyre::{Result, bail};
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
mod utils;

use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use libwaysip::WaySip;
use utils::{ShotResult, waysip_to_region};

use crate::utils::EncodingFormat;

// ─── Resolved settings ────────────────────────────────────────────────────────

/// Runtime settings derived by merging CLI flags with the config file.
///
/// Priority for every field: explicit CLI flag > config file value > built-in default.
/// For boolean flags a CLI `true` always wins; config provides the default.
struct AppSettings {
    /// Whether to render the cursor in the captured image.
    cursor: bool,
    /// Final encoding format, after resolving extension / flag / config precedence.
    encoding: EncodingFormat,
    /// Destination file path, or `None` to skip file output.
    file: Option<PathBuf>,
    /// Write image bytes to stdout.
    stdout_print: bool,
    /// JPEG-XL encoder settings (always present; used only when encoding is Jxl).
    jxl: config::Jxl,
    /// PNG encoder settings.
    png: config::Png,
    #[cfg(feature = "clipboard")]
    clipboard: bool,
    #[cfg(feature = "notifications")]
    notifications: bool,
}

impl AppSettings {
    fn resolve(cli: &cli::Cli, config: &Config) -> Self {
        let base = config.base.clone().unwrap_or_default();
        let file_config = config.file.clone().unwrap_or_default();
        let encoding_config = config.encoding.clone().unwrap_or_default();

        // ── Cursor ────────────────────────────────────────────────────────────
        let cursor = cli.cursor || base.cursor.unwrap_or_default();

        // ── Encoding ──────────────────────────────────────────────────────────
        // Resolution order:
        //   1. --encoding flag
        //   2. format inferred from the FILE extension
        //   3. config `[file] encoding`
        //   4. built-in default (PNG)
        let input_encoding: Option<EncodingFormat> =
            cli.file.as_ref().and_then(|p| p.try_into().ok());
        let encoding = cli
            .encoding
            .or(input_encoding)
            .unwrap_or_else(|| file_config.encoding.unwrap_or_default());

        if let Some(ie) = input_encoding
            && ie != encoding
        {
            tracing::warn!(
                "The encoding requested '{encoding}' does not match the output file's encoding '{ie}'. Still using the requested encoding however.",
            );
        }

        // ── File name format ──────────────────────────────────────────────────
        let file_name_format = cli.file_name_format.clone().unwrap_or_else(|| {
            file_config
                .name_format
                .clone()
                .unwrap_or_else(|| "wayshot-%Y_%m_%d-%H_%M_%S".to_string())
        });

        // ── Stdout / file path ────────────────────────────────────────────────
        // stdout_print starts from config `stdout = true`.
        // resolve_output_file may also flip it to true when FILE is `-`.
        let mut stdout_print = base.stdout.unwrap_or_default();
        let file = resolve_output_file(
            cli.file.clone(),
            &base,
            &file_config,
            &file_name_format,
            encoding,
            &mut stdout_print,
        );

        AppSettings {
            cursor,
            encoding,
            file,
            stdout_print,
            jxl: encoding_config.jxl.unwrap_or_default(),
            png: encoding_config.png.unwrap_or_default(),
            #[cfg(feature = "clipboard")]
            clipboard: cli.clipboard || base.clipboard.unwrap_or_default(),
            #[cfg(feature = "notifications")]
            notifications: !cli.silent && base.notifications.unwrap_or(true),
        }
    }
}

/// Determine where (if anywhere) to write the image file.
///
/// Sets `stdout_print` to `true` when the caller passed `-` as the file path.
fn resolve_output_file(
    cli_file: Option<PathBuf>,
    base: &config::Base,
    file_config: &config::File,
    file_name_format: &str,
    encoding: EncodingFormat,
    stdout_print: &mut bool,
) -> Option<PathBuf> {
    if let Some(path) = cli_file {
        if path.to_string_lossy() == "-" {
            *stdout_print = true;
            return None;
        }
        return Some(utils::get_full_file_name(&path, file_name_format, encoding));
    }
    if base.file.unwrap_or_default() {
        let dir = file_config
            .path
            .clone()
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        return Some(utils::get_full_file_name(&dir, file_name_format, encoding));
    }
    None
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let config_path = cli.config.clone().unwrap_or(Config::get_default_path());
    let config = Config::load(&config_path).unwrap_or_default();

    #[cfg(feature = "logger")]
    logger::setup(&cli, &config);

    let settings = AppSettings::resolve(&cli, &config);
    let output = cli
        .output
        .or_else(|| config.base.as_ref().and_then(|b| b.output.clone()));

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

    #[cfg(feature = "color_picker")]
    if cli.color {
        return color_picker::pick(&wayshot_conn);
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
                    settings.cursor,
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
                    wayshot_conn.screenshot_toplevel(toplevel, settings.cursor)?,
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
                    wayshot_conn.screenshot_toplevel(active[idx], settings.cursor)?,
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
                    wayshot_conn.screenshot_single_output(output, settings.cursor)?,
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
                    wayshot_conn.screenshot_single_output(&outputs[index], settings.cursor)?,
                    ShotResult::Output {
                        name: output_names[index].to_string(),
                    },
                ))
            } else {
                bail!("No output found!");
            }
        } else {
            Ok((
                wayshot_conn.screenshot_all(settings.cursor)?,
                ShotResult::All,
            ))
        }
    })();

    match result {
        Ok((image_buffer, shot_result)) => {
            #[cfg(feature = "clipboard")]
            let mut image_buf: Option<Cursor<Vec<u8>>> = None;

            if let Some(f) = settings.file {
                if settings.encoding == EncodingFormat::Jxl {
                    if let Err(e) = utils::encode_to_jxl(
                        &image_buffer,
                        &f,
                        settings.jxl.get_lossless(),
                        settings.jxl.get_distance(),
                        settings.jxl.get_encoder_speed(),
                    ) {
                        tracing::error!("Failed to encode to JXL: {}", e);
                    }
                } else if settings.encoding == EncodingFormat::Png {
                    if let Err(e) = utils::encode_to_png(
                        &image_buffer,
                        &f,
                        settings.png.get_compression(),
                        settings.png.get_filter(),
                    ) {
                        tracing::error!("Failed to encode to PNG: {}", e);
                    }
                } else {
                    image_buffer.save(f)?;
                }
            }

            if settings.stdout_print {
                let buffer = if settings.encoding == EncodingFormat::Jxl {
                    let data = utils::encode_to_jxl_bytes(
                        &image_buffer,
                        settings.jxl.get_lossless(),
                        settings.jxl.get_distance(),
                        settings.jxl.get_encoder_speed(),
                    )
                    .map_err(|e| eyre::eyre!("Failed to encode JXL: {}", e))?;
                    Cursor::new(data)
                } else if settings.encoding == EncodingFormat::Png {
                    let data = utils::encode_to_png_bytes(
                        &image_buffer,
                        settings.png.get_compression(),
                        settings.png.get_filter(),
                    )
                    .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?;
                    Cursor::new(data)
                } else {
                    let mut buffer = Cursor::new(Vec::new());
                    image_buffer.write_to(&mut buffer, settings.encoding.into())?;
                    buffer
                };
                writer.write_all(buffer.get_ref())?;
                #[cfg(feature = "clipboard")]
                {
                    image_buf = Some(buffer);
                }
            }

            #[cfg(feature = "clipboard")]
            if settings.clipboard {
                clipboard::copy_to_clipboard(match image_buf {
                    Some(buf) => buf.into_inner(),
                    None => {
                        if settings.encoding == EncodingFormat::Jxl {
                            utils::encode_to_jxl_bytes(
                                &image_buffer,
                                settings.jxl.get_lossless(),
                                settings.jxl.get_distance(),
                                settings.jxl.get_encoder_speed(),
                            )
                            .map_err(|e| eyre::eyre!("Failed to encode JXL: {}", e))?
                        } else if settings.encoding == EncodingFormat::Png {
                            utils::encode_to_png_bytes(
                                &image_buffer,
                                settings.png.get_compression(),
                                settings.png.get_filter(),
                            )
                            .map_err(|e| eyre::eyre!("Failed to encode PNG: {}", e))?
                        } else {
                            let mut buffer = Cursor::new(Vec::new());
                            image_buffer.write_to(&mut buffer, settings.encoding.into())?;
                            buffer.into_inner()
                        }
                    }
                })?;
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
