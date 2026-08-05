use crate::config::NotificationConfig;
use crate::notification::build_base_notification;

#[test]
fn success_notification_uses_default_summary_and_appname() {
    let config = NotificationConfig::default();
    let n = build_base_notification(&config, true);
    assert_eq!(n.summary, "Screenshot Taken");
    assert_eq!(n.appname, "wayshot");
    assert_eq!(n.timeout, notify_rust::Timeout::Milliseconds(5000));
}

#[test]
fn failure_notification_uses_default_summary() {
    let config = NotificationConfig::default();
    let n = build_base_notification(&config, false);
    assert_eq!(n.summary, "Screenshot Failed");
}

#[test]
fn custom_summaries_override_defaults() {
    let config = NotificationConfig {
        success_summary: Some("Custom Success".to_string()),
        failure_summary: Some("Custom Failure".to_string()),
        ..NotificationConfig::default()
    };
    assert_eq!(
        build_base_notification(&config, true).summary,
        "Custom Success"
    );
    assert_eq!(
        build_base_notification(&config, false).summary,
        "Custom Failure"
    );
}

#[test]
fn custom_app_name_and_timeout() {
    let config = NotificationConfig {
        app_name: Some("my-wayshot".to_string()),
        timeout_ms: Some(1234),
        ..NotificationConfig::default()
    };
    let n = build_base_notification(&config, true);
    assert_eq!(n.appname, "my-wayshot");
    assert_eq!(n.timeout, notify_rust::Timeout::Milliseconds(1234));
}

#[test]
fn icon_is_set_when_configured() {
    let config = NotificationConfig {
        icon: Some("camera-photo".to_string()),
        ..NotificationConfig::default()
    };
    let n = build_base_notification(&config, true);
    assert_eq!(n.icon, "camera-photo");
}

#[test]
fn icon_is_empty_when_not_configured() {
    let config = NotificationConfig::default();
    let n = build_base_notification(&config, true);
    assert!(n.icon.is_empty());
}
