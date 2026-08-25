//! Canonical map building edge cases (RFC 8949 §4.2).
//!
//! Note: ordering is bytewise over *encoded* keys, so a longer key with an
//! equal-length-prefixed payload can sort after a shorter key whose header
//! byte is smaller.
#![cfg(feature = "canonical")]

use const_cbor::canonical::MapBuilder;
use const_cbor::{Decoder, Encoder, Error};

#[test]
fn already_sorted_takes_fast_path_without_scratch() {
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.map_len(3).unwrap();
    let mut m = MapBuilder::<3>::new(&mut e);
    m.add(|k| k.u8(1), |v| v.u8(10)).unwrap();
    m.add(|k| k.u8(2), |v| v.u8(20)).unwrap();
    m.add(|k| k.str("aa"), |v| v.u8(30)).unwrap();
    // Empty scratch is fine when no reorder is needed.
    m.finish(&mut []).unwrap();
    let (buf, n) = e.finish();
    // 01 0a 02 14 61 61 1e
    // u8(30) needs the two-byte preferred form (30 > 23).
    assert_eq!(&buf[..n], &[0xa3, 0x01, 0x0a, 0x02, 0x14, 0x62, b'a', b'a', 0x18, 0x1e]);
}

#[test]
fn bytewise_order_length_prefix_participates() {
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.map_len(2).unwrap();
    let mut m = MapBuilder::<2>::new(&mut e);
    // "zeta" (5-byte encoding) sorts before "alpha" (6-byte encoding).
    m.add(|k| k.str("zeta"), |v| v.u8(1)).unwrap();
    m.add(|k| k.str("alpha"), |v| v.u8(2)).unwrap();
    m.finish(&mut [0u8; 16]).unwrap();
    let (buf, n) = e.finish();
    let mut d = Decoder::new(&buf[..n]);
    assert_eq!(d.map_len(), Ok(2));
    assert_eq!(d.str(), Ok("zeta"));
    assert_eq!(d.u8(), Ok(1));
    assert_eq!(d.str(), Ok("alpha"));
    assert_eq!(d.u8(), Ok(2));
}

#[test]
fn empty_map_finishes_trivially() {
    let mut buf = [0u8; 8];
    let mut e = Encoder::new(&mut buf);
    e.map_len(0).unwrap();
    let m = MapBuilder::<4>::new(&mut e);
    m.finish(&mut [0u8; 32]).unwrap();
    let (buf, n) = e.finish();
    assert_eq!(&buf[..n], &[0xa0]);
}

#[test]
fn capacity_exhaustion_is_depth_exceeded() {
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.map_len(2).unwrap();
    let mut m: MapBuilder<2> = MapBuilder::new(&mut e);
    m.add(|k| k.u8(1), |v| v.u8(1)).unwrap();
    m.add(|k| k.u8(2), |v| v.u8(2)).unwrap();
    assert!(matches!(m.add(|k| k.u8(3), |v| v.u8(3)), Err(Error::DEPTH_EXCEEDED)));
    // The two valid pairs still finish canonically.
    m.finish(&mut [0u8; 16]).unwrap();
    let (buf, n) = e.finish();
    assert_eq!(&buf[..n], &[0xa2, 0x01, 0x01, 0x02, 0x02]);
}

#[test]
fn scratch_too_small_is_out_of_space_and_no_corruption() {
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);
    e.map_len(2).unwrap();
    let mut m = MapBuilder::<2>::new(&mut e);
    m.add(|k| k.str("bb"), |v| v.u64(1000)).unwrap(); // 4+... bytes
    m.add(|k| k.str("aa"), |v| v.u64(2000)).unwrap();
    // Region larger than the 1-byte scratch.
    assert!(matches!(m.finish(&mut [0u8; 1]), Err(Error::OUT_OF_SPACE)));
    // The buffer still holds insertion-order pairs; verify by decoding.
    let written = e.written();
    let mut d = Decoder::new(&buf[..written]);
    assert_eq!(d.map_len(), Ok(2));
    assert_eq!(d.str(), Ok("bb"));
    assert_eq!(d.u64(), Ok(1000));
}

#[test]
fn keys_differing_after_shared_prefix_sort_bytewise() {
    let mut buf = [0u8; 96];
    let mut e = Encoder::new(&mut buf);
    e.map_len(3).unwrap();
    let mut m = MapBuilder::<3>::new(&mut e);
    m.add(|k| k.bytes(b"\x00\x02"), |v| v.u8(2)).unwrap();
    m.add(|k| k.bytes(b"\x00\x01"), |v| v.u8(1)).unwrap();
    m.add(|k| k.bytes(b"\x00\x01\xff"), |v| v.u8(3)).unwrap();
    m.finish(&mut [0u8; 24]).unwrap();
    let (buf, n) = e.finish();
    // Canonical order is bytewise over ENCODED keys: the length prefix
    // participates, so h'0002' (42 ...) sorts before h'0001ff' (43 ...).
    let mut d = Decoder::new(&buf[..n]);
    assert_eq!(d.map_len(), Ok(3));
    assert_eq!(d.bytes(), Ok(&b"\x00\x01"[..]));
    assert_eq!(d.u8(), Ok(1));
    assert_eq!(d.bytes(), Ok(&b"\x00\x02"[..]));
    assert_eq!(d.u8(), Ok(2));
    assert_eq!(d.bytes(), Ok(&b"\x00\x01\xff"[..]));
    assert_eq!(d.u8(), Ok(3));
}

#[test]
fn single_pair_map_never_reorders() {
    let mut buf = [0u8; 16];
    let mut e = Encoder::new(&mut buf);
    e.map_len(1).unwrap();
    let mut m = MapBuilder::<1>::new(&mut e);
    m.add(|k| k.u8(9), |v| v.str("v")).unwrap();
    m.finish(&mut []).unwrap(); // sorted fast path, empty scratch ok
    let (buf, n) = e.finish();
    assert_eq!(&buf[..n], &[0xa1, 0x09, 0x61, b'v']);
}
