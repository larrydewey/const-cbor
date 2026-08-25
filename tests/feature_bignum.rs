//! Bignum coverage per RFC 8949 §3.4.3 (feature `bignum`).
//!
//! SOP checklist: RFC vectors, preferred-serialization normalization,
//! leading-zero acceptance (MUST), typed range checks, every error
//! variant, failure atomicity, truncation prefixes, roundtrips, and
//! interactions with raw/skip/diagnostic.
#![cfg(feature = "bignum")]

use const_cbor::bignum::{TAG_NEGATIVE, TAG_UNSIGNED, normalized};
use const_cbor::{Decoder, Encoder, Error};

fn encode(f: impl FnOnce(&mut Encoder) -> Result<(), Error>) -> Vec<u8> {
    let mut buf = [0u8; 128];
    let mut e = Encoder::new(&mut buf);
    f(&mut e).unwrap();
    let (buf, n) = e.finish();
    buf[..n].to_vec()
}

// ------------------------------------------------------------- RFC vectors

#[test]
fn rfc_appendix_a_bignum_vectors() {
    // 18446744073709551616 (2^64): C2 49 010000000000000000
    assert_eq!(
        encode(|e| e
            .bignum_uint(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .map(|_| ())),
        vec![0xc2, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
    // -18446744073709551617 (-1 - 2^64): C3 49 010000000000000000
    assert_eq!(
        encode(|e| e.bignum_nint(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0]).map(|_| ())),
        vec![0xc3, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
    // Decoding both vectors.
    let mut d = Decoder::new(&[0xc2, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let mag = d.bignum_unsigned().unwrap();
    assert_eq!(mag, &[0x01, 0, 0, 0, 0, 0, 0, 0, 0]);

    let mut d = Decoder::new(&[0xc3, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let mag = d.bignum_nint().unwrap();
    assert_eq!(mag.len(), 9);
}

#[test]
fn zero_encodes_as_empty_byte_string() {
    // Preferred serialization of n = 0 is the empty byte string.
    assert_eq!(encode(|e| e.bignum_uint(&[0, 0, 0]).map(|_| ())), vec![0xc2, 0x40]);
    assert_eq!(encode(|e| e.bignum_nint(&[]).map(|_| ())), vec![0xc3, 0x40]);
    // And it decodes to zero / -1 respectively.
    let mut d = Decoder::new(&[0xc2, 0x40]);
    assert_eq!(d.bignum_u64(), Ok(0));
    let mut d = Decoder::new(&[0xc2, 0x40]);
    assert_eq!(d.bignum_i128(), Ok(0));
    let mut d = Decoder::new(&[0xc3, 0x40]);
    assert_eq!(d.bignum_i128(), Ok(-1));
}

#[test]
fn leading_zeroes_accepted_on_decode() {
    // MUST-decode case: tag 2 wrapping h'0001' equals 1.
    let input = [0xc2u8, 0x42, 0x00, 0x01];
    let mut d = Decoder::new(&input);
    assert_eq!(d.bignum_u64(), Ok(1));
    let mut d = Decoder::new(&input);
    assert_eq!(d.bignum_unsigned().unwrap(), &[0x00, 0x01]);
    // normalized() strips them.
    assert_eq!(normalized(&[0x00, 0x00, 0x01]), &[0x01]);
    assert_eq!(normalized(&[0x00; 7]), &[]);
}

#[test]
fn boundary_values() {
    // u64::MAX as bignum decodes through bignum_u64.
    let bytes = encode(|e| e.bignum_uint(&[0xff; 8]).map(|_| ()));
    let mut d = Decoder::new(&bytes);
    assert_eq!(d.bignum_u64(), Ok(u64::MAX));

    // One byte wider overflows u64.
    let bytes = encode(|e| {
        e.bignum_uint(&[0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]).map(|_| ())
    });
    let mut d = Decoder::new(&bytes);
    assert_eq!(d.bignum_u64(), Err(Error::INTEGER_OUT_OF_RANGE));

    // i128 boundaries. i128::MIN = -1 - (2^127 - 1): magnitude 0x7fff...
    let i128_min_mag = [
        0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff,
    ];
    let neg_bytes = encode(|e| e.bignum_nint(&i128_min_mag).map(|_| ()));
    let mut d = Decoder::new(&neg_bytes);
    assert_eq!(d.bignum_i128(), Ok(i128::MIN));

    // -1 - (2^127) is one below i128::MIN: out of range.
    let too_small = encode(|e| {
        e.bignum_nint(&[0x81, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).map(|_| ())
    });
    let mut d = Decoder::new(&too_small);
    assert_eq!(d.bignum_i128(), Err(Error::INTEGER_OUT_OF_RANGE));

    // i128::MAX fits unsigned.
    let max_bytes = encode(|e| {
        e.bignum_uint(&[
            0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff,
        ])
        .map(|_| ())
    });
    let mut d = Decoder::new(&max_bytes);
    assert_eq!(d.bignum_i128(), Ok(i128::MAX));
}

// ------------------------------------------------------------ error paths

#[test]
fn type_mismatches() {
    // Tag 3 read by the unsigned getter.
    let mut d = Decoder::new(&[0xc3, 0x41, 0x05]);
    assert_eq!(d.bignum_unsigned(), Err(Error::TYPE_MISMATCH));
    // Tag 2 read by the negative getter.
    let mut d = Decoder::new(&[0xc2, 0x41, 0x05]);
    assert_eq!(d.bignum_nint(), Err(Error::TYPE_MISMATCH));
    // Wrong tag number entirely (tag 23).
    let mut d = Decoder::new(&[0xd7, 0x41, 0x05]);
    assert_eq!(d.bignum_unsigned(), Err(Error::TYPE_MISMATCH));
    // Bignum getter on a basic integer.
    let mut d = Decoder::new(&[0x01]);
    assert_eq!(d.bignum_u64(), Err(Error::TYPE_MISMATCH));
    // Tag 2 wrapping something other than a byte string is invalid.
    let mut d = Decoder::new(&[0xc2, 0x01]);
    assert_eq!(d.bignum_unsigned(), Err(Error::TYPE_MISMATCH));
    let mut d = Decoder::new(&[0xc2, 0x61, b'x']);
    assert_eq!(d.bignum_i128(), Err(Error::TYPE_MISMATCH));
}

#[test]
fn truncated_prefixes_never_panic() {
    let full = [0xc2u8, 0x43, 0x01, 0x02, 0x03, 0xc3, 0x41, 0xff];
    for cut in 0..full.len() {
        let mut d = Decoder::new(&full[..cut]);
        let _ = d.bignum_unsigned();
        let _ = d.bignum_u64();
        let _ = d.bignum_i128();
        let _ = d.bignum_nint();
    }
    // Truncated payload yields TRUNCATED, not a partial value.
    let mut d = Decoder::new(&full[..4]);
    assert_eq!(d.bignum_unsigned(), Err(Error::TRUNCATED));
    // Missing content entirely.
    let mut d = Decoder::new(&[0xc2]);
    assert_eq!(d.bignum_unsigned(), Err(Error::TRUNCATED));
}

#[test]
fn encoder_out_of_space_is_atomic() {
    // Not even room for the tag head.
    let mut tiny = [0u8; 0];
    let mut e = Encoder::new(&mut tiny);
    assert!(matches!(e.bignum_uint(&[1]), Err(Error::OUT_OF_SPACE)));

    // Room for the tag but not the payload: cursor must roll back to zero.
    let mut tiny = [0u8; 1];
    let mut e = Encoder::new(&mut tiny);
    assert!(matches!(e.bignum_uint(&[1, 2, 3]), Err(Error::OUT_OF_SPACE)));
    assert_eq!(e.written(), 0);

    // Exactly enough space succeeds.
    let mut buf = [0u8; 5]; // c2 43 xx xx xx
    let mut e = Encoder::new(&mut buf);
    e.bignum_uint(&[9, 9, 9]).unwrap();
    assert_eq!(&buf, &[0xc2, 0x43, 9, 9, 9]);
}

// -------------------------------------------------------------- roundtrips

#[test]
fn magnitude_roundtrips() {
    let magnitudes: [&[u8]; 6] =
        [&[], &[0x01], &[0xff], &[0x01, 0x00], &[0xde, 0xad, 0xbe, 0xef], &[0xff; 32]];
    for mag in magnitudes {
        for sign in [false, true] {
            let bytes = encode(|e| {
                if sign { e.bignum_nint(mag).map(|_| ()) } else { e.bignum_uint(mag).map(|_| ()) }
            });
            let mut d = Decoder::new(&bytes);
            let got = if sign { d.bignum_nint().unwrap() } else { d.bignum_unsigned().unwrap() };
            assert_eq!(got, normalized(mag), "magnitude {mag:?} sign {sign}");
        }
    }
}

#[test]
fn i128_roundtrip_across_signs() {
    for v in [
        0i128,
        1,
        -1,
        -255,
        255,
        i128::from(i64::MAX) + 1,
        i128::from(i64::MIN) - 1,
        i128::MAX,
        i128::MIN,
        i128::from(0xdead_beef_cafe_f00d_u64) * 998_877_665,
    ] {
        let bytes = encode(|e| {
            if v >= 0 {
                e.bignum_uint(&v.unsigned_abs().to_be_bytes()).map(|_| ())
            } else {
                // magnitude of a negative bignum is (-v - 1)
                let m = (!(v as u128)).to_be_bytes();
                e.bignum_nint(&m).map(|_| ())
            }
        });
        let mut d = Decoder::new(&bytes);
        assert_eq!(d.bignum_i128(), Ok(v), "roundtrip {v}");
    }
}

// ------------------------------------------------- feature interactions

#[test]
fn skip_and_raw_traverse_whole_bignum() {
    use const_cbor::Major;
    let bytes = encode(|e| e.bignum_uint(&[1, 2, 3, 4]).map(|_| ()));
    let mut d = Decoder::new(&bytes);
    d.skip().unwrap();
    assert!(d.is_empty());

    #[cfg(feature = "raw")]
    {
        let mut d = Decoder::new(&bytes);
        let raw = d.raw_value().unwrap();
        assert_eq!(raw.as_bytes(), &bytes[..]);
    }

    // peek_major reports Tag.
    let mut d = Decoder::new(&bytes);
    assert_eq!(d.peek_major(), Some(Major::Tag));
    assert_eq!(d.tag(), Ok(TAG_UNSIGNED));
    assert_eq!(TAG_UNSIGNED, 2);
    assert_eq!(TAG_NEGATIVE, 3);
}

#[cfg(feature = "stream")]
#[test]
fn stream_delivers_complete_bignums() {
    use const_cbor::stream::StreamDecoder;
    let item = encode(|e| e.bignum_nint(&[0xab; 12]).map(|_| ()));
    let mut stream = item.clone();
    stream.extend_from_slice(&encode(|e| e.u8(1).map(|_| ())));

    for cut in 0..stream.len() {
        let mut scratch = [0u8; 64];
        let mut s = StreamDecoder::new(&mut scratch);
        assert_eq!(s.feed(&stream[..cut]), cut);
        assert_eq!(s.feed(&stream[cut..]), stream.len() - cut);
        let mut d = s.take_item().unwrap().unwrap();
        assert!(d.bignum_i128().is_ok());
        let mut d = s.take_item().unwrap().unwrap();
        assert_eq!(d.u8(), Ok(1));
        assert!(s.take_item().unwrap().is_none());
    }
}

#[cfg(feature = "diagnostic")]
#[test]
fn diagnostic_renders_generic_tag_notation() {
    use const_cbor::diagnostic::render;
    let mut out = [0u8; 64];
    // Generic tag rendering: 2(h'010203').
    let n = render(&[0xc2, 0x43, 1, 2, 3], &mut out).unwrap();
    assert_eq!(n, "2(h'010203')");
}
