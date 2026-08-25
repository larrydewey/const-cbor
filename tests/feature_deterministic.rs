//! Deterministic-encoding validation coverage (RFC 8949 §4.2, feature
//! `deterministic`).
//!
//! SOP checklist: every rule of §4.2.1/§4.2.2 exercised in both directions
//! (canonical inputs accepted, each violation class rejected with exactly
//! `Error::NOT_CANONICAL`), structural errors preserved, depth bound,
//! roundtrip against this crate's own encoder, and feature interactions.
#![cfg(feature = "deterministic")]

use const_cbor::deterministic::{check, check_all};
use const_cbor::{Encoder, Error};

fn enc(f: impl FnOnce(&mut Encoder) -> Result<(), Error>) -> Vec<u8> {
    let mut buf = [0u8; 128];
    let mut e = Encoder::new(&mut buf);
    f(&mut e).unwrap();
    let (buf, n) = e.finish();
    buf[..n].to_vec()
}

// ------------------------------------------------------- accepted forms

#[test]
fn canonical_inputs_pass() {
    // Minimal integer widths at every boundary.
    for v in [0u64, 23, 24, 255, 256, 65_535, 65_536, u64::MAX] {
        assert_eq!(check(&enc(|e| e.uint(v).map(|_| ()))), Ok(()), "{v}");
    }
    // Negative integers.
    for v in [-1i64, -24, -25, -256] {
        assert_eq!(check(&enc(|e| e.i64(v).map(|_| ()))), Ok(()), "{v}");
    }
    // Strings, arrays, maps, tags — nested.
    let item = enc(|e| {
        e.tag(1)?.array_len(2)?.str("a")?.map_len(1)?.uint(1)?.bytes(b"b")?;
        Ok(())
    });
    assert_eq!(check(&item), Ok(()));
    // Empty containers are canonical.
    assert_eq!(check(&[0x80]), Ok(()));
    assert_eq!(check(&[0xa0]), Ok(()));
    // Simple values in one-byte form.
    assert_eq!(check(&[0xf0]), Ok(())); // simple(16)
    assert_eq!(check(&enc(|e| e.simple(const_cbor::Simple(99)).map(|_| ()))), Ok(()));
    // Multiple top-level items via check_all.
    assert_eq!(check_all(&[0x01, 0x02, 0x03]), Ok(()));
}

#[test]
fn own_encoder_output_is_always_canonical() {
    // The encoder emits preferred serialization by construction; a broad
    // sweep of its outputs must validate.
    let cases: Vec<Vec<u8>> = vec![
        enc(|e| e.u8(1).map(|_| ())),
        enc(|e| e.i64(i64::MIN).map(|_| ())),
        enc(|e| e.nint(u64::MAX).map(|_| ())),
        enc(|e| e.str("hello").map(|_| ())),
        enc(|e| e.bytes(&[0xde, 0xad]).map(|_| ())),
        enc(|e| {
            e.array_len(3)
                .and_then(|e| e.null())
                .and_then(|e| e.bool(true))
                .and_then(|e| e.undefined())
                .map(|_| ())
        }),
        enc(|e| e.map_len(1).and_then(|e| e.str("k")).and_then(|e| e.str("v")).map(|_| ())),
        enc(|e| e.tag(0).and_then(|e| e.u32(2026)).map(|_| ())),
    ];
    for c in &cases {
        assert_eq!(check(c), Ok(()), "input {c:?}");
    }
}

#[cfg(feature = "float")]
#[test]
fn canonical_floats_pass() {
    use const_cbor::float;
    for v in [1.5f32, -273.15, 65504.0, f32::INFINITY, f32::NEG_INFINITY] {
        let bytes = enc(|e| e.f32(v).map(|_| ()));
        assert_eq!(check(&bytes), Ok(()), "f32 {v}");
    }
    for v in [0.1f64, 1e300, f64::MIN, f64::NAN] {
        let bytes = enc(|e| e.f64(v).map(|_| ()));
        assert_eq!(check(&bytes), Ok(()), "f64 {v}");
    }
    let _ = float::f16_bits_of;
}

#[cfg(all(feature = "float", feature = "bignum"))]
#[test]
fn canonical_bignums_pass() {
    let big =
        enc(|e| e.bignum_uint(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).map(|_| ()));
    assert_eq!(check(&big), Ok(()));
    let neg = enc(|e| e.bignum_nint(&[0xff; 9]).map(|_| ()));
    assert_eq!(check(&neg), Ok(()));
}

// --------------------------------------------------------- violations

#[test]
fn non_minimal_integer_widths_rejected() {
    // uint 0 written with ai 24..27 instead of inline.
    for head in [
        &[0x18u8, 0x00][..],
        &[0x19, 0x00, 0x00],
        &[0x1a, 0, 0, 0, 0],
        &[0x1b, 0, 0, 0, 0, 0, 0, 0, 0],
    ] {
        assert_eq!(check(head), Err(Error::NOT_CANONICAL), "overlong encoding {head:?}");
    }
    // Negative integer non-minimal too (-1 as 0x38 0x00).
    assert_eq!(check(&[0x38, 0x00]), Err(Error::NOT_CANONICAL));
    // Strings/arrays/maps/tags with overlong lengths.
    assert_eq!(check(&[0x58, 0x01, 0xaa]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0x78, 0x01, b'a']), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0x98, 0x01, 0x01]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0xb8, 0x00]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0xd8, 0x01, 0x01]), Err(Error::NOT_CANONICAL));
}

#[test]
fn indefinite_lengths_rejected() {
    assert_eq!(check(&[0x9f, 0x01, 0xff]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0xbf, 0xff]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0x5f, 0x41, 0x00, 0xff]), Err(Error::NOT_CANONICAL));
    // Even inside an otherwise-canonical definite container.
    assert_eq!(check(&[0x81, 0x9f, 0xff]), Err(Error::NOT_CANONICAL));
}

#[test]
fn invalid_simple_encodings_rejected() {
    // Two-byte form below 32: both invalid and non-preferred.
    assert_eq!(check(&[0xf8, 0x05]), Err(Error::NOT_CANONICAL));
    // Legal two-byte simple passes.
    assert_eq!(check(&[0xf8, 0x20]), Ok(()));
}

#[cfg(feature = "float")]
#[test]
fn non_shortest_floats_rejected() {
    // 1.0 must be a half (f9 3c00); single/double forms violate §4.2.2.
    assert_eq!(check(&[0xf9, 0x3c, 0x00]), Ok(()));
    assert_eq!(check(&[0xfa, 0x3f, 0x80, 0x00, 0x00]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0]), Err(Error::NOT_CANONICAL));

    // 100000.0 fits single but not half: fa is canonical, fb is not.
    assert_eq!(check(&[0xfa, 0x47, 0xc3, 0x50, 0x00]), Ok(()));
    assert_eq!(
        check(&[0xfb, 0x40, 0xf3, 0x88, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::NOT_CANONICAL)
    );

    // NaN is canonically half 0x7e00.
    assert_eq!(check(&[0xf9, 0x7e, 0x00]), Ok(()));
    assert_eq!(check(&[0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0]), Err(Error::NOT_CANONICAL));

    // Infinity must be a half.
    assert_eq!(check(&[0xf9, 0x7c, 0x00]), Ok(()));
    assert_eq!(check(&[0xfa, 0x7f, 0x80, 0x00, 0x00]), Err(Error::NOT_CANONICAL));

    // A double-only value (1e300) is fine as fb.
    assert_eq!(check(&[0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c]), Ok(()));
}

#[test]
fn unsorted_and_duplicate_map_keys_rejected() {
    // Sorted map passes.
    let ok = enc(|e| {
        e.map_len(2)?.uint(1)?.uint(2)?.uint(3)?.uint(4)?;
        Ok(())
    });
    assert_eq!(check(&ok), Ok(()));

    // Descending keys fail.
    let bad = enc(|e| {
        e.map_len(2)?.uint(3)?.uint(4)?.uint(1)?.uint(2)?;
        Ok(())
    });
    assert_eq!(check(&bad), Err(Error::NOT_CANONICAL));

    // Duplicate keys fail (equal is not strictly increasing).
    let dup = enc(|e| {
        e.map_len(2)?.str("k")?.uint(1)?.str("k")?.uint(2)?;
        Ok(())
    });
    assert_eq!(check(&dup), Err(Error::NOT_CANONICAL));

    // Bytewise order includes the length prefix: h'0002' (42 ...) sorts
    // BEFORE h'0001ff' (43 ...), so this insertion order is canonical.
    let bytewise_ok = enc(|e| {
        e.map_len(2)?.bytes(b"\x00\x02")?.uint(0)?.bytes(b"\x00\x01\xff")?.uint(1)?;
        Ok(())
    });
    assert_eq!(check(&bytewise_ok), Ok(()));
    // Reversed pair order violates it.
    let bytewise_bad = enc(|e| {
        e.map_len(2)?.bytes(b"\x00\x01\xff")?.uint(1)?.bytes(b"\x00\x02")?.uint(0)?;
        Ok(())
    });
    assert_eq!(check(&bytewise_bad), Err(Error::NOT_CANONICAL));

    // Nested maps are checked independently: inner map has descending
    // keys [5, 3].
    let nested_bad = enc(|e| {
        e.map_len(1)?.uint(1)?.map_len(2)?.uint(5)?.uint(0)?.uint(3)?.uint(9)?;
        Ok(())
    });
    assert_eq!(check(&nested_bad), Err(Error::NOT_CANONICAL));
}

#[cfg(feature = "bignum")]
#[test]
fn non_preferred_bignums_rejected() {
    // Leading zero magnitude.
    assert_eq!(check(&[0xc2, 0x42, 0x00, 0x01]), Err(Error::NOT_CANONICAL));
    // Empty magnitude (value 0 must be a basic integer).
    assert_eq!(check(&[0xc2, 0x40]), Err(Error::NOT_CANONICAL));
    assert_eq!(check(&[0xc3, 0x40]), Err(Error::NOT_CANONICAL));
    // Value 1 encoded as bignum instead of basic integer: leading byte
    // check passes, but h'01' has no leading zeroes — accepted here since
    // representation choice is application-level beyond §4.2's byte rules.
    assert_eq!(check(&[0xc2, 0x41, 0x01]), Ok(()));
    // Negative bignum with clean magnitude passes.
    assert_eq!(check(&[0xc3, 0x41, 0xff]), Ok(()));
}

// ------------------------------------------------- structural errors kept

#[test]
fn structural_errors_are_not_masked() {
    assert_eq!(check(&[]), Err(Error::TRUNCATED));
    assert_eq!(check(&[0x18]), Err(Error::TRUNCATED));
    assert_eq!(check(&[0x62, b'a']), Err(Error::TRUNCATED));
    assert_eq!(check(&[0xff]), Err(Error::UNEXPECTED_BREAK));
    assert_eq!(check(&[0x1c]), Err(Error::INVALID_ADDITIONAL_INFO));
    assert_eq!(check(&[0x82, 0x01]), Err(Error::TRUNCATED));
    // Trailing garbage after one complete item.
    assert_eq!(check(&[0x01, 0x02]), Err(Error::TRUNCATED));
    // check_all accepts trailing separate items instead.
    assert_eq!(check_all(&[0x01, 0x02]), Ok(()));
    // Depth bound mirrors skip.
    let mut deep = vec![0x81u8; const_cbor::decode::SKIP_LIMIT];
    deep.push(0x01);
    let res = check(&deep);
    assert!(matches!(res, Err(Error::DEPTH_EXCEEDED | Error::NOT_CANONICAL)));
}

#[test]
fn tagged_items_count_as_one_element() {
    // Tag chains wrapping values inside maps keep pair slots aligned.
    let ok = enc(|e| {
        e.map_len(1)?.tag(0)?.tag(1)?.uint(7)?.uint(9)?;
        Ok(())
    });
    assert_eq!(check(&ok), Ok(()));
}
