#[cfg(feature = "selector")]
use crate::utils::get_region_area;
use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use eyre::{Result, bail};
use libwayshot::WayshotConnection;

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
    /// Interactive area/region selection via libwaysip.
    #[cfg(feature = "selector")]
    Geometry {
        foreground_color: Option<String>,
        background_color: Option<String>,
    },
    /// A fixed region parsed from a slurp/waysip geometry string.
    GeometryRegion(libwayshot::LogicalRegion),
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

/// Capture a screenshot according to `mode`.
#[cfg_attr(not(feature = "selector"), allow(unused_variables))]
pub fn capture(
    conn: &WayshotConnection,
    mode: &CaptureMode,
    cursor: bool,
    freeze: bool,
) -> Result<(image::DynamicImage, ShotResult)> {
    match mode {
        #[cfg(feature = "selector")]
        CaptureMode::Geometry {
            foreground_color,
            background_color,
        } => capture_geometry(
            conn,
            cursor,
            freeze,
            foreground_color.clone(),
            background_color.clone(),
        ),
        CaptureMode::GeometryRegion(region) => {
            let image = conn.screenshot(*region, cursor)?;
            Ok((image, ShotResult::Area))
        }
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
    freeze: bool,
    foreground_color: Option<String>,
    background_color: Option<String>,
) -> Result<(image::DynamicImage, ShotResult)> {
    let image = if freeze {
        conn.screenshot_freeze(
            move |w_conn| {
                get_region_area(w_conn, foreground_color.clone(), background_color.clone())
                    .map_err(libwayshot::Error::FreezeCallbackError)
            },
            cursor,
        )?
    } else {
        let region = get_region_area(conn, foreground_color, background_color)
            .map_err(|e| eyre::eyre!("{e}"))?;
        conn.screenshot(region, cursor)?
    };
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
