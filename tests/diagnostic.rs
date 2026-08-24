//! Diagnostic-notation rendering checked against RFC 8949 Appendix A
//! examples and edge cases.

#![cfg(feature = "diagnostic")]

use const_cbor::decode::SKIP_LIMIT;
use const_cbor::{Encoder, Error};

fn diag(f: impl FnOnce(&mut Encoder)) -> String {
    let mut buf = [0u8; 256];
    let mut enc = Encoder::new(&mut buf);
    f(&mut enc);
    let (out, n) = enc.finish();
    let mut text = [0u8; 512];
    const_cbor::diagnostic::render(&out[..n], &mut text).unwrap().to_string()
}

#[test]
fn appendix_a_scalars() {
    assert_eq!(
        diag(|e| {
            e.u32(0).unwrap();
        }),
        "0"
    );
    assert_eq!(
        diag(|e| {
            e.u32(10).unwrap();
        }),
        "10"
    );
    assert_eq!(
        diag(|e| {
            e.u32(1000).unwrap();
        }),
        "1000"
    );
    assert_eq!(
        diag(|e| {
            e.u64(u64::MAX).unwrap();
        }),
        "18446744073709551615"
    );
    assert_eq!(
        diag(|e| {
            e.i64(-1).unwrap();
        }),
        "-1"
    );
    assert_eq!(
        diag(|e| {
            e.i64(-1000).unwrap();
        }),
        "-1000"
    );
    // Magnitude beyond i64 stays exact.
    let mut buf = [0u8; 16];
    let mut enc = Encoder::new(&mut buf);
    enc.nint(u64::MAX).unwrap();
    let (out, n) = enc.finish();
    let mut text_buf = [0u8; 64];
    let text = const_cbor::diagnostic::render(&out[..n], &mut text_buf).unwrap();
    assert_eq!(text, "-18446744073709551616");
}

#[test]
fn appendix_a_strings() {
    assert_eq!(
        diag(|e| {
            e.bytes(b"\x01\x02\x03\x04\x05").unwrap();
        }),
        "h'0102030405'"
    );
    assert_eq!(
        diag(|e| {
            e.str("IETF").unwrap();
        }),
        "\"IETF\""
    );
    assert_eq!(
        diag(|e| {
            e.str("\u{00fc}").unwrap();
        }),
        "\"ü\""
    );
    assert_eq!(
        diag(|e| {
            e.str("\u{6c34}").unwrap();
        }),
        "\"水\""
    );
    // Escapes.
    assert_eq!(
        diag(|e| {
            e.str("a\"b\\c\nd\te").unwrap();
        }),
        "\"a\\\"b\\\\c\\nd\\te\""
    );
}

#[test]
fn appendix_a_structures() {
    assert_eq!(
        diag(|e| {
            e.array_len(3).unwrap();
            e.bool(true).unwrap();
            e.bool(false).unwrap();
            e.null().unwrap();
        }),
        "[true, false, null]"
    );
    assert_eq!(
        diag(|e| {
            e.map_len(2).unwrap();
            e.str("a").unwrap().u8(1).unwrap();
            e.str("b").unwrap().array_len(2).unwrap().u8(2).unwrap().u8(3).unwrap();
        }),
        "{\"a\": 1, \"b\": [2, 3]}"
    );
    // Empty containers.
    assert_eq!(
        diag(|e| {
            e.array_len(0).unwrap();
        }),
        "[]"
    );
    assert_eq!(
        diag(|e| {
            e.map_len(0).unwrap();
        }),
        "{}"
    );
}

#[test]
fn tags_and_simple_values() {
    assert_eq!(
        diag(|e| {
            e.tag(1).unwrap();
            e.u32(1363896240).unwrap();
        }),
        "1(1363896240)"
    );
    assert_eq!(
        diag(|e| {
            e.simple(const_cbor::Simple(99)).unwrap();
        }),
        "simple(99)"
    );
    assert_eq!(
        diag(|e| {
            e.undefined().unwrap();
        }),
        "undefined"
    );
}

#[cfg(feature = "float")]
#[test]
fn floats_render_rfc_style() {
    assert_eq!(
        diag(|e| {
            e.f32(1.5).unwrap();
        }),
        "1.5"
    );
    assert_eq!(
        diag(|e| {
            e.f64(-3.75).unwrap();
        }),
        "-3.75"
    );
    assert_eq!(
        diag(|e| {
            e.f32(0.0).unwrap();
        }),
        "0"
    );
    assert_eq!(
        diag(|e| {
            e.f32(f32::NAN).unwrap();
        }),
        "NaN"
    );
    assert_eq!(
        diag(|e| {
            e.f32(f32::INFINITY).unwrap();
        }),
        "Infinity"
    );
    assert_eq!(
        diag(|e| {
            e.f32(f32::NEG_INFINITY).unwrap();
        }),
        "-Infinity"
    );
}

#[cfg(feature = "indefinite")]
#[test]
fn indefinite_renders_with_underscore_markers() {
    assert_eq!(
        diag(|e| {
            e.str_indef(&["strea", "ming"]).unwrap();
        }),
        "(_ \"strea\", \"ming\")"
    );
    assert_eq!(
        diag(|e| {
            e.begin_array().unwrap();
            e.u8(1).unwrap();
            e.begin_array().unwrap().u8(2).unwrap().end_indef().unwrap();
            e.end_indef().unwrap();
        }),
        "[_ 1, [_ 2]]"
    );
    assert_eq!(
        diag(|e| {
            e.begin_map().unwrap();
            e.str("a").unwrap().u8(1).unwrap();
            e.end_indef().unwrap();
        }),
        "{_ \"a\": 1}"
    );
}

#[test]
fn deep_nesting_bounded_not_recursive() {
    // SKIP_LIMIT nested arrays render fine.
    let mut buf = [0u8; 256];
    let mut enc = Encoder::new(&mut buf);
    for _ in 0..SKIP_LIMIT - 2 {
        enc.array_len(1).unwrap();
    }
    enc.array_len(1).unwrap();
    enc.u8(7).unwrap();
    let (out, n) = enc.finish();
    // Render: must not panic or overflow the stack.
    let mut text = [0u8; 512];
    let r = const_cbor::diagnostic::render(&out[..n], &mut text).unwrap();
    assert!(r.starts_with('['));
    assert!(r.contains('7'));
    assert!(r.ends_with(']'));
}

#[test]
fn depth_exceeded_and_truncation_are_errors() {
    // Over-deep nesting.
    let mut buf = [0u8; 128];
    let mut enc = Encoder::new(&mut buf);
    for _ in 0..40 {
        enc.array_len(1).unwrap();
    }
    enc.u8(0).unwrap();
    let (out, n) = enc.finish();
    assert_eq!(
        const_cbor::diagnostic::render(&out[..n], &mut [0u8; 256]),
        Err(Error::DEPTH_EXCEEDED)
    );
    // Truncated input.
    assert_eq!(
        const_cbor::diagnostic::render(&[0x19, 0x03], &mut [0u8; 64]),
        Err(Error::TRUNCATED)
    );
    // Out of space.
    let mut small = [0u8; 4];
    assert_eq!(
        const_cbor::diagnostic::render(&[0x65, b'h', b'e', b'l', b'l', b'o'], &mut small),
        Err(Error::OUT_OF_SPACE)
    );
}

#[test]
fn write_to_forwards_to_fmt_targets() {
    struct Counted(String);
    impl core::fmt::Write for Counted {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.push_str(s);
            Ok(())
        }
    }
    let mut buf = [0u8; 16];
    let mut enc = Encoder::new(&mut buf);
    enc.u16(51966).unwrap();
    let (out, n) = enc.finish();

    let mut sink = Counted(String::new());
    const_cbor::diagnostic::write_to(&out[..n], &mut sink).unwrap();
    assert_eq!(sink.0, "51966");
}
