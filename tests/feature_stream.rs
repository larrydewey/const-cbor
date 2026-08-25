//! Incremental decoding: every split boundary of representative streams,
//! plus error propagation across feeds.
#![cfg(feature = "stream")]

use const_cbor::Error;
use const_cbor::stream::StreamDecoder;

/// A small representative multi-item stream:
/// 1, -1, "hi", h'fe', [_ 1], [1, 2], {1: 2}, tag-wrapped null.
const STREAM: &[u8] = &[
    0x01, // 1
    0x20, // -1
    0x62, b'h', b'i', // "hi"
    0x41, 0xfe, // h'fe'
    0x9f, 0x01, 0xff, // [_ 1]
    0x82, 0x01, 0x02, // [1, 2]
    0xa1, 0x01, 0x02, // {1: 2}
    0xc0, 0xf6, // tag(0, null)
];

const ITEM_COUNT: usize = 8;

fn feed_whole(scratch: &mut [u8]) -> usize {
    let mut s = StreamDecoder::new(scratch);
    assert_eq!(s.feed(STREAM), STREAM.len());
    let mut count = 0;
    while let Some(mut d) = s.take_item().unwrap() {
        d.skip().unwrap();
        count += 1;
    }
    count
}

#[test]
fn whole_stream_at_once() {
    let mut scratch = [0u8; 64];
    assert_eq!(feed_whole(&mut scratch), ITEM_COUNT);
}

#[test]
fn every_single_byte_split_boundary() {
    for cut in 0..=STREAM.len() {
        let mut scratch = [0u8; 64];
        let mut s = StreamDecoder::new(&mut scratch);
        assert_eq!(s.feed(&STREAM[..cut]), cut);
        assert_eq!(s.feed(&STREAM[cut..]), STREAM.len() - cut);
        let mut count = 0;
        while let Some(mut d) = s.take_item().unwrap() {
            d.skip().unwrap();
            count += 1;
        }
        assert_eq!(count, ITEM_COUNT, "split at byte {cut}");
        assert_eq!(s.pending(), 0);
    }
}

#[test]
fn byte_by_byte_feeding() {
    let mut scratch = [0u8; 64];
    let mut s = StreamDecoder::new(&mut scratch);
    let mut taken = 0;
    for b in STREAM {
        assert_eq!(s.feed(&[*b]), 1);
        while let Some(mut d) = s.take_item().unwrap() {
            d.skip().unwrap();
            taken += 1;
        }
    }
    while s.take_item().unwrap().is_some() {
        taken += 1;
    }
    assert_eq!(taken, ITEM_COUNT);
}

#[test]
fn empty_feed_is_valid() {
    let mut scratch = [0u8; 4];
    let mut s = StreamDecoder::new(&mut scratch);
    assert_eq!(s.feed(&[]), 0);
    assert!(matches!(s.take_item(), Ok(None)));
}

#[test]
fn malformed_data_errors_even_across_boundaries() {
    // ai 28 reserved, split between feeds.
    let mut scratch = [0u8; 8];
    let mut s = StreamDecoder::new(&mut scratch);
    assert_eq!(s.feed(&[0x1c]), 1);
    // No structural check happens until take_item probes.
    assert!(matches!(s.take_item(), Err(Error::INVALID_ADDITIONAL_INFO)));

    // Truncated indefinite array never completes.
    let mut scratch = [0u8; 8];
    let mut s = StreamDecoder::new(&mut scratch);
    s.feed(&[0x9f, 0x01]);
    assert!(matches!(s.take_item(), Ok(None)));

    // Invalid UTF-8 inside text string: structurally fine, so the item is
    // delivered and the error surfaces on typed decode.
    let mut scratch = [0u8; 8];
    let mut s = StreamDecoder::new(&mut scratch);
    s.feed(&[0x62, 0xc0, 0x80]);
    let mut d = match s.take_item() {
        Ok(Some(d)) => d,
        other => panic!("expected Ok(Some), got {other:?}"),
    };
    assert_eq!(d.str(), Err(Error::INVALID_UTF8));

    // Break byte at top level.
    let mut scratch = [0u8; 8];
    let mut s = StreamDecoder::new(&mut scratch);
    s.feed(&[0xff]);
    assert!(matches!(s.take_item(), Err(Error::UNEXPECTED_BREAK)));
}

#[test]
fn remainder_survives_after_item_extraction() {
    // Two items where the second spans the extraction point.
    let mut scratch = [0u8; 16];
    let mut s = StreamDecoder::new(&mut scratch);
    s.feed(&[0x01, 0x63, b'a', b'b', b'c']);
    let mut d = s.take_item().unwrap().unwrap();
    assert_eq!(d.u8(), Ok(1));
    assert_eq!(s.pending(), 4);
    let mut d = s.take_item().unwrap().unwrap();
    assert_eq!(d.str(), Ok("abc"));
    assert_eq!(s.pending(), 0);
}

#[test]
fn capacity_bounds_item_size() {
    let mut scratch = [0u8; 3];
    let mut s = StreamDecoder::new(&mut scratch);
    // Item larger than scratch is partially accepted.
    assert_eq!(s.feed(&[0x63, b'a', b'b', b'c']), 3);
    assert_eq!(s.capacity_left(), 0);
    // Cannot complete -> stays pending forever, no false positives.
    assert!(matches!(s.take_item(), Ok(None)));
    assert_eq!(s.pending(), 3);
}
