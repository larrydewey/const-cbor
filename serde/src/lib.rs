//! Serde support for [`const-cbor`](const_cbor) — a CBOR (RFC 8949)
//! `serde::Serializer` / `serde::Deserializer` over the allocation-free
//! core codec.
//!
//! # Representation choices
//!
//! RFC 8949 does not define a struct/enum mapping, so this crate follows
//! the ecosystem conventions established by `serde_cbor` and `ciborium`
//! for interoperability:
//!
//! * structs and maps serialize as CBOR maps keyed by field name,
//! * enums are externally tagged: unit variants as bare text strings,
//!   other variants as one-entry maps `{ name: payload }`,
//! * sequences serialize as indefinite-length arrays, maps as
//!   indefinite-length maps (the core encoder requires known lengths for
//!   definite forms),
//! * `Option::None`/unit map to `null`,
//! * `char` maps to a single-character text string.
//!
//! The crate is `no_std` + `alloc`; it never allocates on the encode or
//! decode path itself.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod de;
mod error;
mod ser;

pub use crate::de::Deserializer;
pub use crate::error::Error;
pub use crate::ser::Serializer;

use serde::{Deserialize, Serialize};

/// Serializes `value` into `buf`, returning the number of bytes written.
///
/// # Errors
///
/// Encoding failures ([`Error::Cbor`] out-of-space) and unsupported
/// data-model elements.
pub fn to_slice<T: Serialize + ?Sized>(value: &T, buf: &mut [u8]) -> Result<usize, Error> {
    let mut ser = Serializer::new(buf);
    value.serialize(&mut ser)?;
    Ok(ser.written())
}

/// Deserializes a `T` from the complete CBOR `input`.
///
/// Zero-copy: borrowed strings and byte slices point into `input`.
///
/// # Errors
///
/// Structural/type errors from the decoder, plus trailing-bytes checks.
pub fn from_slice<'a, T: Deserialize<'a>>(input: &'a [u8]) -> Result<T, Error> {
    let mut de = Deserializer::new(input);
    let value = T::deserialize(&mut de)?;
    if de.pos != de.input.len() {
        return Err(Error::custom("trailing bytes after value"));
    }
    Ok(value)
}

/// Serializes into a fresh `Vec<u8>` (`alloc`).
///
/// Grows a scratch buffer until the value fits; worst case is O(n) total
/// work across attempts.
///
/// # Errors
///
/// As [`to_slice`] (except out-of-space, which is resolved by growth).
pub fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<alloc::vec::Vec<u8>, Error> {
    let mut capacity = 512usize;
    loop {
        let mut buf = alloc::vec![0u8; capacity];
        match to_slice(value, &mut buf) {
            Ok(n) => {
                buf.truncate(n);
                return Ok(buf);
            }
            Err(Error::Cbor(const_cbor::Error::OUT_OF_SPACE)) => {
                capacity = capacity.saturating_mul(2);
            }
            Err(e) => return Err(e),
        }
    }
}
