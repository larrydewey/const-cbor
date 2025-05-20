// SPDX-License-Identifier: MIT

//! Error types for CBOR encoding and decoding operations.
//!
//! This module defines the possible errors that can occur during CBOR operations.

/// Represents errors that can occur during CBOR encoding and decoding operations.
///
/// These errors are designed to be lightweight and suitable for use in `no_std` environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Error {
    /// The output buffer is too small to contain the encoded data.
    BufferOverflow,

    /// The input contains an invalid or unsupported CBOR data type.
    InvalidType,
}

#[cfg(test)]
mod tests {

    use super::Error;
    use crate::result::Result;

    #[test]
    fn test_error_equality() {
        let err1 = Error::BufferOverflow;
        let err2 = Error::BufferOverflow;
        let err3 = Error::InvalidType;

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_error_copy() {
        let err1 = Error::BufferOverflow;
        let err2 = err1; // This should copy, not move

        // Both should still be valid and equal
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_error_clone() {
        let err1 = Error::BufferOverflow;
        let err2 = err1.clone();

        assert_eq!(err1, err2);
    }

    #[test]
    fn test_result_type() {
        // Test that our Result type alias works as expected
        let ok_result: Result<u32> = Ok(42);
        let err_result: Result<u32> = Err(Error::BufferOverflow);

        assert!(ok_result.is_ok());
        assert!(err_result.is_err());

        assert_eq!(ok_result.unwrap(), 42);
        assert_eq!(err_result.unwrap_err(), Error::BufferOverflow);
    }
}
