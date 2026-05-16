//! Desktop notifications via `notify-rust` (D-Bus / libnotify).

use eyre::Error;
use notify_rust::{Hint, Notification, Urgency};
use rustix::runtime::{self, Fork};
use std::path::Path;
use std::process::Command;

use crate::config::NotificationConfig;
use crate::screenshot::ShotResult;

const DEFAULT_TIMEOUT_MS: i32 = 5000;

pub fn send_success(
    result: &ShotResult,
    saved_location: Option<&Path>,
    config: &NotificationConfig,
) {
    let body = match result {
        ShotResult::Output { name } => format!("Screenshot of output '{name}' saved"),
        ShotResult::Toplevel { name } => format!("Screenshot of toplevel '{name}' saved"),
        ShotResult::Area => "Screenshot of selected area saved".to_string(),
        ShotResult::All => "Screenshot of all outputs saved".to_string(),
    };

    let mut notification = build_base_notification(config, true);
    notification.body(&body);

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
                            let cmd = match &config.action {
                                Some(custom) => custom.clone(),
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
                let _ = notification.show();
            }
        }
    } else {
        let _ = notification.show();
    }
}

pub fn send_failure(error: &Error, config: &NotificationConfig) {
    let _ = build_base_notification(config, false)
        .body(&error.to_string())
        .show();
}

fn build_base_notification(config: &NotificationConfig, is_success: bool) -> Notification {
    let default_summary = if is_success {
        "Screenshot Taken"
    } else {
        "Screenshot Failed"
    };
    let summary = if is_success {
        config.success_summary.as_deref().unwrap_or(default_summary)
    } else {
        config.failure_summary.as_deref().unwrap_or(default_summary)
    };

    let mut n = Notification::new();
    n.summary(summary)
        .appname(config.app_name.as_deref().unwrap_or("wayshot"))
        .timeout(config.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

    if let Some(icon) = &config.icon {
        n.icon(icon);
    }

    if let Some(urgency) = &config.urgency {
        n.urgency(parse_urgency(urgency));
    }

    if let Some(sound) = &config.sound_name {
        n.sound_name(sound);
    }

    if let Some(transient) = config.transient {
        n.hint(Hint::Transient(transient));
    }

    if let Some(category) = &config.category {
        n.hint(Hint::Category(category.clone()));
    }

    n
}

fn parse_urgency(s: &str) -> Urgency {
    match s.to_lowercase().as_str() {
        "low" => Urgency::Low,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
    }
}
