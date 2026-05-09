use super::palette;
use super::*;

#[test]
fn styled_char_default_is_space() {
    let sc = StyledChar::default();
    assert_eq!(sc.ch, ' ');
    assert!(!sc.bold);
}

#[test]
fn styled_char_default_color_is_text() {
    let sc = StyledChar::default();
    assert_eq!(sc.color, palette::TEXT);
}

#[test]
fn palette_bg_rgb() {
    let c = palette::BG;
    assert_eq!(c.r(), 18);
    assert_eq!(c.g(), 18);
    assert_eq!(c.b(), 28);
}

#[test]
fn palette_green_rgb() {
    let c = palette::GREEN;
    assert_eq!(c.r(), 78);
    assert_eq!(c.g(), 201);
    assert_eq!(c.b(), 148);
}
