//! Diagnostic float rendering without the `float` feature: bit patterns.
//! See `feature_diagnostic` for the float-enabled rendering paths.
#![cfg(all(feature = "diagnostic", not(feature = "float")))]

use const_cbor::diagnostic::render;

#[test]
fn floats_render_as_bit_patterns_without_float_feature() {
    let mut out = [0u8; 64];
    // f16 1.5 -> f9 3e00
    let n = render(&[0xf9, 0x3e, 0x00], &mut out).unwrap();
    assert_eq!(n, "float(0x3e00)");
    // f32 -> fa + 4 bytes.
    let n = render(&[0xfa, 0x47, 0xc3, 0x50, 0x00], &mut out).unwrap();
    assert_eq!(n, "float(0x47c35000)");
    // f64 -> fb + 8 bytes.
    let input = [0xfb, 0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18];
    let n = render(&input, &mut out).unwrap();
    assert_eq!(n, "float(0x400921fb54442d18)");
}
