use image::{DynamicImage, GenericImageView, ImageEncoder, ImageError};
use std::{env, fs, path::PathBuf};

use crate::utils::waysip_to_region;
use dialoguer::FuzzySelect;
use dialoguer::theme::ColorfulTheme;
use libwayshot::WayshotConnection;

const TMP: &str = "/tmp";

use libwayshot::ext_image_protocols::CaptureOption;
use libwayshot::region::{Position, Region, Size};

#[derive(Debug, Clone)]
pub enum WayshotResult {
    StdoutSucceeded,
    SaveToFile(PathBuf),
    ColorSucceeded,
}

pub const SUCCEED_IMAGE: &str = "haruhi_succeeded";
pub const FAILED_IMAGE: &str = "haruhi_failed";
pub const TIMEOUT: i32 = 10000;

#[derive(Debug, thiserror::Error)]
pub enum WayshotImageWriteError {
    #[error("Image Error")]
    ImageError(#[from] ImageError),
    #[error("file created failed")]
    FileCreatedFailed(#[from] std::io::Error),
    #[error("FuzzySelect Failed")]
    FuzzySelectFailed(#[from] dialoguer::Error),
    #[error("Output not exist")]
    OutputNotExist,
    #[error("Wayland shot error")]
    WaylandError(#[from] libwayshot::error::WayshotError),
}

pub fn notify_result(shot_result: Result<WayshotResult, WayshotImageWriteError>) {
    use notify_rust::Notification;
    match shot_result {
        Ok(WayshotResult::StdoutSucceeded) => {
            let _ = Notification::new()
                .summary("Screenshot Succeed")
                .body("Screenshot Succeed")
                .icon(SUCCEED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
        Ok(WayshotResult::SaveToFile(file)) => {
            let file_name = file.to_string_lossy().to_string();
            let _ = Notification::new()
                .summary("File Saved SUcceed")
                .body(format!("File Saved to {file:?}").as_str())
                .icon(&file_name)
                .timeout(TIMEOUT)
                .show();
        }
        Ok(WayshotResult::ColorSucceeded) => {}
        Err(e) => {
            let _ = Notification::new()
                .summary("File Saved Failed")
                .body(&e.to_string())
                .icon(FAILED_IMAGE)
                .timeout(TIMEOUT)
                .show();
        }
    }
}

trait ToCaptureOption {
	fn to_capture_option(self) -> CaptureOption;
}

impl ToCaptureOption for bool {
	fn to_capture_option(self) -> CaptureOption {
		if self {
			CaptureOption::PaintCursors
		} else {
			CaptureOption::None
		}
	}
}

pub fn ext_capture_toplevel(
	state: &mut WayshotConnection,
	use_stdout: bool,
	pointer: bool,
) -> Result<DynamicImage, WayshotImageWriteError> {
	let toplevels = state.toplevels();
	let names: Vec<String> = toplevels.iter().map(|info| info.id_and_title()).collect();

	let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
		.with_prompt("Choose Application")
		.default(0)
		.items(&names)
		.interact()?;

	let toplevel = toplevels[selection].clone();
	let img = state
		.ext_capture_toplevel2(pointer.to_capture_option(), toplevel)
		.map_err(WayshotImageWriteError::WaylandError)?;
	Ok(img)
}

pub fn ext_capture_output(
	state: &mut WayshotConnection,
	output: Option<String>,
	use_stdout: bool,
	pointer: bool,
) -> eyre::Result<image::DynamicImage, WayshotImageWriteError> {
	let outputs = state.vector_of_Outputs();
	let names: Vec<&str> = outputs.iter().map(|info| info.name()).collect();

	let selection = match output {
		Some(name) => names
			.iter()
			.position(|tname| *tname == name)
			.ok_or(WayshotImageWriteError::OutputNotExist)?,
		None => FuzzySelect::with_theme(&ColorfulTheme::default())
			.with_prompt("Choose Screen")
			.default(0)
			.items(&names)
			.interact()?,
	};

	let output = outputs[selection].clone();
	let img = state
		.ext_capture_single_output(pointer.to_capture_option(), output)
		.map_err(WayshotImageWriteError::WaylandError)?;
	Ok(img)
}

pub fn ext_capture_area(
	state: &mut WayshotConnection,
	use_stdout: bool,
	pointer: bool,
) -> Result<DynamicImage, WayshotImageWriteError> {
	let (data, img_width, img_height, _color_type, region) = state.ext_capture_area2(pointer.to_capture_option(), |w_conn: &WayshotConnection| {
		let info = libwaysip::get_area(
			Some(libwaysip::WaysipConnection {
				connection: &w_conn.conn,
				globals: &w_conn.globals,
			}),
			libwaysip::SelectionType::Area,
		)
			.map_err(|e| libwayshot::error::WayshotError::CaptureFailed(e.to_string()))?
			.ok_or(libwayshot::error::WayshotError::CaptureFailed(
				"Failed to capture the area".to_string(),
			))?;

		// Map the Result<LogicalRegion> directly to Result<Region>
		waysip_to_region(info.size(), info.left_top_point())
			.map(|logical_region| logical_region.inner)
	})?;

	let Region { position: Position { x, y }, size: Size { width, height } } = region;
	// Always use RGBA8, as ext_capture_area2 already does the conversion
	let buffer = image::ImageBuffer::from_vec(img_width, img_height, data)
		.ok_or(ImageError::Parameter(
			image::error::ParameterError::from_kind(
				image::error::ParameterErrorKind::DimensionMismatch,
			),
		))?;
	let full_img = DynamicImage::ImageRgba8(buffer);
	let cropped = full_img.crop_imm(x as u32, y as u32, width as u32, height as u32);
	Ok(cropped)
}

use image::codecs::png::PngEncoder;

pub fn ext_capture_color(
    state: &mut WayshotConnection,
) -> Result<WayshotResult, WayshotImageWriteError> {
    let (data, img_width, img_height, color_type, region) = state.ext_capture_area2(CaptureOption::None, |w_conn: &WayshotConnection| {
        let info = libwaysip::get_area(
            Some(libwaysip::WaysipConnection {
                connection: &w_conn.conn,
                globals: &w_conn.globals,
            }),
            libwaysip::SelectionType::Point,
        )
        .map_err(|e| libwayshot::error::WayshotError::CaptureFailed(e.to_string()))?
        .ok_or(libwayshot::error::WayshotError::CaptureFailed(
            "Failed to capture the area".to_string(),
        ))?;

        // Map the Result<LogicalRegion> directly to Result<Region>
        waysip_to_region(info.size(), info.left_top_point())
            .map(|logical_region| logical_region.inner)
    })?;

    let Region { position: Position { x, y }, size: Size { width, height } } = region;
    let mut buff = std::io::Cursor::new(Vec::new());
    PngEncoder::new(&mut buff).write_image(&data, img_width, img_height, color_type.into())?;
    let img = image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();

    let clipimage = img.view(x as u32, y as u32, width as u32, height as u32);
    let pixel = clipimage.get_pixel(0, 0);
    println!(
        "RGB: R:{}, G:{}, B:{}, A:{}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
    println!(
        "16hex: #{:02x}{:02x}{:02x}{:02x}",
        pixel.0[0], pixel.0[1], pixel.0[2], pixel[3]
    );
    Ok(WayshotResult::ColorSucceeded)
}
