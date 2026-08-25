//! Adversarial indefinite-length coverage (RFC 8949 §3.2).
#![cfg(feature = "indefinite")]

use const_cbor::{Decoder, Encoder, Error, Major, Simple};

#[test]
fn zero_chunk_strings_are_valid() {
    // RFC: an indefinite string with no chunks and a break is well-formed.
    for input in [&[0x5fu8, 0xff][..], &[0x7f, 0xff][..]] {
        let mut dec = Decoder::new(input);
        assert_eq!(dec.skip(), Ok(()), "input {input:?}");
        assert!(dec.is_empty());
    }
}

#[test]
fn chunk_type_mismatch_is_malformed() {
    // Indefinite byte string containing a *text* chunk.
    let mut dec = Decoder::new(&[0x5f, 0x60, 0xff]);
    assert_eq!(dec.skip(), Err(Error::TYPE_MISMATCH));
    // Text iterator over a byte chunk fails cleanly.
    let mut dec = Decoder::new(&[0x7f, 0x41, 0x00, 0xff]);
    let mut it = dec.str_chunks().unwrap();
    assert_eq!(it.next(), None);
    assert!(!it.finished());
    dec.sync_chunks(&it).unwrap_err();
}

#[test]
fn break_in_wrong_positions() {
    // Break at top level.
    let mut dec = Decoder::new(&[0xff]);
    assert_eq!(dec.skip(), Err(Error::UNEXPECTED_BREAK));
    // Break inside a definite array.
    let mut dec = Decoder::new(&[0x81, 0xff]);
    assert_eq!(dec.skip(), Err(Error::UNEXPECTED_BREAK));
    // Break *after* a complete definite item is simply trailing garbage:
    // skip() completes at the end of the well-formed prefix.
    let mut dec = Decoder::new(&[0x81, 0x01, 0xff]);
    assert_eq!(dec.skip(), Ok(()));
    assert_eq!(dec.position(), 2);
    assert_eq!(dec.peek_major(), Some(Major::Simple));
}

#[test]
fn truncated_indefinite_every_prefix() {
    // Full item: indefinite array [1, 2] with break.
    let full = [0x9f, 0x01, 0x02, 0xff];
    for cut in 1..full.len() {
        let mut dec = Decoder::new(&full[..cut]);
        assert_eq!(dec.skip(), Err(Error::TRUNCATED), "prefix of len {cut} must be truncated");
    }
    // Same for chunked strings, per byte.
    let full = [0x7f, 0x61, b'x', 0xff];
    for cut in 1..full.len() {
        let mut dec = Decoder::new(&full[..cut]);
        let _ = dec.skip(); // may be TRUNCATED or TYPE_MISMATCH mid-chunk; must not panic
    }
}

#[test]
fn nested_indefinite_inside_definite() {
    let input = [0x82, 0x9f, 0x01, 0xff, 0x02];
    let mut dec = Decoder::new(&input);
    assert_eq!(dec.array_len(), Ok(2));
    dec.skip().unwrap();
    assert_eq!(dec.u8(), Ok(2));

    // Definite inside indefinite.
    let input = [0x9f, 0x80, 0xa0, 0xff]; // [_ [] {} ]
    let mut dec = Decoder::new(&input);
    dec.skip().unwrap();
    assert!(dec.is_empty());
}

#[test]
fn encoder_scope_tracking() {
    let mut buf = [0u8; 64];
    let mut e = Encoder::new(&mut buf);

    // end_indef with nothing open.
    assert!(matches!(e.end_indef(), Err(Error::TYPE_MISMATCH)));
    assert_eq!(e.written(), 0);

    // Wrong-kind close.
    e.begin_bytes().unwrap();
    assert!(matches!(e.end_str(), Err(Error::TYPE_MISMATCH)));
    // Chunk into the wrong kind.
    assert!(matches!(e.str_chunk("x"), Err(Error::TYPE_MISMATCH)));
    e.bytes_chunk(b"ab").unwrap();
    e.end_bytes().unwrap();

    // begin_* out of space rolls back the scope push.
    let mut tiny = [0u8; 0];
    let mut e2 = Encoder::new(&mut tiny);
    assert!(matches!(e2.begin_array(), Err(Error::OUT_OF_SPACE)));
    assert!(matches!(e2.end_indef(), Err(Error::TYPE_MISMATCH)), "scope must be rolled back");

    // write_break out of space keeps the scope open.
    let mut tiny = [0u8; 1];
    let mut e3 = Encoder::new(&mut tiny);
    e3.begin_map().unwrap();
    assert!(matches!(e3.end_indef(), Err(Error::OUT_OF_SPACE)));
    // Scope still open: wrong-kind close proves it wasn't popped.
    assert!(matches!(e3.end_bytes(), Err(Error::TYPE_MISMATCH)));
}

#[test]
fn encoder_depth_limit() {
    let mut buf = [0u8; 256];
    let mut e = Encoder::new(&mut buf);
    for _ in 0..const_cbor::indefinite::ENCODE_DEPTH_LIMIT {
        e.begin_array().unwrap();
    }
    assert!(matches!(e.begin_array(), Err(Error::DEPTH_EXCEEDED)));
    for _ in 0..const_cbor::indefinite::ENCODE_DEPTH_LIMIT {
        e.end_indef().unwrap();
    }
    assert!(matches!(e.end_indef(), Err(Error::TYPE_MISMATCH)));
}

#[test]
fn string_chunk_iterators() {
    // Bytes chunks: iterate, finish, sync.
    let mut buf = [0u8; 32];
    let mut e = Encoder::new(&mut buf);
    e.bytes_indef(&[b"ab", b"cd"]).unwrap();
    let (buf, n) = e.finish();
    let mut dec = Decoder::new(&buf[..n]);
    let mut it = dec.bytes_chunks().unwrap();
    assert_eq!(&it.next().unwrap(), b"ab");
    assert_eq!(&it.next().unwrap(), b"cd");
    assert_eq!(it.next(), None);
    assert!(it.finished());
    dec.sync_chunks(&it).unwrap();
    assert!(dec.is_empty());

    // Str chunks with UTF-8 validation.
    let mut buf = [0u8; 32];
    let mut e = Encoder::new(&mut buf);
    e.str_indef(&["he", "llo"]).unwrap();
    let (buf, n) = e.finish();
    let mut dec = Decoder::new(&buf[..n]);
    let mut it = dec.str_chunks().unwrap();
    assert_eq!(it.next(), Some("he"));
    assert_eq!(it.next(), Some("llo"));
    assert_eq!(it.next(), None);
    dec.sync_chunks(&it).unwrap();
    assert_eq!(n, dec.position());

    // Zero-chunk text string.
    let mut buf = [0u8; 4];
    let mut e = Encoder::new(&mut buf);
    e.str_indef(&[]).unwrap();
    let (buf, n) = e.finish();
    assert_eq!(&buf[..n], &[0x7f, 0xff]);
    let mut dec = Decoder::new(&buf[..n]);
    let mut it = dec.str_chunks().unwrap();
    assert_eq!(it.next(), None);
    assert!(it.finished());
    dec.sync_chunks(&it).unwrap();
}

#[test]
fn chunk_iterator_malformed_states() {
    // Truncated (no break): finished() == false, sync fails TRUNCATED.
    let input = [0x5f, 0x41, 0x00];
    let mut dec = Decoder::new(&input);
    let mut it = dec.bytes_chunks().unwrap();
    assert_eq!(Some(&[0u8][..]), it.next());
    assert_eq!(it.next(), None);
    assert!(!it.finished());
    assert_eq!(dec.sync_chunks(&it), Err(Error::TRUNCATED));

    // Iterator is Copy: resuming from a copy continues where left off.
    let good = [0x5f, 0x41, 0x00, 0xff];
    let mut dec = Decoder::new(&good);
    let mut it = dec.bytes_chunks().unwrap();
    assert_eq!(Some(&[0u8][..]), it.next());
    // Only the break byte remains.
    assert_eq!(it.unconsumed(), 1);
    let mut resumed = it;
    assert_eq!(resumed.next(), None);
    assert!(resumed.finished());
    assert_eq!(resumed.unconsumed(), 0);
    // The original copy still sits before the break: Copy semantics mean
    // each copy advances independently.
    assert!(!it.finished());
    assert_eq!(it.unconsumed(), 1);

    // Type mismatch: bytes_chunks on a definite byte string.
    let mut dec = Decoder::new(&[0x41, 0x00]);
    assert!(matches!(dec.bytes_chunks(), Err(Error::TYPE_MISMATCH)));
    let mut dec = Decoder::new(&[0x41, 0x00]);
    assert!(matches!(dec.str_chunks(), Err(Error::TYPE_MISMATCH)));
}

#[test]
fn skip_of_deeply_nested_indefinite() {
    // Interleave indefinite/definite nesting up to just under SKIP_LIMIT.
    let mut buf = vec![0u8; 512];
    let mut e = Encoder::new(&mut buf);
    let mut opens = 0usize;
    for i in 0..(const_cbor::decode::SKIP_LIMIT - 1) {
        if i % 2 == 0 {
            e.begin_array().unwrap();
            opens += 1;
        } else {
            e.array_len(1).unwrap();
        }
    }
    e.u8(7).unwrap();
    for _ in 0..opens {
        e.end_indef().unwrap();
    }
    let (buf, n) = e.finish();
    let mut dec = Decoder::new(&buf[..n]);
    assert_eq!(dec.skip(), Ok(()));
    assert!(dec.is_empty());

    // Unbalanced indefinite at depth: missing break -> truncated.
    let mut buf2 = vec![0u8; 512];
    let mut e2 = Encoder::new(&mut buf2);
    e2.begin_array().unwrap().begin_array().unwrap().u8(1).unwrap();
    let (buf2, n2) = e2.finish();
    let mut dec = Decoder::new(&buf2[..n2]);
    assert_eq!(dec.skip(), Err(Error::TRUNCATED));
}

#[test]
fn simple_and_major_helpers_with_indefinite_feature() {
    // Major::bits / from_initial still behave when indefinite code is linked.
    assert_eq!(Major::from_initial(0x1f), Major::Unsigned);
    assert!(Simple(255).valid_two_byte());
}
