// SPDX-License-Identifier: MIT

//! # const-cbor
//!
//! A `no_std` compatible library for Concise Binary Object Representation (CBOR)
//! encoding and decoding that supports compile-time operations via `const fn`.
//!
//! CBOR is a data format whose design goals include the possibility of extremely small
//! code size, fairly small message size, and extensibility without the need for version
//! negotiation. This implementation focuses on being usable in `const` contexts, making
//! it ideal for embedded systems and other resource-constrained environments.
//!
//! ## Example
//!
//! ```rust
//! use const_cbor::{Value, encode::encode};
//!
//! // Create a CBOR value
//! let value = Value::unsigned(42);
//! let mut buf = [0u8; 16];
//!
//! // Encode it to a buffer
//! let size = encode(&value, &mut buf).unwrap();
//! assert_eq!(size, 2);
//! assert_eq!(buf[0], 0x18);
//! assert_eq!(buf[1], 42);
//! ```

#![no_std]
#![deny(unsafe_code)]
#![forbid(
    clippy::all,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    absolute_paths_not_starting_with_crate,
    deprecated_in_future,
    missing_copy_implementations,
    noop_method_call,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_code,
    unreachable_patterns,
    unstable_features,
    unused,
    unsafe_op_in_unsafe_fn,
    unused_import_braces,
    unused_results,
    variant_size_differences
)]

pub mod encode;
pub mod error;
pub mod result;

mod value;

pub use value::*;
