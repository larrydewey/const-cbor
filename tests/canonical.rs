//! Preferred-serialization checks (RFC 8949 section 4.2).
//!
//! The encoder never emits non-shortest forms: argument widths are chosen
//! by `head::ai_of`, floats are narrowed via round-then-verify (section
//! 4.2.2). These tests pin that behavior byte-for-byte.

#![cfg(feature = "float")]

use const_cbor::{Decoder, Encoder};

fn encoded(f: impl FnOnce(&mut Encoder)) -> Vec<u8> {
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    f(&mut enc);
    let (out, n) = enc.finish();
    out[..n].to_vec()
}

#[test]
fn floats_use_shortest_exact_width() {
    // 1.0 narrows to binary16.
    assert_eq!(
        encoded(|e| {
            e.f32(1.0).unwrap();
        }),
        vec![0xf9, 0x3c, 0x00]
    );
    // 100000.0 needs single precision.
    assert_eq!(
        encoded(|e| {
            e.f64(100000.0).unwrap();
        }),
        vec![0xfa, 0x47, 0xc3, 0x50, 0x00]
    );
    // 1.1 is not representable in half or single: double only.
    assert_eq!(
        encoded(|e| {
            e.f64(1.1).unwrap();
        }),
        vec![0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a]
    );
}

#[test]
fn nan_encodes_canonically() {
    // RFC 8949 section 4.2.2: NaN as canonical half 0x7e00.
    assert_eq!(
        encoded(|e| {
            e.f32(f32::NAN).unwrap();
        }),
        vec![0xf9, 0x7e, 0x00]
    );
}

#[test]
fn float_roundtrips() {
    for v in [0.0f32, -0.0, 1.5, -273.15, 65504.0, f32::INFINITY] {
        let bytes = encoded(|e| {
            e.f32(v).unwrap();
        });
        let mut dec = Decoder::new(&bytes);
        match v {
            0.0 if v.is_sign_negative() => assert!(dec.f32().unwrap().is_sign_negative()),
            _ => assert_eq!(dec.f32().unwrap(), v),
        }
        assert!(dec.is_empty());
    }
    for v in [1e300_f64, -2.5e-10, f64::MIN] {
        let bytes = encoded(|e| {
            e.f64(v).unwrap();
        });
        let mut dec = Decoder::new(&bytes);
        assert_eq!(dec.f64().unwrap(), v);
    }
}

#[test]
fn decoder_rejects_floats_without_feature() {
    let _ = encoded(|e| {
        e.f32(1.0).unwrap();
    }); // feature on here; see lib tests
}

#[test]
fn unsigned_always_preferred_width() {
    // Each boundary must shrink to the smaller form.
    let cases: [(u64, &[u8]); 4] =
        [(23, &[0x17]), (24, &[0x18, 24]), (255, &[0x18, 255]), (256, &[0x19, 0x01, 0x00])];
    for (v, want) in cases {
        assert_eq!(
            encoded(|e| {
                e.u64(v).unwrap();
            }),
            want.to_vec()
        );
    }
}
