//! Colour picker mode (`--color`).

use eyre::Result;
use libwayshot::WayshotConnection;
use libwaysip::WaySip;

use crate::utils::waysip_to_region;

/// Freeze the screen, let the user click a pixel, and print its color.
pub fn pick(conn: &WayshotConnection) -> Result<()> {
    let image = conn
        .screenshot_freeze(
            |w_conn| {
                let info = WaySip::new()
                    .with_connection(w_conn.conn.clone())
                    .with_selection_type(libwaysip::SelectionType::Point)
                    .get()
                    .map_err(|e| libwayshot::Error::FreezeCallbackError(e.to_string()))?
                    .ok_or_else(|| {
                        libwayshot::Error::FreezeCallbackError(
                            "Failed to capture the point".to_string(),
                        )
                    })?;
                waysip_to_region(
                    libwaysip::Size {
                        width: 1,
                        height: 1,
                    },
                    info.left_top_point(),
                )
            },
            false,
        )?
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
