//! A const-first, allocation-free, trait-free CBOR codec per RFC 8949.
//!
//! `const-cbor` encodes and decodes CBOR data items directly between Rust
//! values and byte buffers. There is no intermediate value tree, no heap
//! allocation, no `unsafe`, no traits, and no dependencies: the crate uses
//! only `core`.
//!
//! # Design
//!
//! * **`const fn` everywhere** — heads, argument widths, and encoded lengths
//!   are computed by `const fn`s (see [`head`] and [`encode`]), so the
//!   compiler folds them away for statically known inputs. Entire encodings
//!   can be evaluated entirely at compile time with the panicking `put_*`
//!   helpers:
//!
//!   ```
//!   use const_cbor::encode;
//!
//!   const ITEM: ([u8; 4], usize) = {
//!       let mut buf = [0u8; 4];
//!       let n = encode::consts::put_u64(&mut buf, 42);
//!       (buf, n)
//!   };
//!   assert_eq!(&ITEM.0[..ITEM.1], &[0x18, 0x2a]);
//!   ```
//!
//! * **Concrete-typed methods** — encoding and decoding go through
//!   [`Encoder`] and [`Decoder`], which expose one method per CBOR type
//!   (`u8`, `i64`, `str`, `bytes`, `array`, …). No trait impls are required
//!   from users or for user types.
//! * **Zero-copy decoding** — text strings and byte strings borrow from the
//!   input buffer ([`Decoder::str`], [`Decoder::bytes`]).
//! * **Compact errors** — every failure is a single-byte code ([`Error`],
//!   `Copy`, no formatting machinery).
//!
//! # Scope
//!
//! The core supports major types 0–7 with definite lengths, full-width
//! integers (`u8`–`u64`, `i8`–`i64`), tags, simple values, booleans,
//! null/undefined, and — behind the `float` feature — f16/f32/f64.
//! Indefinite-length items (RFC 8949 §3.2) are rejected by the decoder and
//! never produced by the encoder; a streaming decoder is out of scope here.
//!
//! # Behavior guarantees
//!
//! | Property | Guarantee |
//! |---|---|
//! | Panics | never at runtime; malformed input is always `Err(Error)` |
//! | Allocation | never; only `core`, no allocator touched |
//! | Unsafe | forbidden at compile time |
//! | Recursion | none; `Decoder::skip` uses a fixed 32-entry stack |
//! | Failure atomicity | failed encoder calls write nothing and keep the cursor |
//! | Zero-copy | `Decoder::str`/`bytes` borrow from the caller's input |
//!
//! Non-minimal integer encodings are accepted on input (RFC 8949 §4.2.1
//! makes preferred form a SHOULD); forms that are actually invalid —
//! reserved additional information, two-byte simples below 32, non-UTF-8
//! text — are rejected.
//!
//! Further documentation lives in the repository's `docs/` directory
//! (`behavior.md`, `guide.md`, `conformance.md`).
//!
//! # Example: encode then decode
//!
//! ```
//! use const_cbor::{Decoder, Encoder};
//!
//! let mut buf = [0u8; 64];
//! let mut enc = Encoder::new(&mut buf);
//!
//! enc.array_len(3).unwrap();
//! enc.u8(1).unwrap();
//! enc.i64(-25_000).unwrap();
//! enc.str("hello").unwrap();
//!
//! let n = enc.written();
//! assert_eq!(&buf[..n], &[
//!     0x83,             // array(3)
//!     0x01,             // 1
//!     0x39, 0x61, 0xa7, // -25000
//!     0x65, b'h', b'e', b'l', b'l', b'o',
//! ]);
//!
//! let mut dec = Decoder::new(&buf[..n]);
//! assert_eq!(dec.array_len().unwrap(), 3);  // header, then items in order
//! assert_eq!(dec.u8().unwrap(), 1);
//! assert_eq!(dec.i64().unwrap(), -25_000);
//! assert_eq!(dec.str().unwrap(), "hello");
//! assert!(dec.is_empty());
//! ```

#![no_std]
#![forbid(unsafe_code)]

pub mod decode;
pub mod encode;
pub mod error;
pub mod head;

#[cfg(feature = "float")]
pub mod float;

#[cfg(feature = "indefinite")]
pub mod indefinite;

#[cfg(feature = "raw")]
pub mod raw;

#[cfg(feature = "stream")]
pub mod stream;

#[cfg(feature = "canonical")]
pub mod canonical;

#[cfg(feature = "diagnostic")]
pub mod diagnostic;

pub use crate::decode::Decoder;
pub use crate::encode::Encoder;
pub use crate::error::Error;
pub use crate::head::{Major, Simple};
