//! Colour picker mode (`--color`).

use eyre::Result;
use libwayshot::WayshotConnection;

use crate::utils::get_region_point;

/// Let the user click a pixel and print its color. When `freeze` is true, the screen is frozen first.
pub fn pick(conn: &WayshotConnection, freeze: bool) -> Result<()> {
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
    println!("RGBA       : R:{r}, G:{g}, B:{b}, A:{a}");
    println!(
        "RGBA(float): R:{:.2}, G:{:.2}, B:{:.2}, A:{:.2}",
        r as f32 / 255.,
        g as f32 / 255.,
        b as f32 / 255.,
        a as f32 / 255.
    );
    println!("16hex      : #{:02x}{:02x}{:02x}{:02x}", r, g, b, a);
    Ok(())
}
