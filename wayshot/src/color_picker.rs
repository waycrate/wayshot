//! Colour picker mode (`--color`).

use eyre::Result;
use libwayshot::WayshotConnection;

use crate::cli::ColorFormat;
use crate::utils::get_region_point;

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (u16, u8, u8) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;

    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    let h = ((h + 360.0) % 360.0) as u16;
    let s = (s * 100.0).round() as u8;
    let l = (l * 100.0).round() as u8;
    (h, s, l)
}

/// Let the user click a pixel and print its color. When `freeze` is true, the screen is frozen first.
pub fn pick(conn: &WayshotConnection, freeze: bool, format: ColorFormat) -> Result<()> {
    let image = (if freeze {
        conn.screenshot_freeze(
            |w_conn| get_region_point(w_conn).map_err(libwayshot::Error::FreezeCallbackError),
            false,
        )?
    } else {
        let region = get_region_point(conn).map_err(|e| eyre::eyre!("{e}"))?;
        conn.screenshot(region, false)?
    })
    .to_rgba8();

    let [r, g, b, a] = image.get_pixel(0, 0).0;
    match format {
        ColorFormat::Hex => println!("#{:02x}{:02x}{:02x}", r, g, b),
        ColorFormat::HexAlpha => println!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
        ColorFormat::Rgb => println!("rgb({r}, {g}, {b})"),
        ColorFormat::Rgba => println!("rgba({r}, {g}, {b}, {:.2})", a as f32 / 255.),
        ColorFormat::Hsl => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            println!("hsl({h}, {s}%, {l}%)");
        }
        ColorFormat::Plain => {
            println!("RGBA       : R:{r}, G:{g}, B:{b}, A:{a}");
            println!(
                "RGBA(float): R:{:.2}, G:{:.2}, B:{:.2}, A:{:.2}",
                r as f32 / 255.,
                g as f32 / 255.,
                b as f32 / 255.,
                a as f32 / 255.
            );
            println!("16hex      : #{:02x}{:02x}{:02x}{:02x}", r, g, b, a);
        }
    }
    Ok(())
}
