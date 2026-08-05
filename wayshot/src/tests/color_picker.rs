use crate::color_picker::rgb_to_hsl;

#[test]
fn rgb_to_hsl_black() {
    assert_eq!(rgb_to_hsl(0, 0, 0), (0, 0, 0));
}

#[test]
fn rgb_to_hsl_white() {
    assert_eq!(rgb_to_hsl(255, 255, 255), (0, 0, 100));
}

#[test]
fn rgb_to_hsl_pure_red() {
    let (h, s, l) = rgb_to_hsl(255, 0, 0);
    assert_eq!(h, 0);
    assert_eq!(s, 100);
    assert_eq!(l, 50);
}

#[test]
fn rgb_to_hsl_pure_green() {
    let (h, s, l) = rgb_to_hsl(0, 255, 0);
    assert_eq!(h, 120);
    assert_eq!(s, 100);
    assert_eq!(l, 50);
}

#[test]
fn rgb_to_hsl_pure_blue() {
    let (h, s, l) = rgb_to_hsl(0, 0, 255);
    assert_eq!(h, 240);
    assert_eq!(s, 100);
    assert_eq!(l, 50);
}

#[test]
fn rgb_to_hsl_gray_has_zero_saturation() {
    let (_, s, l) = rgb_to_hsl(128, 128, 128);
    assert_eq!(s, 0);
    assert_eq!(l, 50);
}
