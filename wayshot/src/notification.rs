//! Desktop notifications via `notify-rust` (D-Bus / libnotify).

use eyre::Error;
use notify_rust::Notification;

use crate::utils::ShotResult;

const TIMEOUT_MS: i32 = 5000;

pub fn send_success(result: &ShotResult) {
    let body = match result {
        ShotResult::Output { name } => format!("Screenshot of output '{name}' saved"),
        ShotResult::Toplevel { name } => format!("Screenshot of toplevel '{name}' saved"),
        ShotResult::Area => "Screenshot of selected area saved".to_string(),
        ShotResult::All => "Screenshot of all outputs saved".to_string(),
    };
    let _ = Notification::new()
        .summary("Screenshot Taken")
        .body(&body)
        .timeout(TIMEOUT_MS)
        .show();
}

pub fn send_failure(error: &Error) {
    let _ = Notification::new()
        .summary("Screenshot Failed")
        .body(&error.to_string())
        .timeout(TIMEOUT_MS)
        .show();
}
