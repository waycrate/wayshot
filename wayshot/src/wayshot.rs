use clap::Parser;
use config::Config;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use eyre::{bail, Result};
use libwayshot::{region::LogicalRegion, WayshotConnection};
use nix::unistd::{fork, ForkResult};
use std::{
    env,
    io::{self, BufWriter, Cursor, Write},
    process::Command,
};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

mod cli;
mod config;
mod utils;

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
    // cli args
    let cli = cli::Cli::parse();

    // config path
    let config_path = cli.config.unwrap_or(Config::get_default_path());

    // config
    let config = Config::load(&config_path).unwrap_or_default();
    let base = config.base.unwrap_or_default();
    let file = config.file.unwrap_or_default();

    // pre-work vars definitions
    let log_level = cli.log_level.unwrap_or(base.get_log_level());
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(io::stderr)
        .init();

    let cursor = cli.cursor.unwrap_or(base.cursor.unwrap_or_default());
    let clipboard = cli.clipboard.unwrap_or(base.clipboard.unwrap_or_default());
    let filename_format = cli.filename_format.unwrap_or(
        file.format
            .unwrap_or("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
    );

    let input_encoding = cli
        .file
        .as_ref()
        .and_then(|pathbuf| pathbuf.try_into().ok());
    let requested_encoding = cli.encoding.or(input_encoding);
    let encoding = match requested_encoding {
        Some(e) => {
            if let Some(input_encoding) = input_encoding {
                if input_encoding.ne(&e) {
                    tracing::warn!(
                        "The encoding requested '{e}' does not match the output file's encoding '{input_encoding}'. Still using the requested encoding however.",
                    );
                }

                input_encoding
            } else {
                e
            }
        }
        _ => file.encoding.unwrap_or_default(),
    };

    let stdout_print = cli.stdout.unwrap_or(base.stdout.unwrap_or_default());
    let file_path = match cli.file {
        Some(mut f) => {
            if f.is_dir() {
                Some(utils::get_full_file_name(&f, &filename_format, encoding))
            } else {
                f.set_extension("");
                let dir = file.path.unwrap_or(env::current_dir().unwrap_or_default());
                Some(utils::get_full_file_name(
                    &dir,
                    f.to_str().unwrap(),
                    encoding,
                ))
            }
        }
        _ => {
            if base.fs.unwrap_or_default() {
                let dir = file.path.unwrap_or(env::current_dir().unwrap_or_default());
                Some(utils::get_full_file_name(&dir, &filename_format, encoding))
            } else {
                None
            }
        }
    };

    let output = cli.output.or(base.output);

    let wayshot_conn = WayshotConnection::new()?;

    if cli.list_outputs {
        let valid_outputs = wayshot_conn.get_all_outputs();
        for output in valid_outputs {
            tracing::info!("{:#?}", output.name);
        }
        return Ok(());
    }

    // take a screenshot
    let image_buffer = if let Some(slurp_region) = cli.slurp {
        let slurp_region = slurp_region.clone();
        wayshot_conn.screenshot_freeze(
            Box::new(move || {
                || -> Result<LogicalRegion> {
                    let slurp_output = Command::new("slurp")
                        .args(slurp_region.split(' '))
                        .output()?
                        .stdout;

                    utils::parse_geometry(&String::from_utf8(slurp_output)?)
                }()
                .map_err(|_| libwayshot::Error::FreezeCallbackError)
            }),
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
        if let Some(index) = select_ouput(&output_names) {
            wayshot_conn.screenshot_single_output(&outputs[index], cursor)?
        } else {
            bail!("No output found!");
        }
    } else {
        wayshot_conn.screenshot_all(cursor)?
    };

    // save the screenshot data
    let mut image_buf: Option<Cursor<Vec<u8>>> = None;
    if let Some(file_path) = file_path {
        image_buffer.save(file_path)?;
    }

    if stdout_print {
        let mut buffer = Cursor::new(Vec::new());
        image_buffer.write_to(&mut buffer, encoding)?;
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write_all(buffer.get_ref())?;
        image_buf = Some(buffer);
    }

    if clipboard {
        clipboard_daemonize(match image_buf {
            Some(buf) => buf,
            None => {
                let mut buffer = Cursor::new(Vec::new());
                image_buffer.write_to(&mut buffer, encoding)?;
                buffer
            }
        })?;
    }

    Ok(())
}

/// Daemonize and copy the given buffer containing the encoded image to the clipboard
fn clipboard_daemonize(buffer: Cursor<Vec<u8>>) -> Result<()> {
    let mut opts = Options::new();
    match unsafe { fork() } {
        // Having the image persistently available on the clipboard requires a wayshot process to be alive.
        // Fork the process with a child detached from the main process and have the parent exit
        Ok(ForkResult::Parent { .. }) => {
            return Ok(());
        }
        Ok(ForkResult::Child) => {
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
