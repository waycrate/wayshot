use crate::listing::{DisplayInfo, PositionInfo, SizeInfo, ToplevelInfo};

#[test]
fn display_info_serializes_with_expected_shape() {
    let info = DisplayInfo {
        name: "eDP-1".to_string(),
        description: "Built-in display".to_string(),
        size: SizeInfo {
            width: 1920,
            height: 1080,
        },
        logical_size: SizeInfo {
            width: 1920,
            height: 1080,
        },
        position: PositionInfo { x: 0, y: 0 },
    };
    let value = serde_json::to_value(&info).expect("should serialize");
    assert_eq!(value["name"], "eDP-1");
    assert_eq!(value["description"], "Built-in display");
    assert_eq!(value["size"]["width"], 1920);
    assert_eq!(value["size"]["height"], 1080);
    assert_eq!(value["logical_size"]["width"], 1920);
    assert_eq!(value["position"]["x"], 0);
    assert_eq!(value["position"]["y"], 0);
}

#[test]
fn toplevel_info_serializes_with_expected_shape() {
    let info = ToplevelInfo {
        title: "Terminal".to_string(),
        app_id: "org.example.Terminal".to_string(),
        identifier: "abc123".to_string(),
    };
    let value = serde_json::to_value(&info).expect("should serialize");
    assert_eq!(value["title"], "Terminal");
    assert_eq!(value["app_id"], "org.example.Terminal");
    assert_eq!(value["identifier"], "abc123");
}

#[test]
fn size_info_generic_over_float() {
    let size: SizeInfo<f64> = SizeInfo {
        width: 1.5,
        height: 2.5,
    };
    let value = serde_json::to_value(size).expect("should serialize");
    assert_eq!(value["width"], 1.5);
    assert_eq!(value["height"], 2.5);
}

#[test]
fn list_of_display_info_serializes_as_json_array() {
    let outputs = vec![DisplayInfo {
        name: "eDP-1".to_string(),
        description: "d".to_string(),
        size: SizeInfo {
            width: 1,
            height: 1,
        },
        logical_size: SizeInfo {
            width: 1,
            height: 1,
        },
        position: PositionInfo { x: 0, y: 0 },
    }];
    let json = serde_json::to_string_pretty(&outputs).expect("should serialize");
    assert!(json.starts_with('['));
    assert!(json.contains("\"eDP-1\""));
}
