//! `Decoder::skip` depth-limit and structural traversal coverage.

use const_cbor::decode::SKIP_LIMIT;
use const_cbor::{Decoder, Encoder, Error};

/// Builds `n` nested single-item arrays: `[ [ [ ... 1 ... ] ] ]`.
fn nested_arrays(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n * 3];
    let mut enc = Encoder::new(&mut buf);
    for _ in 0..n {
        enc.array_len(1).unwrap();
    }
    enc.u8(1).unwrap();
    let (_, end) = enc.finish();
    buf.truncate(end);
    buf
}

#[test]
fn skip_depth_boundary() {
    // The virtual root frame occupies one slot, so SKIP_LIMIT - 1 levels
    // of nesting are the deepest accepted.
    let ok = nested_arrays(SKIP_LIMIT - 1);
    let mut dec = Decoder::new(&ok);
    assert_eq!(dec.skip(), Ok(()));
    assert!(dec.is_empty());

    // One more level is rejected with DEPTH_EXCEEDED.
    let too_deep = nested_arrays(SKIP_LIMIT);
    let mut dec = Decoder::new(&too_deep);
    assert_eq!(dec.skip(), Err(Error::DEPTH_EXCEEDED));

    // Shallow nesting round-trips positionally.
    let shallow = nested_arrays(2);
    let mut dec = Decoder::new(&shallow);
    assert_eq!(dec.skip(), Ok(()));
    assert_eq!(dec.position(), shallow.len());
}

#[test]
fn skip_counts_items_not_bytes() {
    // array(4) of scalars, then a trailing item.
    let input = [0x84, 0x01, 0x02, 0x03, 0x04, 0x05];
    let mut dec = Decoder::new(&input);
    dec.skip().unwrap();
    assert_eq!(dec.position(), 5);
    assert_eq!(dec.u8(), Ok(5));
}

#[test]
fn skip_map_pairs_and_saturation() {
    // map(1) {1: 2} followed by an integer.
    let input = [0xa1, 0x01, 0x02, 0x03];
    let mut dec = Decoder::new(&input);
    dec.skip().unwrap();
    assert_eq!(dec.u8(), Ok(3));

    // Huge declared counts must not overflow the counter arithmetic:
    // map(u64::MAX/…) saturates and then simply runs out of input.
    let huge = {
        let mut b = [0u8; 9];
        let mut e = Encoder::new(&mut b);
        e.map_len(usize::MAX).unwrap();
        let (buf, n) = e.finish();
        buf[..n].to_vec()
    };
    let mut dec = Decoder::new(&huge);
    assert_eq!(dec.skip(), Err(Error::TRUNCATED));
}

#[test]
fn skip_tag_chains_and_empty_containers() {
    // Tag chain wrapping an empty map inside an empty array.
    // c0 c1 a0 -> tag(0, tag(1, {})) ; then 80 = []
    let input = [0xc0, 0xc1, 0xa0, 0x80];
    let mut dec = Decoder::new(&input);
    dec.skip().unwrap();
    assert_eq!(dec.position(), 3);
    dec.skip().unwrap();
    assert_eq!(dec.position(), 4);
}

#[test]
fn truncated_at_every_prefix_never_panics() {
    let mut full = [0u8; 64];
    let mut e = Encoder::new(&mut full);
    e.array_len(2).unwrap();
    e.str("hello").unwrap();
    e.tag(1).unwrap().u32(1000).unwrap();
    e.bytes(&[9, 9, 9]).unwrap();
    let (full, n) = e.finish();

    for cut in 0..n {
        let mut dec = Decoder::new(&full[..cut]);
        // Any outcome is fine as long as it's an Err, not a panic.
        let _ = dec.skip();
        // And typed getters on prefixes are also safe.
        let mut d2 = Decoder::new(&full[..cut]);
        let _ = encode_probe(&mut d2);
    }
}

fn encode_probe(d: &mut Decoder<'_>) -> Result<(), Error> {
    let _ = d.peek_major();
    while !d.is_empty() {
        match d.peek_major() {
            Some(const_cbor::Major::Unsigned) => {
                d.u64()?;
            }
            Some(const_cbor::Major::Negative) => {
                d.i64()?;
            }
            Some(const_cbor::Major::Bytes) => {
                d.bytes()?;
            }
            Some(const_cbor::Major::Text) => {
                d.str()?;
            }
            Some(const_cbor::Major::Array) => {
                let n = d.array_len()?;
                for _ in 0..n.min(4) {
                    d.skip()?;
                }
            }
            _ => return d.skip(),
        }
    }
    Ok(())
}
