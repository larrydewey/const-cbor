// SPDX-License-Identifier: MIT

//! Cursor implementation for writing bytes to a buffer.
//!
//! This module provides a `Cursor` type that facilitates writing to a mutable byte slice
//! while tracking the position and handling buffer overflow conditions.

use crate::{error::Error, result::Result};

/// A cursor for writing bytes to a buffer with position tracking.
///
/// This struct maintains a reference to a mutable byte slice and tracks the current
/// position within that slice. It ensures that writes do not exceed the buffer's capacity.
#[derive(Debug, PartialEq)]
pub(crate) struct Cursor<'a> {
    /// The underlying byte buffer where data will be written.
    pub(crate) data: &'a mut [u8],

    /// The current position in the buffer.
    pub(crate) pos: usize,
}

impl<'a> Cursor<'a> {
    /// Creates a new cursor positioned at the start of the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `data` - The mutable byte slice to write into.
    #[inline]
    pub(crate) const fn new(data: &'a mut [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    /// Writes a single byte to the buffer at the current position and advances the cursor.
    ///
    /// # Arguments
    ///
    /// * `byte` - The byte to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the byte was successfully written.
    /// * `Err(Error::BufferOverflow)` if the buffer is full.
    #[inline]
    pub(crate) fn write_byte(&mut self, byte: u8) -> Result<()> {
        if self.pos < self.data.len() {
            self.data[self.pos] = byte;
            self.pos += 1;
            Ok(())
        } else {
            Err(Error::BufferOverflow)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Value, encode::encode, error::Error};

    #[test]
    fn test_cursor_overflow() {
        // Create a byte string that's larger than our buffer
        let large_array = [0u8; 100];
        let value = Value::bytes(&large_array);

        // Try to encode into a smaller buffer
        let mut small_buf = [0u8; 10];
        let result = encode(&value, &mut small_buf);

        // Should result in a buffer overflow error
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::BufferOverflow);
    }

    #[test]
    fn test_multiple_writes() {
        // Create a nested structure that requires multiple write operations
        let inner1 = [Value::unsigned(1), Value::unsigned(2)];
        let inner2 = [Value::unsigned(3), Value::unsigned(4)];

        // Store the arrays in variables to extend their lifetime
        let array1 = Value::array(&inner1);
        let array2 = Value::array(&inner2);
        let outer_array = [array1, array2];

        let value = Value::array(&outer_array);

        // Buffer should be large enough
        let mut buf = [0u8; 32];
        let result = encode(&value, &mut buf);

        // Should succeed
        assert!(result.is_ok());
        let size = result.unwrap();

        // Expected encoding:
        // 0x82 (array of 2 items)
        //   0x82 (array of 2 items)
        //     0x01 (1)
        //     0x02 (2)
        //   0x82 (array of 2 items)
        //     0x03 (3)
        //     0x04 (4)
        assert_eq!(size, 7);
        assert_eq!(buf[0], 0x82);
        assert_eq!(buf[1], 0x82);
        assert_eq!(buf[2], 0x01);
        assert_eq!(buf[3], 0x02);
        assert_eq!(buf[4], 0x82);
        assert_eq!(buf[5], 0x03);
        assert_eq!(buf[6], 0x04);
    }

    #[test]
    fn test_cursor_exact_size() {
        // Create a value that needs exactly N bytes
        let value = Value::unsigned(42); // Needs 2 bytes: 0x18, 0x2A

        // Create a buffer of exactly that size
        let mut buf = [0u8; 2];
        let result = encode(&value, &mut buf);

        // Should succeed exactly
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert_eq!(buf[0], 0x18);
        assert_eq!(buf[1], 42);
    }

    #[test]
    fn test_cursor_one_byte_too_small() {
        // Create a value that needs exactly N bytes
        let value = Value::unsigned(42); // Needs 2 bytes: 0x18, 0x2A

        // Create a buffer one byte too small
        let mut buf = [0u8; 1];
        let result = encode(&value, &mut buf);

        // Should fail with buffer overflow
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::BufferOverflow);
    }
}
