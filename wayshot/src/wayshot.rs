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
use utils::waysip_to_region;

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

    if cli.set_default_path.is_some() || cli.set_cursor.is_some() || cli.set_clipboard.is_some() {

        utils::update_config(
            &config_path,
            cli.set_default_path,
            cli.set_cursor,
            cli.set_clipboard,
        )?;
        return Ok(());
    }

    
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

    if let Some(ie) = input_encoding {
        if ie != encoding {
            tracing::warn!(
                "The encoding requested '{encoding}' does not match the output file's encoding '{ie}'. Still using the requested encoding however.",
            );
        }
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

    let image_buffer = if cli.geometry {
        wayshot_conn.screenshot_freeze(
            |w_conn| {
                let info = libwaysip::get_area(
                    Some(libwaysip::WaysipConnection {
                        connection: &w_conn.conn,
                        globals: &w_conn.globals,
                    }),
                    libwaysip::SelectionType::Area,
                )
                .map_err(|e| libwayshot::Error::FreezeCallbackError(e.to_string()))?
                .ok_or(libwayshot::Error::FreezeCallbackError(
                    "Failed to capture the area".to_string(),
                ))?;
                waysip_to_region(info.size(), info.left_top_point())
            },
            cursor,
        )?
    } else if let Some(output_name) = output {
        let outputs = wayshot_conn.get_all_outputs();
        if let Some(output) = outputs.iter().find(|output| output.name == output_name) {
            wayshot_conn.screenshot_single_output(output, cursor)?
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
            wayshot_conn.screenshot_single_output(&outputs[index], cursor)?
        } else {
            bail!("No output found!");
        }
    } else {
        wayshot_conn.screenshot_all(cursor)?
    };

    let mut image_buf: Option<Cursor<Vec<u8>>> = None;
    if let Some(f) = file {
        image_buffer.save(f)?;
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
