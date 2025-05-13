// SPDX-License-Identifier: MIT

//! CBOR encoding functionality.
//!
//! This module provides functions and types for encoding CBOR values into bytes.
//! The implementation follows [RFC 7049](https://tools.ietf.org/html/rfc7049).
//!
//! # Examples
//!
//! ```rust
//! use const_cbor::{Value, encode::encode};
//!
//! // Create a simple integer value
//! let value = Value::unsigned(42);
//!
//! // Prepare a buffer to hold the encoded bytes
//! let mut buf = [0u8; 16];
//!
//! // Encode the value into the buffer
//! let bytes_written = encode(&value, &mut buf).unwrap();
//!
//! // The encoded result should be [0x18, 0x2A] (CBOR for integer 42)
//! assert_eq!(bytes_written, 2);
//! assert_eq!(buf[0], 0x18);
//! assert_eq!(buf[1], 42);
//! ```

use crate::{Value, result::Result};

mod cursor;

use cursor::Cursor;

/// Trait for types that can be encoded as CBOR.
///
/// Implementing this trait for a type allows it to be converted to its CBOR
/// representation and written to a buffer.
pub trait Encode<'a> {
    /// Encodes the implementing type as CBOR into the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - The buffer to write the CBOR encoded data into.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written to the buffer.
    /// * `Err(Error)` - If an error occurred during encoding.
    fn as_cbor(&'a self, buf: &'a mut [u8]) -> Result<usize>;
}

/// CBOR major types as defined in RFC 7049.
///
/// The major type is encoded in the high-order 3 bits of the first byte
/// of a data item, and indicates the basic type of the data item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MajorType {
    /// Major type 0: Unsigned integer (0..23, U8..U64)
    Unsigned = 0,

    /// Major type 1: Negative integer (-1..-18446744073709551616)
    Negative = 1,

    /// Major type 2: Byte string (0..2^64-1 bytes)
    Bytes = 2,

    /// Major type 3: Text string (0..2^64-1 bytes)
    Text = 3,

    /// Major type 4: Array of data items
    Array = 4,

    /// Major type 5: Map of pairs of data items
    Map = 5,

    /// Major type 6: Tagged data items
    Tag = 6,

    /// Major type 7: Simple values, floating point, and special values
    Simple = 7,
}

/// Encodes a CBOR header byte and additional bytes for the given major type and value.
///
/// This function follows the CBOR encoding rules to create a header byte with the major type
/// in the high 3 bits and the additional information in the low 5 bits, along with any
/// additional bytes needed to represent the value.
///
/// # Arguments
///
/// * `major` - The major type (0-7) of the CBOR data item.
/// * `value` - The value to encode (interpreted according to the major type).
///
/// # Returns
///
/// A tuple containing:
/// * The header byte with the major type and additional information.
/// * An array of up to 8 additional bytes for the value (if needed).
/// * The number of additional bytes used (0-8).
#[inline]
const fn encode_header(major: u8, value: u64) -> (u8, [u8; 8], usize) {
    let major_shift = major << 5;

    if value <= 23 {
        (major_shift | value as u8, [0; 8], 0)
    } else if value <= u8::MAX as u64 {
        (major_shift | 24, [value as u8, 0, 0, 0, 0, 0, 0, 0], 1)
    } else if value <= u16::MAX as u64 {
        (
            major_shift | 25,
            [(value >> 8) as u8, value as u8, 0, 0, 0, 0, 0, 0],
            2,
        )
    } else if value <= u32::MAX as u64 {
        (
            major_shift | 26,
            [
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
                0,
                0,
                0,
                0,
            ],
            4,
        )
    } else {
        (
            major_shift | 27,
            [
                (value >> 56) as u8,
                (value >> 48) as u8,
                (value >> 40) as u8,
                (value >> 32) as u8,
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ],
            8,
        )
    }
}

/// Calculates the number of bytes needed to encode a CBOR value.
///
/// This function traverses the `Value` structure recursively to determine exactly how many
/// bytes would be required to encode it in CBOR format. This is useful for allocating
/// buffers of the correct size before encoding.
///
/// # Arguments
///
/// * `value` - The CBOR value to calculate the encoded size for.
///
/// # Returns
///
/// The number of bytes needed to encode the value in CBOR format.
///
/// # Examples
///
/// ```
/// use const_cbor::{Value, encode::encoded_size};
///
/// let value = Value::unsigned(42);
/// let size = encoded_size(&value);
/// assert_eq!(size, 2); // 1 byte for header, 1 byte for value
/// ```
#[inline]
pub const fn encoded_size(value: &Value) -> usize {
    match value {
        Value::Unsigned(n) => {
            let (_, _, extra) = encode_header(MajorType::Unsigned as u8, *n);
            1 + extra
        }
        Value::Negative(n) => {
            let (_, _, extra) = encode_header(MajorType::Negative as u8, *n);
            1 + extra
        }
        Value::Bytes(b) => {
            let (_, _, extra) = encode_header(MajorType::Bytes as u8, b.len() as u64);
            1 + extra + b.len()
        }
        Value::Text(t) => {
            let (_, _, extra) = encode_header(MajorType::Text as u8, t.len() as u64);
            1 + extra + t.len()
        }
        Value::Array(items) => {
            let (_, _, extra) = encode_header(MajorType::Array as u8, items.len() as u64);
            let mut size = 1 + extra;
            let mut i = 0;

            while i < items.len() {
                size += encoded_size(&items[i]);
                i += 1;
            }
            size
        }
        Value::Map(pairs) => {
            let (_, _, extra) = encode_header(MajorType::Map as u8, pairs.len() as u64);
            let mut size = 1 + extra;
            let mut i = 0;

            while i < pairs.len() {
                size += encoded_size(&pairs[i].0) + encoded_size(&pairs[i].1);
                i += 1;
            }
            size
        }
        Value::Tag(tag, item) => {
            let (_, _, extra) = encode_header(MajorType::Tag as u8, *tag);
            1 + extra + encoded_size(item)
        }
        Value::Simple(s) => {
            let (_, _, extra) = encode_header(MajorType::Simple as u8, *s as u64);
            1 + extra
        }
        Value::Float(_) => 9,
    }
}

/// Encodes a CBOR value into a byte buffer.
///
/// This is the main encoding function that converts a `Value` into its CBOR binary representation.
/// It writes the encoded value to the provided buffer and returns the number of bytes written.
///
/// # Arguments
///
/// * `value` - The CBOR value to encode.
/// * `buf` - The buffer to write the encoded data into.
///
/// # Returns
///
/// * `Ok(usize)` - The number of bytes written to the buffer.
/// * `Err(Error::BufferOverflow)` - If the buffer is too small to hold the encoded data.
///
/// # Examples
///
/// ```
/// use const_cbor::{Value, encode::encode};
///
/// let value = Value::unsigned(42);
/// let mut buf = [0u8; 16];
/// let size = encode(&value, &mut buf).unwrap();
/// assert_eq!(size, 2);
/// assert_eq!(buf[0], 0x18); // CBOR header for uint8
/// assert_eq!(buf[1], 42);   // the value
/// ```
#[inline]
pub fn encode(value: &Value, buf: &mut [u8]) -> Result<usize> {
    let mut cursor = Cursor::new(buf);
    encode_value(value, &mut cursor)?;
    Ok(cursor.pos)
}

/// Internal function that encodes a CBOR value using a cursor.
///
/// This function performs the actual encoding by writing bytes to the cursor based on
/// the type and content of the value. It handles different CBOR major types and
/// recursively processes nested values.
///
/// # Arguments
///
/// * `value` - The CBOR value to encode.
/// * `cursor` - A mutable reference to the cursor used for writing bytes.
///
/// # Returns
///
/// * `Ok(())` - If the value was successfully encoded.
/// * `Err(Error::BufferOverflow)` - If the cursor's buffer is too small.
#[inline]
fn encode_value(value: &Value, cursor: &mut Cursor) -> Result<()> {
    match value {
        Value::Unsigned(n) => {
            let (header, extra, len) = encode_header(MajorType::Unsigned as u8, *n);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
        }
        Value::Negative(n) => {
            let (header, extra, len) = encode_header(MajorType::Negative as u8, *n);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
        }
        Value::Bytes(bytes) => {
            let (header, extra, len) = encode_header(MajorType::Bytes as u8, bytes.len() as u64);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
            for &byte in *bytes {
                cursor.write_byte(byte)?;
            }
        }
        Value::Text(text) => {
            let (header, extra, len) = encode_header(MajorType::Text as u8, text.len() as u64);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
            for &byte in text.as_bytes() {
                cursor.write_byte(byte)?;
            }
        }
        Value::Array(items) => {
            let (header, extra, len) = encode_header(MajorType::Array as u8, items.len() as u64);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
            for value in *items {
                encode_value(value, cursor)?;
            }
        }
        Value::Map(pairs) => {
            let (header, extra, len) = encode_header(MajorType::Map as u8, pairs.len() as u64);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
            for (key, value) in *pairs {
                encode_value(key, cursor)?;
                encode_value(value, cursor)?;
            }
        }
        Value::Tag(tag, item) => {
            let (header, extra, len) = encode_header(MajorType::Tag as u8, *tag);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
            encode_value(item, cursor)?;
        }
        Value::Simple(s) => {
            let (header, extra, len) = encode_header(MajorType::Simple as u8, *s as u64);
            cursor.write_byte(header)?;
            for i in 0..len {
                cursor.write_byte(extra[i])?;
            }
        }
        Value::Float(f) => {
            cursor.write_byte((MajorType::Simple as u8) << 5 | 27)?;
            let bytes = f.to_bits().to_be_bytes();
            for byte in bytes {
                cursor.write_byte(byte)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Value;
    use crate::encode::{encode, encoded_size};
    use crate::error::Error;

    /// Test encoding of unsigned integers.
    ///
    /// This test verifies that a small unsigned integer (42) is encoded correctly using
    /// the CBOR format. According to the specification, it should be encoded as:
    /// - 0x18: Major type 0 (unsigned integer) with additional info 24 (8-bit uint follows)
    /// - 0x2A: The value 42 in hexadecimal
    #[test]
    fn test_uint() {
        let value = Value::unsigned(42);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 2);
        assert_eq!(buf[0], 0x18);
        assert_eq!(buf[1], 42);
        assert_eq!(encoded_size(&value), 2);
    }

    /// Test encoding of null values.
    ///
    /// This test verifies that CBOR null is encoded correctly. According to the
    /// specification, null should be encoded as 0xF6, which is major type 7
    /// (simple value) with the simple value 22.
    #[test]
    fn test_null() {
        let value = Value::null();
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 1);
        assert_eq!(buf[0], 0xf6);
        assert_eq!(encoded_size(&value), 1);
    }

    /// Test encoding of arrays.
    ///
    /// This test verifies that an array containing two unsigned integers (1 and 2)
    /// is encoded correctly. According to the specification, it should be encoded as:
    /// - 0x82: Major type 4 (array) with additional info 2 (array of 2 elements)
    /// - 0x01: Unsigned integer 1
    /// - 0x02: Unsigned integer 2
    #[test]
    fn test_array() {
        let items = [Value::unsigned(1), Value::unsigned(2)];

        let value = Value::array(&items);

        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 3);
        assert_eq!(buf[0], 0x82);
        assert_eq!(buf[1], 0x01);
        assert_eq!(buf[2], 0x02);
        assert_eq!(encoded_size(&value), 3);
    }

    /// Test encoding of maps with nested structures.
    ///
    /// This test verifies that a complex map with string keys and various value types
    /// (including strings, booleans, and a nested array) is encoded correctly according
    /// to the CBOR specification. The expected byte sequence is detailed in the assertions.
    #[test]
    fn test_map() {
        let colors = [Value::text("blue"), Value::text("green")];

        let items = [
            (Value::text("first_name"), Value::text("River")),
            (Value::text("last_name"), Value::text("Song")),
            (Value::text("is_admin"), Value::bool(true)),
            (Value::text("favorite_colors"), Value::array(&colors)),
        ];

        let map = Value::map(&items);

        let mut buf = [0u8; 75];

        let size = encode(&map, &mut buf).unwrap();

        assert_eq!(size, 71);

        assert_eq!(
            &buf[..size],
            &[
                0xA4, // Map with 4 pairs
                0x6A, // String with 10 bytes
                0x66, 0x69, 0x72, 0x73, 0x74, 0x5F, 0x6E, 0x61, 0x6D, 0x65, // "first_name"
                0x65, // String with 5 bytes
                0x52, 0x69, 0x76, 0x65, 0x72, // "River"
                0x69, // String with 9 bytes
                0x6C, 0x61, 0x73, 0x74, 0x5F, 0x6E, 0x61, 0x6D, 0x65, // "last_name"
                0x64, // String with 4 bytes
                0x53, 0x6F, 0x6E, 0x67, // "Song"
                0x68, // String with 8 bytes
                0x69, 0x73, 0x5F, 0x61, 0x64, 0x6D, 0x69, 0x6E, // "is_admin"
                0xF5, // True
                0x6F, // String with 15 bytes
                0x66, 0x61, 0x76, 0x6F, 0x72, 0x69, 0x74, 0x65, 0x5F, 0x63, 0x6F, 0x6C, 0x6F, 0x72,
                0x73, // "favorite_colors"
                0x82, // Array with 2 items
                0x64, // String with 4 bytes
                0x62, 0x6C, 0x75, 0x65, // "blue"
                0x65, // String with 5 bytes
                0x67, 0x72, 0x65, 0x65, 0x6E, // "green"
            ]
        )
    }

    // Tests for different unsigned integer encodings
    #[test]
    fn test_encode_uint_small() {
        let value = Value::unsigned(23);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x17); // 0x00 | 23 (direct value)
    }

    #[test]
    fn test_encode_uint_u8() {
        let value = Value::unsigned(255);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 2);
        assert_eq!(buf[0], 0x18); // 0x00 | 24 (one-byte uint follows)
        assert_eq!(buf[1], 255);
    }

    #[test]
    fn test_encode_uint_u16() {
        let value = Value::unsigned(1000);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 3);
        assert_eq!(buf[0], 0x19); // 0x00 | 25 (two-byte uint follows)
        assert_eq!(buf[1], 0x03); // 1000 >> 8
        assert_eq!(buf[2], 0xE8); // 1000 & 0xFF
    }

    #[test]
    fn test_encode_uint_u32() {
        let value = Value::unsigned(1000000);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 5);
        assert_eq!(buf[0], 0x1A); // 0x00 | 26 (four-byte uint follows)
        assert_eq!(buf[1], 0x00); // 1000000 >> 24
        assert_eq!(buf[2], 0x0F); // (1000000 >> 16) & 0xFF
        assert_eq!(buf[3], 0x42); // (1000000 >> 8) & 0xFF
        assert_eq!(buf[4], 0x40); // 1000000 & 0xFF
    }

    #[test]
    fn test_encode_uint_u64() {
        let value = Value::unsigned(1000000000000);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 9);
        assert_eq!(buf[0], 0x1B); // 0x00 | 27 (eight-byte uint follows)
        assert_eq!(
            &buf[1..9],
            &[0x00, 0x00, 0x00, 0xE8, 0xD4, 0xA5, 0x10, 0x00]
        );
    }

    // Tests for different negative integer encodings
    #[test]
    fn test_encode_negative_small() {
        let value = Value::negative(-10); // -10 is encoded as 9
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x29); // 0x20 | 9 (direct value)
    }

    #[test]
    fn test_encode_negative_u8() {
        let value = Value::negative(-100); // -100 is encoded as 99
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 2);
        assert_eq!(buf[0], 0x38); // 0x20 | 24 (one-byte uint follows)
        assert_eq!(buf[1], 99);
    }

    #[test]
    fn test_encode_negative_u16() {
        let value = Value::negative(-1000); // -1000 is encoded as 999
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 3);
        assert_eq!(buf[0], 0x39); // 0x20 | 25 (two-byte uint follows)
        assert_eq!(buf[1], 0x03); // 999 >> 8
        assert_eq!(buf[2], 0xE7); // 999 & 0xFF
    }

    #[test]
    fn test_encode_negative_u32() {
        let value = Value::negative(-1000000); // -1000000 is encoded as 999999
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 5);
        assert_eq!(buf[0], 0x3A); // 0x20 | 26 (four-byte uint follows)
        assert_eq!(buf[1], 0x00); // 999999 >> 24
        assert_eq!(buf[2], 0x0F); // (999999 >> 16) & 0xFF
        assert_eq!(buf[3], 0x42); // (999999 >> 8) & 0xFF
        assert_eq!(buf[4], 0x3F); // 999999 & 0xFF
    }

    // Tests for byte string encoding
    #[test]
    fn test_encode_empty_byte_string() {
        let value = Value::bytes(&[]);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x40); // 0x40 | 0 (empty byte string)
    }

    #[test]
    fn test_encode_byte_string() {
        let value = Value::bytes(&[0x01, 0x02, 0x03]);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 4);
        assert_eq!(buf[0], 0x43); // 0x40 | 3 (byte string of length 3)
        assert_eq!(buf[1], 0x01);
        assert_eq!(buf[2], 0x02);
        assert_eq!(buf[3], 0x03);
    }

    #[test]
    fn test_encode_long_byte_string() {
        let bytes = [0xAA; 100];
        let value = Value::bytes(&bytes);
        let mut buf = [0u8; 110];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 102);
        assert_eq!(buf[0], 0x58); // 0x40 | 24, followed by length 100 as one byte
        assert_eq!(buf[1], 100);
        for i in 0..100 {
            assert_eq!(buf[i + 2], 0xAA);
        }
    }

    // Tests for text string encoding
    #[test]
    fn test_encode_empty_text_string() {
        let value = Value::text("");
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0x60); // 0x60 | 0 (empty text string)
    }

    #[test]
    fn test_encode_text_string() {
        let value = Value::text("abc");
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 4);
        assert_eq!(buf[0], 0x63); // 0x60 | 3 (text string of length 3)
        assert_eq!(buf[1], b'a');
        assert_eq!(buf[2], b'b');
        assert_eq!(buf[3], b'c');
    }

    // Tests for boolean encoding
    #[test]
    fn test_encode_boolean_true() {
        let value = Value::bool(true);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0xF5); // 0xE0 | 21 (true)
    }

    #[test]
    fn test_encode_boolean_false() {
        let value = Value::bool(false);
        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();
        assert_eq!(size, 1);
        assert_eq!(buf[0], 0xF4); // 0xE0 | 20 (false)
    }

    // Tests for complex nested structures
    #[test]
    fn test_encode_nested_array() {
        let inner = [Value::unsigned(1), Value::unsigned(2)];
        let outer = [Value::array(&inner), Value::text("test")];
        let value = Value::array(&outer);

        let mut buf = [0u8; 32];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 9);
        assert_eq!(buf[0], 0x82); // array of 2 items
        assert_eq!(buf[1], 0x82); // array of 2 items
        assert_eq!(buf[2], 0x01); // unsigned 1
        assert_eq!(buf[3], 0x02); // unsigned 2
        assert_eq!(buf[4], 0x64); // text string of length 4
        assert_eq!(buf[5], b't');
        assert_eq!(buf[6], b'e');
        assert_eq!(buf[7], b's');
        assert_eq!(buf[8], b't');
    }

    // Test for tagged value encoding
    #[test]
    fn test_encode_tagged_value() {
        let inner = Value::text("2024-01-01T00:00:00Z");
        let value = Value::tag(0, &inner);

        let mut buf = [0u8; 32];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 22);
        assert_eq!(buf[0], 0xC0); // tag 0
        assert_eq!(buf[1], 0x74); // text string of length 20
        // The rest would be the UTF-8 encoding of the timestamp
    }

    // Test for float value encoding
    #[test]
    fn test_encode_float() {
        let value = Value::float(3.14159);

        let mut buf = [0u8; 16];
        let size = encode(&value, &mut buf).unwrap();

        assert_eq!(size, 9);
        assert_eq!(buf[0], 0xFB); // 0xE0 | 27 (IEEE 754 double-precision float)
        // The next 8 bytes are the IEEE 754 encoding of 3.14159
        let expected = 3.14159f64.to_bits().to_be_bytes();
        assert_eq!(&buf[1..9], &expected);
    }

    // Test buffer size errors
    #[test]
    fn test_encode_buffer_overflow() {
        let value = Value::unsigned(42);
        let mut buf = [0u8; 1]; // Too small for the encoding (needs 2 bytes)

        let result = encode(&value, &mut buf);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::BufferOverflow);
    }

    // Test for encoded size calculation
    #[test]
    fn test_encoded_size() {
        let v1 = Value::unsigned(10);
        assert_eq!(encoded_size(&v1), 1);

        let v2 = Value::unsigned(1000);
        assert_eq!(encoded_size(&v2), 3);

        let v3 = Value::text("hello");
        assert_eq!(encoded_size(&v3), 6); // 1 byte header + 5 bytes for "hello"

        let v4 = Value::bytes(&[0xFF; 10]);
        assert_eq!(encoded_size(&v4), 11); // 1 byte header + 10 bytes data
    }

    // Test for complex structure encoded size calculation
    #[test]
    fn test_complex_encoded_size() {
        let colors = [Value::text("blue"), Value::text("green")];
        let items = [
            (Value::text("name"), Value::text("test")),
            (Value::text("colors"), Value::array(&colors)),
        ];
        let map = Value::map(&items);

        let expected_size = 30; // Calculated by hand or from actual encoding
        assert_eq!(encoded_size(&map), expected_size);

        // Verify by actually encoding
        let mut buf = [0u8; 64];
        let actual_size = encode(&map, &mut buf).unwrap();
        assert_eq!(actual_size, expected_size);
    }
}
