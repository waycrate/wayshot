use std::{
    io::{stdout, BufWriter, Cursor, Write},
    process::Command,
};

use clap::Parser;
use eyre::{bail, Result};
use libwayshot::{region::LogicalRegion, WayshotConnection};

mod cli;
mod utils;

use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use utils::EncodingFormat;

use wl_clipboard_rs::copy::{MimeType, Options, Source};

use rustix::runtime::{fork, Fork};

use clap_complete::{generate,Shell}; // <-- Add this import
use clap::CommandFactory;    // <-- Add this import

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
    let cli = cli::Cli::parse();

    // Handle --generate-completions
    if let Some(shell) = cli.generate_completions {
        let mut cmd = cli::Cli::command();
        eprintln!("Generating completions for {:?}...", shell);

        // Get the user's shell from the environment
        let user_shell = std::env::var("SHELL")
            .unwrap_or_else(|_| "bash".to_string()) // Default to bash if SHELL is not set
            .to_lowercase();

        // Map the shell to the appropriate completion file path
        let completion_path = match user_shell {
            s if s.contains("bash") => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.bash_completion.d/wayshot", home)
            }
            s if s.contains("zsh") => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.zsh/completions/_wayshot", home)
            }
            s if s.contains("fish") => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.config/fish/completions/wayshot.fish", home)
            }
            _ => {
                eprintln!("Unsupported shell: {}", user_shell);
                return Ok(());
            }
        };

        // Create the directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&completion_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write the completions to the file
        let mut file = std::fs::File::create(&completion_path)?;
        generate(shell, &mut cmd, "wayshot", &mut file);
        eprintln!("Completions installed to: {}", completion_path);

        return Ok(());
    }


    tracing_subscriber::fmt()
        .with_max_level(cli.log_level)
        .with_writer(std::io::stderr)
        .init();

    let input_encoding = cli
        .file
        .as_ref()
        .and_then(|pathbuf| pathbuf.try_into().ok());
    let requested_encoding = cli
        .encoding
        .or(input_encoding)
        .unwrap_or(EncodingFormat::default());

    if let Some(input_encoding) = input_encoding {
        if input_encoding != requested_encoding {
            tracing::warn!(
                "The encoding requested '{requested_encoding}' does not match the output file's encoding '{input_encoding}'. Still using the requested encoding however.",
            );
        }
    }

    let wayshot_conn = WayshotConnection::new()?;

    if cli.list_outputs {
        let valid_outputs = wayshot_conn.get_all_outputs();
        for output in valid_outputs {
            tracing::info!("{:#?}", output.name);
        }
        return Ok(());
    }

    let image_buffer = if let Some(slurp_args) = cli.slurp {
        let slurp_region = slurp_args.unwrap_or("".to_string());
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
            cli.cursor,
        )?
    } else if let Some(output_name) = cli.output {
        let outputs = wayshot_conn.get_all_outputs();
        if let Some(output) = outputs.iter().find(|output| output.name == output_name) {
            wayshot_conn.screenshot_single_output(output, cli.cursor)?
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
            wayshot_conn.screenshot_single_output(&outputs[index], cli.cursor)?
        } else {
            bail!("No output found!");
        }
    } else {
        wayshot_conn.screenshot_all(cli.cursor)?
    };

    let mut stdout_print = false;
    let file = match cli.file {
        Some(mut pathbuf) => {
            if pathbuf.to_string_lossy() == "-" {
                stdout_print = true;
                None
            } else {
                if pathbuf.is_dir() {
                    pathbuf.push(utils::get_default_file_name(requested_encoding));
                }
                Some(pathbuf)
            }
        }
        _none => {
            if cli.clipboard {
                None
            } else {
                Some(utils::get_default_file_name(requested_encoding))
            }
        }
    };

    let mut image_buf: Option<Cursor<Vec<u8>>> = None;
    if let Some(file) = file {
        image_buffer.save(file)?;
    } else if stdout_print {
        let mut buffer = Cursor::new(Vec::new());
        image_buffer.write_to(&mut buffer, requested_encoding)?;
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write_all(buffer.get_ref())?;
        image_buf = Some(buffer);
    }

    if cli.clipboard {
        clipboard_daemonize(match image_buf {
            Some(buf) => buf,
            None => {
                let mut buffer = Cursor::new(Vec::new());
                image_buffer.write_to(&mut buffer, requested_encoding)?;
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
        Ok(Fork::Parent(_)) => {
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
