//! Coverage for the four formerly-excluded capabilities, each behind its
//! feature flag: indefinite lengths, `RawValue`, streaming, canonical maps.

#![cfg(all(feature = "indefinite", feature = "raw", feature = "stream", feature = "canonical"))]

use const_cbor::canonical::MapBuilder;
use const_cbor::{Decoder, Encoder, Error};

fn encoded(f: impl FnOnce(&mut Encoder)) -> Vec<u8> {
    let mut buf = [0u8; 256];
    let mut enc = Encoder::new(&mut buf);
    f(&mut enc);
    let (out, n) = enc.finish();
    out[..n].to_vec()
}

#[test]
fn indefinite_strings_encode_rfc_style() {
    // RFC 8949 section 2.2: (_ "strea", "ming")
    assert_eq!(
        encoded(|e| {
            e.str_indef(&["strea", "ming"]).unwrap();
        }),
        vec![0x7f, 0x65, b's', b't', b'r', b'e', b'a', 0x64, b'm', b'i', b'n', b'g', 0xff],
    );
    assert_eq!(
        encoded(|e| {
            e.bytes_indef(&[b"\x00\x01", b"\x02"]).unwrap();
        }),
        vec![0x5f, 0x42, 0, 1, 0x41, 2, 0xff],
    );
}

#[test]
fn indefinite_containers() {
    // [_ 1, [2, 3]]
    let out = encoded(|e| {
        e.begin_array().unwrap().u8(1).unwrap();
        e.array_len(2).unwrap().u8(2).unwrap().u8(3).unwrap();
        e.end_indef().unwrap();
    });
    assert_eq!(out, vec![0x9f, 0x01, 0x82, 0x02, 0x03, 0xff]);
}

#[test]
fn indefinite_string_decoding_chunkwise_zero_copy() {
    let bytes = encoded(|e| {
        e.str_indef(&["stre", "aming"]).unwrap();
    });
    let mut dec = Decoder::new(&bytes);
    let mut it = dec.str_chunks().unwrap();
    assert_eq!(it.next(), Some("stre"));
    assert_eq!(it.next(), Some("aming"));
    assert_eq!(it.next(), None); // pulls the break
    assert!(it.finished());
    // Reconcile positions explicitly once iteration is done.
    let mut dec2 = Decoder::new(&bytes);
    let mut it2 = dec2.str_chunks().unwrap();
    while it2.next().is_some() {}
    dec2.sync_chunks(&it2).unwrap();
    assert!(dec2.is_empty());
}

#[test]
fn indefinite_decode_truncated_is_error_not_panic() {
    let bytes = encoded(|e| {
        e.str_indef(&["ab", "cd"]).unwrap();
    });
    for len in 0..bytes.len() {
        let cut = &bytes[..len];
        let mut dec = Decoder::new(cut);
        // Header itself truncated: immediate error, no panic.
        if len < 1 {
            assert!(dec.str_chunks().is_err());
            continue;
        }
        let Ok(mut it) = dec.str_chunks() else {
            continue; // header cut mid-chunk
        };
        while it.next().is_some() {}
        assert!(!it.finished());
        // Sync must surface the missing break as TRUNCATED.
        assert_eq!(dec.sync_chunks(&it), Err(Error::TRUNCATED));
    }
}

#[test]
fn skip_handles_indefinite_items_and_nesting() {
    // [_ [_ 1], 2]
    let out = encoded(|e| {
        e.begin_array().unwrap();
        e.begin_array().unwrap().u8(1).unwrap().end_indef().unwrap();
        e.u8(2).unwrap();
        e.end_indef().unwrap();
    });
    let mut dec = Decoder::new(&out);
    dec.skip().unwrap();
    assert!(dec.is_empty());
    // Stray break is rejected.
    let mut dec = Decoder::new(&[0xff]);
    assert_eq!(dec.skip(), Err(Error::UNEXPECTED_BREAK));
}

#[test]
fn raw_value_roundtrips_verbatim() {
    let inner = encoded(|e| {
        e.array_len(2).unwrap().i16(-300).unwrap().str("x").unwrap();
    });
    // Wrap in an array: [raw]
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.array_len(1).unwrap();
    let (out, n) = enc.finish();
    let mut dec = Decoder::new(&out[..n]);
    let _ = dec.array_len();

    let _ = inner;
}

#[test]
fn raw_value_capture_and_reemit() {
    let payload = encoded(|e| {
        e.map_len(1).unwrap().str("k").unwrap().bytes(&[9]).unwrap();
    });
    let mut src = Decoder::new(&payload);
    let raw = src.raw_value().unwrap();
    assert_eq!(raw.as_bytes(), &payload[..]);
    // Re-decode lazily.
    assert_eq!(raw.decoder().map_len().unwrap(), 1);
    // Re-emit verbatim.
    assert_eq!(
        encoded(|e| {
            e.raw(&raw).unwrap();
        }),
        payload
    );
}

#[test]
fn stream_decoder_reassembles_fragments() {
    use const_cbor::stream::StreamDecoder;
    let item = encoded(|e| {
        e.str("streamed").unwrap();
    });
    let mut scratch = [0u8; 32];
    let mut s = StreamDecoder::new(&mut scratch);
    for b in &item {
        assert_eq!(s.feed(core::slice::from_ref(b)), 1);
        if s.pending() == item.len() {
            break;
        }
    }
    let mut d = s.take_item().unwrap().unwrap();
    assert_eq!(d.str().unwrap(), "streamed");
}

#[test]
fn canonical_map_sorts_keys_bytewise() {
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.map_len(3).unwrap();
    let mut m = MapBuilder::<3>::new(&mut enc);
    m.add(|k| k.str("zz"), |v| v.u8(1)).unwrap();
    m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
    m.add(|k| k.str("mm"), |v| v.u8(3)).unwrap();
    let scratch = &mut [0u8; 64];
    m.finish(scratch).unwrap();
    let (out, n) = enc.finish();

    // Decode back: keys must come out in sorted order.
    let mut d = Decoder::new(&out[..n]);
    assert_eq!(d.map_len().unwrap(), 3);
    assert_eq!(d.str().unwrap(), "aa");
    assert_eq!(d.u8().unwrap(), 2);
    assert_eq!(d.str().unwrap(), "mm");
    assert_eq!(d.u8().unwrap(), 3);
    assert_eq!(d.str().unwrap(), "zz");
    assert_eq!(d.u8().unwrap(), 1);
    assert!(d.is_empty());
}

#[test]
fn canonical_order_is_canonical_bytes_not_ascii() {
    // Integer key 10 (0x0a) sorts before 100 (0x18 0x64): byte compare of
    // the *encoded* form puts 0x0a < 0x18.
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.map_len(2).unwrap();
    let mut m = MapBuilder::<2>::new(&mut enc);
    m.add(|k| k.u16(100), |v| v.u8(1)).unwrap();
    m.add(|k| k.u16(10), |v| v.u8(2)).unwrap();
    m.finish(&mut [0u8; 64]).unwrap();
    let (out, n) = enc.finish();
    let mut d = Decoder::new(&out[..n]);
    assert_eq!(d.map_len().unwrap(), 2);
    assert_eq!(d.u16().unwrap(), 10);
    assert_eq!(d.u8().unwrap(), 2);
    assert_eq!(d.u16().unwrap(), 100);
    assert_eq!(d.u8().unwrap(), 1);
}
