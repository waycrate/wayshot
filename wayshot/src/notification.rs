//! Desktop notifications via `notify-rust` (D-Bus / libnotify).

use eyre::Error;
use notify_rust::Notification;
use rustix::runtime::{self, Fork};
use std::path::Path;
use std::process::Command;

use crate::screenshot::ShotResult;

const TIMEOUT_MS: i32 = 5000;

pub fn send_success(
    result: &ShotResult,
    saved_location: Option<&Path>,
    action_command: Option<&str>,
) {
    let body = match result {
        ShotResult::Output { name } => format!("Screenshot of output '{name}' saved"),
        ShotResult::Toplevel { name } => format!("Screenshot of toplevel '{name}' saved"),
        ShotResult::Area => "Screenshot of selected area saved".to_string(),
        ShotResult::All => "Screenshot of all outputs saved".to_string(),
    };

    let mut notification = Notification::new();
    notification
        .summary("Screenshot Taken")
        .appname("wayshot")
        .timeout(TIMEOUT_MS);

    if let Some(path) = saved_location {
        let dir_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string();

        notification.action("open_location", "Open Folder");
        notification.action("default", "Open Folder");

        match unsafe { runtime::kernel_fork() } {
            Ok(Fork::Child(_)) => {
                if let Ok(handle) = notification.show() {
                    handle.wait_for_action(|action| {
                        if action == "open_location" || action == "default" {
                            let cmd = match action_command {
                                Some(custom) => custom.to_string(),
                                None => format!("xdg-open {dir_path}"),
                            };
                            let _ = Command::new("sh")
                                .args(["-c", &cmd])
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                        }
                    });
                }
                std::process::exit(0);
            }
            Ok(Fork::ParentOf(_)) => {}
            Err(e) => {
                tracing::error!("Fork failed for notification action: {}", e);
                let _ = notification.body(&body).show();
            }
        }
    } else {
        let _ = notification.body(&body).show();
    }
}

pub fn send_failure(error: &Error) {
    let _ = Notification::new()
        .summary("Screenshot Failed")
        .body(&error.to_string())
        .timeout(TIMEOUT_MS)
        .show();
}
