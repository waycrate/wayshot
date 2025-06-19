use image::{GenericImageView, ImageEncoder, ImageError};
use std::{env, fs, path::PathBuf};

use dialoguer::FuzzySelect;
use dialoguer::theme::ColorfulTheme;
use libwayshot::ext_image_protocols::HaruhiShotState;

const TMP: &str = "/tmp";

use libwayshot::ext_image_protocols::ImageViewInfo;
use libwayshot::region::{Size, Position, Region};

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
    WaylandError(#[from] libwayshot::ext_image_protocols::HaruhiError),
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

pub fn ext_capture_output(
    state: &mut HaruhiShotState,
    output: Option<String>,
    use_stdout: bool,
    pointer: bool,
) -> eyre::Result<WayshotResult, WayshotImageWriteError> {
    let outputs = state.outputs();
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
    let image_info = state.ext_capture_single_output(pointer.to_capture_option(), output)?;

    write_to_image(image_info, use_stdout)
}

use libwayshot::ext_image_protocols::{CaptureOption, ImageInfo};

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

fn write_to_image(
    image_info: ImageInfo,
    use_stdout: bool,
) -> Result<WayshotResult, WayshotImageWriteError> {
    if use_stdout {
        write_to_stdout(image_info)
    } else {
        write_to_file(image_info)
    }
}

use image::codecs::png::PngEncoder;
use std::io::{BufWriter, Write, stdout};

fn write_to_stdout(
    ImageInfo {
        data,
        width,
        height,
        color_type,
    }: ImageInfo,
) -> Result<WayshotResult, WayshotImageWriteError> {
    let stdout = stdout();
    let mut writer = BufWriter::new(stdout.lock());
    PngEncoder::new(&mut writer).write_image(&data, width, height, color_type.into())?;
    Ok(WayshotResult::StdoutSucceeded)
}

fn write_to_file(
    ImageInfo {
        data,
        width,
        height,
        color_type,
    }: ImageInfo,
) -> Result<WayshotResult, WayshotImageWriteError> {
    let file = random_file_path();
    let mut writer =
        std::fs::File::create(&file).map_err(WayshotImageWriteError::FileCreatedFailed)?;

    PngEncoder::new(&mut writer).write_image(&data, width, height, color_type.into())?;
    Ok(WayshotResult::SaveToFile(file))
}

fn random_file_path() -> PathBuf {
    let file_name = format!(
        "{}-haruhui.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    SAVEPATH.join(file_name)
}

use std::sync::LazyLock;

pub static SAVEPATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let Ok(home) = env::var("HOME") else {
        return PathBuf::from(TMP);
    };
    let targetpath = PathBuf::from(home).join("Pictures").join("haruhishot");
    if !targetpath.exists() && fs::create_dir_all(&targetpath).is_err() {
        return PathBuf::from(TMP);
    }
    targetpath
});

pub fn ext_capture_area(
    state: &mut HaruhiShotState,
    use_stdout: bool,
    pointer: bool,
) -> Result<WayshotResult, WayshotImageWriteError> {
    let ImageViewInfo {
        info:
            ImageInfo {
                data,
                width: img_width,
                height: img_height,
                color_type,
            },
        region:
            Region {
                position: Position { x, y },
                size: Size { width, height },
            },
    } = state.ext_capture_area2(pointer.to_capture_option(), |w_conn: &HaruhiShotState| {
        let info = libwaysip::get_area(
            Some(libwaysip::WaysipConnection {
                connection: w_conn.connection(),
                globals: w_conn.globals(),
            }),
            libwaysip::SelectionType::Area,
        )
        .map_err(|e| libwayshot::ext_image_protocols::HaruhiError::CaptureFailed(e.to_string()))?
        .ok_or(libwayshot::ext_image_protocols::HaruhiError::CaptureFailed(
            "Failed to capture the area".to_string(),
        ))?;
        waysip_to_region(info.size(), info.left_top_point())
    })?;

    let mut buff = std::io::Cursor::new(Vec::new());
    PngEncoder::new(&mut buff).write_image(&data, img_width, img_height, color_type.into())?;
    let img = image::load_from_memory_with_format(buff.get_ref(), image::ImageFormat::Png).unwrap();
    let clipimage = img.view(x as u32, y as u32, width as u32, height as u32);
    if use_stdout {
        let mut buff = std::io::Cursor::new(Vec::new());
        clipimage
            .to_image()
            .write_to(&mut buff, image::ImageFormat::Png)?;
        let content = buff.get_ref();
        let stdout = stdout();
        let mut writer = BufWriter::new(stdout.lock());
        writer.write_all(content)?;
        Ok(WayshotResult::StdoutSucceeded)
    } else {
        let file = random_file_path();
        clipimage.to_image().save(&file)?;
        Ok(WayshotResult::SaveToFile(file))
    }
}

pub fn waysip_to_region(
	size: libwaysip::Size,
	point: libwaysip::Position,
) -> Result<Region, libwayshot::ext_image_protocols::HaruhiError> {
	let size: Size = Size {
		width: size.width as u32,
		height: size.height as u32,
	};
	let position: Position = Position {
		x: point.x,
		y: point.y,
	};

	Ok(Region { position, size })
}

pub fn ext_capture_color(state: &mut HaruhiShotState) -> Result<WayshotResult, WayshotImageWriteError> {
	let ImageViewInfo {
		info:
		ImageInfo {
			data,
			width: img_width,
			height: img_height,
			color_type,
		},
		region:
		Region {
			position: Position { x, y },
			size: Size { width, height },
		},
	} = state.ext_capture_area2(CaptureOption::None, |w_conn: &HaruhiShotState| {
		let info = libwaysip::get_area(
			Some(libwaysip::WaysipConnection {
				connection: w_conn.connection(),
				globals: w_conn.globals(),
			}),
			libwaysip::SelectionType::Point,
		)
			.map_err(|e| libwayshot::ext_image_protocols::HaruhiError::CaptureFailed(e.to_string()))?
			.ok_or(libwayshot::ext_image_protocols::HaruhiError::CaptureFailed(
				"Failed to capture the area".to_string(),
			))?;
		waysip_to_region(info.size(), info.left_top_point())
	})?;

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