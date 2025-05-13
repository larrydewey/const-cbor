// SPDX-License-Identifier: MIT

//! Result type for CBOR operations.
//!
//! This module provides a type alias for simplifying the return type of operations
//! that might result in a CBOR-specific error.

use crate::error::Error;

/// A specialized `Result` type for CBOR operations.
///
/// This type is used throughout the library to return either a successful value `T`
/// or a CBOR-specific `Error`.
pub type Result<T> = core::result::Result<T, Error>;
