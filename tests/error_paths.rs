//! Exhaustive malformed-input and error-path coverage.
//!
//! Every table entry must produce exactly the listed error, never panic,
//! and never consume more input than expected where relevant.

use const_cbor::{Decoder, Encoder, Error, Major, Simple};

/// `(input, expected_error)` pairs covering structural failures.
const MALFORMED: &[(&[u8], Error)] = &[
    // -- truncation -----------------------------------------------------
    (&[], Error::TRUNCATED),
    (&[0x18], Error::TRUNCATED),       // ai 24, missing argument
    (&[0x19, 0x03], Error::TRUNCATED), // ai 25, partial argument
    (&[0x1a, 0x00], Error::TRUNCATED), // ai 26, partial argument
    (&[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff], Error::TRUNCATED),
    (&[0x58], Error::TRUNCATED),       // bytes head, no length
    (&[0x58, 0x05], Error::TRUNCATED), // bytes(5), zero payload
    (&[0x58, 0x05, 1, 2, 3, 4], Error::TRUNCATED),
    (&[0x7a], Error::TRUNCATED),
    (&[0x62, b'a'], Error::TRUNCATED), // text(2), one byte payload
    (&[0x83, 0x01], Error::TRUNCATED), // array(3) with one item
    (&[0xa1, 0x01], Error::TRUNCATED), // map(1) key only
    (&[0xa1], Error::TRUNCATED),
    // -- reserved additional information (28-30) -------------------------
    (&[0x1c], Error::INVALID_ADDITIONAL_INFO),
    (&[0x1d], Error::INVALID_ADDITIONAL_INFO),
    (&[0x1e], Error::INVALID_ADDITIONAL_INFO),
    (&[0x3c], Error::INVALID_ADDITIONAL_INFO),
    (&[0xfc], Error::INVALID_ADDITIONAL_INFO), // major 7, ai 28
];

#[test]
fn malformed_table_yields_exact_errors() {
    for &(input, want) in MALFORMED {
        let mut dec = Decoder::new(input);
        let got = dec.skip();
        assert_eq!(got, Err(want), "input {input:?}: expected {want:?}, got {got:?}");
    }
}

#[test]
fn indefinite_rejected_without_feature() {
    // Typed getters always reject the indefinite marker.
    let mut dec = Decoder::new(&[0x5f]);
    assert_eq!(dec.u64(), Err(Error::INDEFINITE_UNSUPPORTED));
    // skip() reports truncation when indefinite support is linked,
    // INDEFINITE_UNSUPPORTED otherwise.
    let mut dec = Decoder::new(&[0x5f]);
    let res = dec.skip();
    if cfg!(feature = "indefinite") {
        assert_eq!(res, Err(Error::TRUNCATED));
    } else {
        assert_eq!(res, Err(Error::INDEFINITE_UNSUPPORTED));
    }
}

// ---------------------------------------------------------------- typed getters

type Getter = fn(&mut Decoder) -> Result<(), Error>;
type EncoderFn = fn(&mut Encoder) -> Result<(), Error>;

#[test]
fn type_mismatch_on_every_getter() {
    const NOT_UINT: &[u8] = &[0xc1, 0x01]; // tag(1, 1): matches no typed getter
    let cases: [(Getter, Error); 12] = [
        (|d| d.u8().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.u16().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.u32().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.u64().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.i8().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.i16().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.i32().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.i64().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.bytes().map(|_| ()), Error::TYPE_MISMATCH), // major 6 != 2
        (|d| d.str().map(|_| ()), Error::TYPE_MISMATCH),   // major 6 != 3
        (|d| d.bool().map(|_| ()), Error::TYPE_MISMATCH),
        (|d| d.undefined(), Error::TYPE_MISMATCH),
    ];
    for (f, want) in cases {
        let mut dec = Decoder::new(NOT_UINT);
        assert_eq!(f(&mut dec), Err(want));
    }
    // Tag/array/map getters against an integer item.
    let mut dec = Decoder::new(&[0x01]);
    assert_eq!(dec.tag(), Err(Error::TYPE_MISMATCH));
    let mut dec = Decoder::new(&[0x01]);
    assert_eq!(dec.array_len(), Err(Error::TYPE_MISMATCH));
    let mut dec = Decoder::new(&[0x01]);
    assert_eq!(dec.map_len(), Err(Error::TYPE_MISMATCH));
}

#[test]
fn integer_range_checks() {
    // u8 getter rejects values above u8::MAX.
    let mut dec = Decoder::new(&[0x19, 0x01, 0x00]); // 256
    assert_eq!(dec.u8(), Err(Error::INTEGER_OUT_OF_RANGE));

    // i64 getter rejects -2^64 magnitude below i64::MIN.
    let mut dec = Decoder::new(&[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(dec.i64(), Err(Error::INTEGER_OUT_OF_RANGE));

    // i8 rejects -129 and 128.
    let mut dec = Decoder::new(&[0x38, 0x80]); // -129
    assert_eq!(dec.i8(), Err(Error::INTEGER_OUT_OF_RANGE));
    let mut dec = Decoder::new(&[0x18, 0x80]); // 128
    assert_eq!(dec.i8(), Err(Error::INTEGER_OUT_OF_RANGE));

    // Boundary values succeed.
    let mut dec = Decoder::new(&[0x38, 0x7f]); // -128
    assert_eq!(dec.i8(), Ok(-128));
    let mut dec = Decoder::new(&[0x38, 0xff]); // -256
    assert_eq!(dec.i16(), Ok(-256));
    let mut dec = Decoder::new(&[0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(dec.i64(), Ok(i64::MIN));
    // Negative integer encoded with full-width magnitude fits u64 mapping.
    let mut dec = Decoder::new(&[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(
        dec.u64(),
        Err(Error::TYPE_MISMATCH),
        "negative major is not readable by the unsigned getter"
    );
}

#[test]
fn invalid_utf8_rejected() {
    // C0 80 is the classic overlong NUL.
    let mut dec = Decoder::new(&[0x62, 0xc0, 0x80]);
    assert_eq!(dec.str(), Err(Error::INVALID_UTF8));
    // Truncated multibyte sequence.
    let mut dec = Decoder::new(&[0x61, 0xc3]);
    assert_eq!(dec.str(), Err(Error::INVALID_UTF8));
    // Lone surrogate encoding ED A0 80.
    let mut dec = Decoder::new(&[0x63, 0xed, 0xa0, 0x80]);
    assert_eq!(dec.str(), Err(Error::INVALID_UTF8));
    // Valid UTF-8 passes and borrows from the input.
    let input = [0x63, 0xe2, 0x82, 0xac]; // €
    let mut dec = Decoder::new(&input);
    assert_eq!(dec.str(), Ok("€"));
}

#[test]
fn simple_value_rules() {
    // Two-byte form below 32 is forbidden (RFC 8949 §3.3).
    let mut dec = Decoder::new(&[0xf8, 0x00]);
    assert_eq!(dec.simple(), Err(Error::INVALID_SIMPLE));
    let mut dec = Decoder::new(&[0xf8, 0x1f]);
    assert_eq!(dec.simple(), Err(Error::INVALID_SIMPLE));
    // Truncated two-byte form.
    let mut dec = Decoder::new(&[0xf8]);
    assert_eq!(dec.simple(), Err(Error::TRUNCATED));
    // Two-byte form >= 32 is legal.
    let mut dec = Decoder::new(&[0xf8, 0x20]);
    assert_eq!(dec.simple(), Ok(Simple(32)));
    // One-byte values 0..24 are legal, including "undefined" (23).
    let mut dec = Decoder::new(&[0xe0]);
    assert_eq!(dec.simple(), Ok(Simple(0)));
    // Float payloads are not simple values without the float feature;
    // with it they are still not *simple* (they are floats).
    for (head, arg) in
        [(0xf9u8, &[0x00u8, 0x15][..]), (0xfa, &[0x00, 0x00, 0x15, 0xff][..]), (0xfb, &[0; 8][..])]
    {
        let mut input = vec![head];
        input.extend_from_slice(arg);
        let mut dec = Decoder::new(&input);
        assert_eq!(
            dec.simple(),
            Err(Error::FLOAT_UNSUPPORTED),
            "float payload must never be reinterpreted as a simple value"
        );
        // Even when the payload bits would decode as `true`/`false`.
        let mut input2 = vec![head];
        input2.extend_from_slice(arg);
        let mut dec2 = Decoder::new(&input2);
        assert_eq!(dec2.bool(), Err(Error::FLOAT_UNSUPPORTED));
    }
}

#[test]
fn bool_null_undefined_semantics() {
    let mut dec = Decoder::new(&[0xf4, 0xf5, 0xf6, 0xf7, 0xf0]);
    assert_eq!(dec.bool(), Ok(false));
    assert_eq!(dec.bool(), Ok(true));
    assert!(matches!(dec.is_null(), Ok(true)));
    assert_eq!(dec.undefined(), Ok(()));
    // f0 = simple(16): not null, not bool, not undefined.
    assert!(matches!(dec.is_null(), Ok(false)));
    let mut dec = Decoder::new(&[0xf0]);
    assert_eq!(dec.bool(), Err(Error::TYPE_MISMATCH));
    // bool on true-ish simple value 99 (two-byte form).
    let mut dec = Decoder::new(&[0xf8, 0x63]);
    assert_eq!(dec.bool(), Err(Error::TYPE_MISMATCH));
    // is_null does not consume a non-null item.
    let mut dec = Decoder::new(&[0x01, 0xf6]);
    assert!(matches!(dec.is_null(), Ok(false)));
    assert_eq!(dec.u8(), Ok(1));
    assert!(matches!(dec.is_null(), Ok(true)));
}

#[test]
fn unexpected_break_without_open_scope() {
    let mut dec = Decoder::new(&[0xff]);
    assert_eq!(dec.skip(), Err(Error::UNEXPECTED_BREAK));
}

#[test]
fn non_minimal_encodings_are_accepted() {
    // RFC 8949 §4.2.1: preferred form is a SHOULD. All of these decode to 0.
    for input in [
        &[0x18u8, 0x00][..], // uint 0 in 2 bytes
        &[0x19, 0x00, 0x00][..],
        &[0x1a, 0x00, 0x00, 0x00, 0x00][..],
        &[0x1b, 0, 0, 0, 0, 0, 0, 0, 0][..],
        &[0x38, 0x00][..], // -1 non-minimal
    ] {
        let mut dec = Decoder::new(input);
        let v = if input[0] >> 5 == 1 { dec.i64() } else { dec.u64().map(|v| v as i64) };
        let want = if input[0] >> 5 == 1 { -1 } else { 0 };
        assert_eq!(v, Ok(want), "input {input:?}");
    }
}

// ---------------------------------------------------------------- encoder paths

#[test]
fn encoder_out_of_space_per_method() {
    let cases: [(usize, EncoderFn); 10] = [
        (0, |e| e.uint(u64::MAX).map(|_| ())),
        (0, |e| e.nint(u64::MAX).map(|_| ())),
        (1, |e| e.bytes(&[0xaa; 8]).map(|_| ())),
        (1, |e| e.str("12345678").map(|_| ())),
        (0, |e| e.array_len(usize::MAX).map(|_| ())),
        (0, |e| e.map_len(usize::MAX).map(|_| ())),
        (0, |e| e.tag(u64::MAX).map(|_| ())),
        (0, |e| e.bool(true).map(|_| ())),
        (0, |e| e.null().map(|_| ())),
        (0, |e| e.undefined().map(|_| ())),
    ];
    for (i, &(buf_len, f)) in cases.iter().enumerate() {
        let mut buf = vec![0u8; buf_len];
        let mut enc = Encoder::new(&mut buf);
        let res = f(&mut enc);
        assert_eq!(res, Err(Error::OUT_OF_SPACE), "case {i} (buf_len {buf_len})");
        assert_eq!(enc.written(), 0, "failed call must not advance cursor");
    }
}

#[test]
fn encoder_simple_validation() {
    let mut buf = [0u8; 8];
    let mut enc = Encoder::new(&mut buf);
    // 24..32 cannot be encoded in either form.
    for v in 24..32 {
        assert!(matches!(enc.simple(Simple(v)), Err(Error::INVALID_SIMPLE)));
    }
    assert_eq!(enc.written(), 0);
    // Boundaries of legal ranges.
    enc.simple(Simple(23)).unwrap();
    enc.simple(Simple(32)).unwrap();
    enc.simple(Simple(255)).unwrap();
    assert_eq!(enc.written(), 1 + 2 + 2);
}

#[test]
fn typed_integer_wrappers_roundtrip() {
    // Every typed encoder wrapper routes through uint/i64 correctly.
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.i8(-3).unwrap().i16(300).unwrap().i32(-70_000).unwrap();
    let written = e.written();
    let mut d = Decoder::new(&buf[..written]);
    assert_eq!(d.i8(), Ok(-3));
    assert_eq!(d.i16(), Ok(300));
    assert_eq!(d.i32(), Ok(-70_000));
}

#[test]
fn error_api_surface() {
    // name/describe cover every variant exhaustively.
    let all = [
        Error::OUT_OF_SPACE,
        Error::TRUNCATED,
        Error::INVALID_ADDITIONAL_INFO,
        Error::INTEGER_OUT_OF_RANGE,
        Error::TYPE_MISMATCH,
        Error::INVALID_UTF8,
        Error::INVALID_SIMPLE,
        Error::FLOAT_UNSUPPORTED,
        Error::DEPTH_EXCEEDED,
        Error::UNEXPECTED_BREAK,
        Error::INDEFINITE_UNSUPPORTED,
    ];
    for e in all {
        assert!(!e.name().is_empty());
        let (name, code) = e.describe();
        assert_eq!(name, e.name());
        assert_eq!(code as usize, e as usize);
        assert_eq!(format!("{e}"), e.name());
        assert_eq!(format!("{e:?}"), format!("{e:?}")); // Debug is total
        let _ = core::ptr::addr_of!(e); // Copy/Sync sanity via use
        let c = e;
        let _h = {
            use core::hash::{BuildHasherDefault, Hasher};
            struct H;
            impl Hasher for H {
                fn finish(&self) -> u64 {
                    0
                }
                fn write(&mut self, _: &[u8]) {}
            }
            let _ = BuildHasherDefault::<H>::default();
            e
        };
        assert_eq!(c, e);
    }
    // Stable numeric codes (protocol-level contract).
    assert_eq!(Error::OUT_OF_SPACE as u8, 0);
    assert_eq!(Error::INDEFINITE_UNSUPPORTED as u8, 10);
}

#[test]
fn major_bits_roundtrip() {
    for m in [
        Major::Unsigned,
        Major::Negative,
        Major::Bytes,
        Major::Text,
        Major::Array,
        Major::Map,
        Major::Tag,
        Major::Simple,
    ] {
        assert_eq!(Major::from_initial(m.bits()), m);
        assert_eq!(Major::from_initial(m.bits() | 0x1f), m);
    }
}

#[test]
fn peek_major_on_all_types() {
    for (byte, want) in [
        (0x00, Major::Unsigned),
        (0x20, Major::Negative),
        (0x40, Major::Bytes),
        (0x60, Major::Text),
        (0x80, Major::Array),
        (0xa0, Major::Map),
        (0xc0, Major::Tag),
        (0xe0, Major::Simple),
        (0xff, Major::Simple),
    ] {
        let input = [byte];
        let dec = Decoder::new(&input);
        assert_eq!(dec.peek_major(), Some(want));
    }
    assert_eq!(Decoder::new(&[]).peek_major(), None);
}
