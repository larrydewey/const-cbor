//! Float codec coverage: all three widths on both sides, narrowing
//! fallbacks, out-of-space, and cross-type rejections.
#![cfg(feature = "float")]

use const_cbor::{Decoder, Encoder, Error};

#[test]
fn encoder_f16_forced_and_out_of_space() {
    // Values not exactly representable fall back to canonical NaN half
    // (0x7e00) — f16() is documented as lossy.
    let mut buf = [0u8; 8];
    let mut e = Encoder::new(&mut buf);
    e.f16(0.1).unwrap();
    let n = e.written();
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0xf9, 0x7e, 0x00]);
    // Exactly representable value keeps its bits.
    let mut buf2 = [0u8; 8];
    let mut e2 = Encoder::new(&mut buf2);
    e2.f16(1.5).unwrap();
    assert_eq!(&buf2[..3], &[0xf9, 0x3e, 0x00]);

    // Out of space inside write_head_exact (needs 3 bytes).
    let mut tiny = [0u8; 2];
    let mut e2 = Encoder::new(&mut tiny);
    assert!(matches!(e2.f32(1.0), Err(Error::OUT_OF_SPACE)));
    assert_eq!(e2.written(), 0);
}

#[test]
fn decoder_f32_accepts_all_widths() {
    // Half precision widened into f32.
    let mut dec = Decoder::new(&[0xf9, 0x3c, 0x00]);
    assert_eq!(dec.f32(), Ok(1.0));
    // Single precision passthrough.
    let mut dec = Decoder::new(&[0xfa, 0x47, 0xc3, 0x50, 0x00]);
    assert_eq!(dec.f32(), Ok(100000.0));
    // Double precision is NOT readable as f32 (would lose information).
    let mut dec = Decoder::new(&[0xfb, 0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18]);
    assert_eq!(dec.f32(), Err(Error::TYPE_MISMATCH));
    // Non-float major 7 item (true).
    let mut dec = Decoder::new(&[0xf5]);
    assert_eq!(dec.f32(), Err(Error::TYPE_MISMATCH));
    // Non-major-7 item.
    let mut dec = Decoder::new(&[0x01]);
    assert_eq!(dec.f32(), Err(Error::TYPE_MISMATCH));
}

#[test]
fn decoder_f64_accepts_all_widths() {
    // Half widened to f64.
    let mut dec = Decoder::new(&[0xf9, 0xbc, 0x00]);
    assert_eq!(dec.f64(), Ok(-1.0));
    // Single widened to f64.
    let mut dec = Decoder::new(&[0xfa, 0x47, 0xc3, 0x50, 0x00]);
    assert_eq!(dec.f64(), Ok(100000.0));
    // Double passthrough.
    let input = [0xfb, 0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18];
    let mut dec = Decoder::new(&input);
    assert_eq!(dec.f64(), Ok(core::f64::consts::PI));
    // Non-float item rejected.
    let mut dec = Decoder::new(&[0xf6]); // null
    assert_eq!(dec.f64(), Err(Error::TYPE_MISMATCH));
}

#[test]
fn float_narrowing_boundary_values() {
    let cases: [(f64, &[u8]); 5] = [
        (1.5, &[0xf9, 0x3e, 0x00]),                  // exact half
        (65504.0, &[0xf9, 0x7b, 0xff]),              // half max
        (100000.0, &[0xfa, 0x47, 0xc3, 0x50, 0x00]), // needs single
        (0.1, &[0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a]), // double
        (-0.0, &[0xf9, 0x80, 0x00]),                 // signed zero preserved
    ];
    for (v, want) in cases {
        let mut buf = [0u8; 16];
        let mut e = Encoder::new(&mut buf);
        e.f64(v).unwrap();
        let n = e.written();
        assert_eq!(&buf[..n], want, "encoding {v}");
        let mut d = Decoder::new(want);
        assert_eq!(d.f64(), Ok(v));
    }
}

#[test]
fn diagnostic_renders_f64_and_specials() {
    use const_cbor::diagnostic::render;
    let mut out = [0u8; 64];
    // Double-width value.
    let n = render(&[0xfb, 0x40, 0x09, 0x21, 0xfb, 0x54, 0x44, 0x2d, 0x18], &mut out).unwrap();
    assert_eq!(n, "3.141592653589793");
    // Negative infinity via single width.
    let n = render(&[0xfa, 0xff, 0x80, 0x00, 0x00], &mut out).unwrap();
    assert_eq!(n, "-Infinity");
    // NaN via half.
    let n = render(&[0xf9, 0x7e, 0x00], &mut out).unwrap();
    assert_eq!(n, "NaN");
}
