//! Byte-exactness and round-trip tests derived from RFC 8949 Appendix A,
//! plus well-formedness rules from sections 3.3 and 4.2.

use const_cbor::{Decoder, Encoder, Error, Major, Simple};

#[track_caller]
fn assert_bytes(f: impl FnOnce(&mut Encoder), want: &[u8]) {
    let mut buf = [0u8; 512];
    let mut enc = Encoder::new(&mut buf);
    f(&mut enc);
    let (out, n) = enc.finish();
    assert_eq!(&out[..n], want);
}

#[test]
fn appendix_a_unsigned_integers() {
    assert_bytes(
        |e| {
            e.u32(0).unwrap();
        },
        &[0x00],
    );
    assert_bytes(
        |e| {
            e.u32(1).unwrap();
        },
        &[0x01],
    );
    assert_bytes(
        |e| {
            e.u32(10).unwrap();
        },
        &[0x0a],
    );
    assert_bytes(
        |e| {
            e.u32(23).unwrap();
        },
        &[0x17],
    );
    assert_bytes(
        |e| {
            e.u32(24).unwrap();
        },
        &[0x18, 0x18],
    );
    assert_bytes(
        |e| {
            e.u32(100).unwrap();
        },
        &[0x18, 0x64],
    );
    assert_bytes(
        |e| {
            e.u32(1000).unwrap();
        },
        &[0x19, 0x03, 0xe8],
    );
    assert_bytes(
        |e| {
            e.u64(1_000_000).unwrap();
        },
        &[0x1a, 0x00, 0x0f, 0x42, 0x40],
    );
    assert_bytes(
        |e| {
            e.u64(1_000_000_000_000).unwrap();
        },
        &[0x1b, 0x00, 0x00, 0x00, 0xe8, 0xd4, 0xa5, 0x10, 0x00],
    );
    assert_bytes(
        |e| {
            e.u64(u64::MAX).unwrap();
        },
        &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn appendix_a_negative_integers() {
    assert_bytes(
        |e| {
            e.i64(-1).unwrap();
        },
        &[0x20],
    );
    assert_bytes(
        |e| {
            e.i64(-10).unwrap();
        },
        &[0x29],
    );
    assert_bytes(
        |e| {
            e.i64(-100).unwrap();
        },
        &[0x38, 0x63],
    );
    assert_bytes(
        |e| {
            e.i64(-1000).unwrap();
        },
        &[0x39, 0x03, 0xe7],
    );
}

#[test]
fn appendix_a_strings_and_bytes() {
    assert_bytes(
        |e| {
            e.bytes(b"\x01\x02\x03\x04\x05").unwrap();
        },
        &[0x45, 1, 2, 3, 4, 5],
    );
    assert_bytes(
        |e| {
            e.str("IETF").unwrap();
        },
        &[0x64, b'I', b'E', b'T', b'F'],
    );
    assert_bytes(
        |e| {
            e.str("\u{00fc}").unwrap();
        },
        &[0x62, 0xc3, 0xbc],
    );
    assert_bytes(
        |e| {
            e.str("\u{6c34}").unwrap();
        },
        &[0x63, 0xe6, 0xb0, 0xb4],
    );
    assert_bytes(
        |e| {
            e.str("\u{10151}").unwrap();
        },
        &[0x64, 0xf0, 0x90, 0x85, 0x91],
    );
    assert_bytes(
        |e| {
            e.bytes(b"").unwrap();
        },
        &[0x40],
    );
}

#[test]
fn appendix_a_structures() {
    // [true, false, null]
    assert_bytes(
        |e| {
            e.array_len(3).unwrap();
            e.bool(true).unwrap();
            e.bool(false).unwrap();
            e.null().unwrap();
        },
        &[0x83, 0xf5, 0xf4, 0xf6],
    );
    // [1, 2, 3]
    assert_bytes(
        |e| {
            e.array_len(3).unwrap();
            e.u8(1).unwrap();
            e.u8(2).unwrap();
            e.u8(3).unwrap();
        },
        &[0x83, 0x01, 0x02, 0x03],
    );
    // {"a": 1, "b": [2, 3]}
    assert_bytes(
        |e| {
            e.map_len(2).unwrap();
            e.str("a").unwrap().u8(1).unwrap();
            e.str("b").unwrap().array_len(2).unwrap();
            e.u8(2).unwrap().u8(3).unwrap();
        },
        &[0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x82, 0x02, 0x03],
    );
    // ["a", {"b": "c"}]
    assert_bytes(
        |e| {
            e.array_len(2).unwrap();
            e.str("a").unwrap();
            e.map_len(1).unwrap();
            e.str("b").unwrap().str("c").unwrap();
        },
        &[0x82, 0x61, 0x61, 0xa1, 0x61, 0x62, 0x61, 0x63],
    );
    // (simple 99)
    assert_bytes(
        |e| {
            e.simple(Simple(99)).unwrap();
        },
        &[0xf8, 0x63],
    );
}

#[test]
fn appendix_a_tags() {
    assert_bytes(
        |e| {
            e.tag(1).unwrap();
            e.u32(1363896240).unwrap();
        },
        &[0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0],
    );
    assert_bytes(
        |e| {
            e.tag(23).unwrap();
            e.bytes(b"\x01\x02\x03\x04").unwrap();
        },
        &[0xd7, 0x44, 1, 2, 3, 4],
    );
}

#[test]
fn roundtrip_all_integer_widths() {
    let mut buf = [0u8; 128];
    let mut enc = Encoder::new(&mut buf);
    for v in [0u64, 23, 24, 255, 256, 65_535, 65_536, u64::from(u32::MAX), u64::MAX] {
        enc.u64(v).unwrap();
    }
    for v in [-1i64, -24, -25, -256, -65_536, i64::MIN] {
        enc.i64(v).unwrap();
    }
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    for v in [0u64, 23, 24, 255, 256, 65_535, 65_536, u64::from(u32::MAX), u64::MAX] {
        assert_eq!(dec.u64().unwrap(), v);
    }
    for v in [-1i64, -24, -25, -256, -65_536, i64::MIN] {
        assert_eq!(dec.i64().unwrap(), v);
    }
    assert!(dec.is_empty());
}

#[test]
fn roundtrip_strings_and_containers() {
    let mut buf = [0u8; 256];
    let mut enc = Encoder::new(&mut buf);
    enc.array_len(2).unwrap();
    enc.map_len(1).unwrap();
    enc.str("key").unwrap();
    enc.bytes(&[9, 9, 9]).unwrap();
    enc.str("\u{6c34}\u{10151}").unwrap();
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    assert_eq!(dec.array_len().unwrap(), 2);
    assert_eq!(dec.map_len().unwrap(), 1);
    assert_eq!(dec.str().unwrap(), "key");
    assert_eq!(dec.bytes().unwrap(), &[9, 9, 9]);
    assert_eq!(dec.str().unwrap(), "\u{6c34}\u{10151}");
    assert!(dec.is_empty());
}

#[test]
fn typed_narrowing_rejects_overflow() {
    let mut buf = [0u8; 16];
    let mut enc = Encoder::new(&mut buf);
    enc.u64(300).unwrap();
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    assert_eq!(dec.u8(), Err(Error::INTEGER_OUT_OF_RANGE));
}

#[test]
fn skip_traverses_nested_structures_iteratively() {
    let mut buf = [0u8; 256];
    let mut enc = Encoder::new(&mut buf);
    enc.array_len(3).unwrap();
    enc.str("skip me").unwrap();
    enc.map_len(1).unwrap();
    enc.array_len(2).unwrap();
    enc.u8(1).unwrap().u8(2).unwrap().tag(9).unwrap().u8(7).unwrap();
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    dec.skip().unwrap();
    assert!(dec.is_empty());
}

#[test]
fn deep_skip_is_bounded() {
    // 40 nested arrays exceed SKIP_LIMIT (32).
    let mut buf = [0u8; 128];
    let mut enc = Encoder::new(&mut buf);
    for _ in 0..40 {
        enc.array_len(1).unwrap();
    }
    enc.u8(0).unwrap();
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    assert_eq!(dec.skip(), Err(Error::DEPTH_EXCEEDED));
}

#[test]
fn malformed_input_never_panics() {
    // Truncated prefixes of a mixed item.
    let raw = [0x19_u8, 0x03, 0xe8, 0x65, b'h', b'i', 0x21, 0x22];
    for len in 0..raw.len() {
        let prefix = &raw[..len];
        let mut dec = Decoder::new(prefix);
        let _ = dec.skip();
    }
    // Reserved additional information 28..=30.
    for ai in 28_u8..=30 {
        let input = [ai];
        let mut dec = Decoder::new(&input);
        assert_eq!(dec.u64(), Err(Error::INVALID_ADDITIONAL_INFO));
    }
    // Indefinite length (ai = 31) is rejected for scalar reads.
    {
        let input = [0x1f];
        let mut dec = Decoder::new(&input);
        // Indefinite scalars are not a thing (RFC section 3.2 limits
        // indefinite lengths to strings, arrays, maps).
        assert_eq!(dec.u64(), Err(Error::INDEFINITE_UNSUPPORTED));
    }
    // Two-byte simple below 32 is invalid (RFC 8949 section 3.3).
    let mut dec = Decoder::new(&[0xf8, 20]);
    assert_eq!(dec.simple(), Err(Error::INVALID_SIMPLE));
    // Invalid UTF-8 payload.
    let mut dec = Decoder::new(&[0x62, 0xff, 0xfe]);
    assert_eq!(dec.str(), Err(Error::INVALID_UTF8));
    // Declared length exceeding input.
    let mut dec = Decoder::new(&[0x65, b'a']);
    assert_eq!(dec.str(), Err(Error::TRUNCATED));
    // Type mismatch.
    let mut dec = Decoder::new(&[0x45, 1, 2, 3, 4, 5]); // bytes
    assert_eq!(dec.str(), Err(Error::TYPE_MISMATCH));
    // Trailing bytes are detectable.
    let mut dec = Decoder::new(&[0x01, 0x02]);
    assert_eq!(dec.u8(), Ok(1));
    assert!(!dec.is_empty());
}

#[test]
fn peek_and_null_helpers() {
    let mut dec = Decoder::new(&[0xf6]);
    assert_eq!(dec.peek_major(), Some(Major::Simple));
    assert!(dec.is_null().unwrap());
    assert!(dec.is_empty());

    let mut dec = Decoder::new(&[0x01]);
    assert!(!dec.is_null().unwrap());
    assert_eq!(dec.u8(), Ok(1));
}
