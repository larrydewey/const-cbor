//! Demonstrates fully compile-time CBOR encoding.
//!
//! `consts::put_*` panics on overflow, which becomes a compile error inside
//! a `const` block — so these items are checked and built entirely at
//! compile time with zero runtime code.

use const_cbor::encode::consts;

/// [-10] — one byte of output, verified at compile time.
const NEG_TEN: ([u8; 2], usize) = {
    let mut buf = [0u8; 2];
    let n = consts::put_i64(&mut buf, -10);
    (buf, n)
};

/// Encoded size of a 300-byte payload string, computed at compile time.
const CHUNK: usize = consts::chunk_len(300);

fn main() {
    assert_eq!(&NEG_TEN.0[..NEG_TEN.1], &[0x29]);
    assert_eq!(CHUNK, 303);
    println!("const items evaluated at compile time");
}
