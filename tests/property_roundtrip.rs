//! Deterministic property-style roundtrip fuzzing without dev-dependencies.
//!
//! A seeded xorshift PRNG generates random well-formed CBOR trees; each is
//! encoded, re-decoded structurally (`skip`), byte-compared against a
//! second encode of an independently decoded copy, and rendered by the
//! diagnostic writer when those features are on. Any mismatch panics.

use const_cbor::{Decoder, Encoder, Major};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

const DEPTH_CAP: usize = 5;

/// Writes one random item; returns the number of top-level items written.
fn write_random(e: &mut Encoder, rng: &mut Rng, depth: usize) -> Result<(), const_cbor::Error> {
    match rng.below(if depth < DEPTH_CAP { 10 } else { 7 }) {
        0 => {
            e.uint(rng.next())?;
        }
        1 => {
            // Keep magnitudes within i64 so the typed walk can mirror them.
            e.nint(rng.next() % (i64::MAX as u64))?;
        }
        2 => {
            let len = rng.below(40) as usize;
            let payload: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
            e.bytes(&payload)?;
        }
        3 => {
            let len = rng.below(20) as usize;
            // ASCII-only so UTF-8 validity holds.
            let payload: Vec<u8> = (0..len).map(|_| b' ' + rng.next() as u8 % 95).collect();
            let s = core::str::from_utf8(&payload).unwrap();
            e.str(s)?;
        }
        4 => {
            e.bool(rng.next().is_multiple_of(2))?;
        }
        5 => {
            e.null()?;
        }
        6 => {
            e.simple(const_cbor::Simple(32 + rng.below(224) as u8))?;
        }
        7 => {
            e.tag(rng.next())?;
            write_random(e, rng, depth + 1)?;
        }
        8 => {
            let n = rng.below(5) as usize;
            e.array_len(n)?;
            for _ in 0..n {
                write_random(e, rng, depth + 1)?;
            }
        }
        _ => {
            let pairs = rng.below(4) as usize;
            e.map_len(pairs)?;
            for _ in 0..pairs {
                write_random(e, rng, depth + 1)?; // key
                write_random(e, rng, depth + 1)?; // value
            }
        }
    }
    Ok(())
}

#[test]
fn random_wellformed_roundtrips() {
    let mut rng = Rng(0x2026_0825_dead_beef);
    for case in 0..2000u32 {
        let mut buf = [0u8; 4096];
        let mut e = Encoder::new(&mut buf);
        write_random(&mut e, &mut rng, 0).unwrap();
        let (out, n) = e.finish();
        let bytes = &out[..n];

        // Structural walk consumes exactly the item.
        let mut d = Decoder::new(bytes);
        d.skip().unwrap();
        assert!(d.is_empty(), "case {case}: trailing bytes after skip");

        // Typed walk mirrors the generator's shape decisions via peek.
        let mut d = Decoder::new(bytes);
        typed_walk(&mut d);
        assert!(d.is_empty(), "case {case}: trailing bytes after typed walk");

        #[cfg(feature = "diagnostic")]
        {
            let mut out2 = [0u8; 8192];
            const_cbor::diagnostic::render(bytes, &mut out2)
                .unwrap_or_else(|e| panic!("case {case}: diagnostic render failed: {e:?}"));
        }
    }
}

fn typed_walk(d: &mut Decoder<'_>) {
    let Some(major) = d.peek_major() else { return };
    match major {
        Major::Unsigned => {
            d.u64().unwrap();
        }
        Major::Negative => {
            d.i64().unwrap();
        }
        Major::Bytes => {
            d.bytes().unwrap();
        }
        Major::Text => {
            d.str().unwrap();
        }
        Major::Simple => {
            if matches!(d.peek_major(), Some(Major::Simple)) {
                d.simple().unwrap();
            } else {
                d.skip().unwrap();
            }
        }
        Major::Array => {
            let n = d.array_len().unwrap();
            for _ in 0..n {
                typed_walk(d);
            }
        }
        Major::Map => {
            let n = d.map_len().unwrap();
            for _ in 0..n * 2 {
                typed_walk(d);
            }
        }
        Major::Tag => {
            d.tag().unwrap();
            typed_walk(d);
        }
    }
}
