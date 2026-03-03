use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use eyre::{Result, bail};
use libwayshot::WayshotConnection;
#[cfg(feature = "selector")]
use libwaysip::WaySip;

use crate::cli::Cli;
#[cfg(feature = "selector")]
use crate::utils::waysip_to_region;

/// Describes what was captured, used to build the notification body.
#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "notifications"), allow(dead_code))]
pub enum ShotResult {
    Output { name: String },
    Toplevel { name: String },
    Area,
    All,
}

/// How the screenshot target is determined.
pub enum CaptureMode {
    /// Interactive area/region selection via waysip.
    #[cfg(feature = "selector")]
    Geometry,
    /// A specific toplevel window by its id+title string.
    Toplevel(String),
    /// Interactive fuzzy-select from active toplevel windows.
    ChooseToplevel,
    /// A named output/display.
    Output(String),
    /// Interactive fuzzy-select from available outputs.
    ChooseOutput,
    /// Every connected output at once.
    All,
}

impl CaptureMode {
    /// Derive the capture mode from parsed CLI flags.
    pub fn from_cli(cli: &Cli) -> Self {
        #[cfg(feature = "selector")]
        if cli.geometry {
            return Self::Geometry;
        }
        if let Some(ref name) = cli.toplevel {
            Self::Toplevel(name.clone())
        } else if cli.choose_toplevel {
            Self::ChooseToplevel
        } else if let Some(ref name) = cli.output {
            Self::Output(name.clone())
        } else if cli.choose_output {
            Self::ChooseOutput
        } else {
            Self::All
        }
    }
}

/// Capture a screenshot according to `mode`.
pub fn capture(
    conn: &WayshotConnection,
    mode: &CaptureMode,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    match mode {
        #[cfg(feature = "selector")]
        CaptureMode::Geometry => capture_geometry(conn, cursor),
        CaptureMode::Toplevel(name) => capture_toplevel_by_name(conn, name, cursor),
        CaptureMode::ChooseToplevel => capture_toplevel_interactive(conn, cursor),
        CaptureMode::Output(name) => capture_output_by_name(conn, name, cursor),
        CaptureMode::ChooseOutput => capture_output_interactive(conn, cursor),
        CaptureMode::All => Ok((conn.screenshot_all(cursor)?, ShotResult::All)),
    }
}

/// Capture an interactively selected screen region.
#[cfg(feature = "selector")]
fn capture_geometry(
    conn: &WayshotConnection,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    let image = conn.screenshot_freeze(
        |w_conn| {
            let info = WaySip::new()
                .with_connection(w_conn.conn.clone())
                .with_selection_type(libwaysip::SelectionType::Area)
                .get()
                .map_err(|e| libwayshot::Error::FreezeCallbackError(e.to_string()))?
                .ok_or_else(|| {
                    libwayshot::Error::FreezeCallbackError("No area selected".to_string())
                })?;
            waysip_to_region(info.size(), info.left_top_point())
        },
        cursor,
    )?;
    Ok((image, ShotResult::Area))
}

fn capture_toplevel_by_name(
    conn: &WayshotConnection,
    name: &str,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    let toplevels = conn.get_all_toplevels();
    let toplevel = toplevels
        .iter()
        .filter(|t| t.active)
        .find(|t| t.id_and_title() == name)
        .ok_or_else(|| eyre::eyre!("No toplevel window matched '{name}'"))?;
    Ok((
        conn.screenshot_toplevel(toplevel, cursor)?,
        ShotResult::Toplevel {
            name: name.to_string(),
        },
    ))
}

fn capture_toplevel_interactive(
    conn: &WayshotConnection,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    let toplevels = conn.get_all_toplevels();
    let active: Vec<_> = toplevels.iter().filter(|t| t.active).collect();
    if active.is_empty() {
        bail!("No active toplevel windows found!");
    }
    let names: Vec<String> = active.iter().map(|t| t.id_and_title()).collect();
    let idx = fuzzy_select(&names).ok_or_else(|| eyre::eyre!("No toplevel window selected!"))?;
    Ok((
        conn.screenshot_toplevel(active[idx], cursor)?,
        ShotResult::Toplevel {
            name: names[idx].clone(),
        },
    ))
}

fn capture_output_by_name(
    conn: &WayshotConnection,
    name: &str,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    let outputs = conn.get_all_outputs();
    let output = outputs
        .iter()
        .find(|o| o.name == name)
        .ok_or_else(|| eyre::eyre!("No output named '{name}' found"))?;
    Ok((
        conn.screenshot_single_output(output, cursor)?,
        ShotResult::Output {
            name: name.to_string(),
        },
    ))
}

fn capture_output_interactive(
    conn: &WayshotConnection,
    cursor: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    let outputs = conn.get_all_outputs();
    let names: Vec<&str> = outputs.iter().map(|o| o.name.as_str()).collect();
    let idx = fuzzy_select(&names).ok_or_else(|| eyre::eyre!("No output selected!"))?;
    Ok((
        conn.screenshot_single_output(&outputs[idx], cursor)?,
        ShotResult::Output {
            name: names[idx].to_string(),
        },
    ))
}

fn fuzzy_select<T: ToString + std::fmt::Display>(items: &[T]) -> Option<usize> {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose Screen")
        .default(0)
        .items(items)
        .interact()
        .ok()
}
