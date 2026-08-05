use crate::dispatch::should_apply_configure;

#[test]
fn zero_width_is_ignored() {
    assert!(!should_apply_configure((1920, 1080), (0, 1080)));
}

#[test]
fn zero_height_is_ignored() {
    assert!(!should_apply_configure((1920, 1080), (1920, 0)));
}

#[test]
fn zero_width_and_height_is_ignored() {
    assert!(!should_apply_configure((1920, 1080), (0, 0)));
}

#[test]
fn unchanged_size_does_not_reapply() {
    assert!(!should_apply_configure((1920, 1080), (1920, 1080)));
}

#[test]
fn changed_width_applies() {
    assert!(should_apply_configure((1920, 1080), (1280, 1080)));
}

#[test]
fn changed_height_applies() {
    assert!(should_apply_configure((1920, 1080), (1920, 720)));
}

#[test]
fn changed_width_and_height_applies() {
    assert!(should_apply_configure((1920, 1080), (1280, 720)));
}
