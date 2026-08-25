//! `RawValue` zero-copy handle coverage.
#![cfg(feature = "raw")]

use const_cbor::{Decoder, Encoder, Error};

fn encode_one(f: impl FnOnce(&mut Encoder) -> Result<(), Error>) -> Vec<u8> {
    let mut buf = [0u8; 128];
    let mut e = Encoder::new(&mut buf);
    f(&mut e).unwrap();
    let (buf, n) = e.finish();
    buf[..n].to_vec()
}

#[test]
fn raw_value_roundtrip_all_majors() {
    let items: Vec<Vec<u8>> = vec![
        encode_one(|e| e.u8(1).map(|_| ())),
        encode_one(|e| e.i64(-1000).map(|_| ())),
        encode_one(|e| e.bytes(b"xyz").map(|_| ())),
        encode_one(|e| e.str("hello").map(|_| ())),
        encode_one(|e| e.array_len(2).and_then(|e| e.u8(1)).and_then(|e| e.str("a")).map(|_| ())),
        encode_one(|e| e.map_len(1).and_then(|e| e.u8(1)).and_then(|e| e.bool(true)).map(|_| ())),
        encode_one(|e| e.tag(1).and_then(|e| e.u32(2)).map(|_| ())),
        encode_one(|e| e.null().map(|_| ())),
        encode_one(|e| e.simple(const_cbor::Simple(99)).map(|_| ())),
        // Indefinite item captured whole.
        encode_one(|e| e.bytes_indef(&[b"a", b"bc"]).map(|_| ())),
    ];
    for bytes in &items {
        let mut dec = Decoder::new(bytes);
        let raw = dec.raw_value().unwrap();
        assert_eq!(raw.as_bytes(), &bytes[..]);
        assert_eq!(raw.len(), bytes.len());
        assert!(!raw.is_empty());
        assert_eq!(&raw[..], &bytes[..]); // Deref
        assert!(dec.is_empty());
    }
}

#[test]
fn raw_lazy_redecode() {
    let inner =
        encode_one(|e| e.map_len(1).and_then(|e| e.str("k")).and_then(|e| e.u64(42)).map(|_| ()));
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.tag(0).unwrap();
    // Manually append inner bytes via a second encoder pass is awkward;
    // instead decode from a composite buffer.
    let mut composite = vec![0xc0u8]; // tag(0)
    composite.extend_from_slice(&inner);
    let mut dec = Decoder::new(&composite);
    assert_eq!(dec.tag(), Ok(0));
    let raw = dec.raw_value().unwrap();
    assert_eq!(raw.as_bytes(), &inner[..]);

    // Lazy typed read through the handle.
    let mut rd = raw.decoder();
    assert_eq!(rd.map_len(), Ok(1));
    assert_eq!(rd.str(), Ok("k"));
    assert_eq!(rd.u64(), Ok(42));
}

#[test]
fn raw_verbatim_preserves_non_preferred_encoding() {
    // Non-preferred uint 0 (0x18 0x00) must be copied byte-exact.
    let input = [0x18u8, 0x00];
    let mut dec = Decoder::new(&input);
    let raw = dec.raw_value().unwrap();
    assert_eq!(raw.as_bytes(), &[0x18, 0x00]);

    let mut out = [0u8; 8];
    let mut e = Encoder::new(&mut out);
    e.raw(&raw).unwrap();
    let (out, n) = e.finish();
    assert_eq!(&out[..n], &[0x18, 0x00]);
}

#[test]
fn raw_out_of_space_and_rollback() {
    // uint 1 (one byte) followed by a two-byte byte string.
    let input = [0x01, 0x41, 0x00];
    let mut dec = Decoder::new(&input);
    dec.raw_value().unwrap(); // consume the integer
    let raw = dec.raw_value().unwrap();
    assert_eq!(raw.len(), 2);

    let mut out = [0u8; 1];
    let mut e = Encoder::new(&mut out);
    assert!(matches!(e.raw(&raw), Err(Error::OUT_OF_SPACE)));
    assert_eq!(e.written(), 0);

    // Exactly-fitting write succeeds.
    let mut out = [0u8; 2];
    let mut e = Encoder::new(&mut out);
    e.raw(&raw).unwrap();
    assert_eq!(e.written(), 2);
}

#[test]
fn raw_multiple_items_sequentially() {
    let mut stream = Vec::new();
    for v in 0..5u8 {
        let item = encode_one(|e| {
            let e = e.array_len(usize::from(v))?;
            for _ in 0..v {
                e.u8(7)?;
            }
            Ok(())
        });
        stream.extend_from_slice(&item);
    }
    let mut dec = Decoder::new(&stream);
    for v in 0..5u8 {
        let raw = dec.raw_value().unwrap();
        let mut rd = raw.decoder();
        assert_eq!(rd.array_len(), Ok(usize::from(v)));
    }
    assert!(dec.is_empty());
}

#[test]
fn raw_of_truncated_fails() {
    let mut dec = Decoder::new(&[0x82, 0x01]); // array(2), one item
    assert_eq!(dec.raw_value(), Err(Error::TRUNCATED));
}

#[test]
fn raw_debug_clone_eq() {
    let input = [0x01];
    let mut dec = Decoder::new(&input);
    let raw = dec.raw_value().unwrap();
    let copy = raw;
    assert_eq!(raw, copy);
    assert_ne!(format!("{raw:?}"), String::new(), "Debug formatting must be total");
}
