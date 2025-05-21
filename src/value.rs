// SPDX-License-Identifier: MIT

//! CBOR value representation.
//!
//! This module provides the `Value` enum, which represents all possible CBOR data types
//! as defined in [RFC 7049](https://tools.ietf.org/html/rfc7049).
//!
//! The CBOR format is designed to be lightweight, flexible, and extensible. It supports
//! all common data types such as integers, strings, arrays, and maps, as well as more
//! specialized types like tagged values and floating-point numbers.
//!
//! The `Value` enum in this module allows for building and manipulating CBOR data
//! structures in a memory-efficient and type-safe way.

type Array<'a> = &'a [Value<'a>];
type Map<'a> = &'a [(Value<'a>, Value<'a>)];

/// Represents a CBOR value that can be encoded or decoded.
///
/// The lifetime parameter `'a` allows the Value to reference data that it does not own,
/// making zero-copy operations possible. This is particularly useful in constrained
/// environments where memory allocation should be minimized.
///
/// Each variant corresponds to one of the major types defined in the CBOR specification,
/// providing a comprehensive representation of all possible CBOR values.
///
/// # Examples
///
/// ```
/// use const_cbor::Value;
///
/// // Create a simple string value
/// let text = Value::text("hello, world");
///
/// // Create a nested array
/// let array = Value::array(&[
///     Value::unsigned(1),
///     Value::text("text"),
///     Value::bool(true)
/// ]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value<'a> {
    /// Unsigned integer (major type 0).
    ///
    /// Represents positive integers from 0 to 18446744073709551615 (2^64-1).
    /// In CBOR, these are encoded with major type 0.
    Unsigned(u64),

    /// Negative integer (major type 1), stored as the absolute value.
    /// The actual value is -1 - n, where n is the stored value.
    ///
    /// Represents negative integers from -1 to -18446744073709551616 (-2^64).
    /// Following CBOR's encoding rules, the value stored is actually the absolute
    /// value minus 1 to avoid overlap with positive integers.
    Negative(u64),

    /// Byte string (major type 2).
    ///
    /// Represents a sequence of bytes. CBOR distinguishes between byte strings
    /// (which may contain arbitrary binary data) and text strings (which must
    /// be valid UTF-8).
    Bytes(&'a [u8]),

    /// UTF-8 text string (major type 3).
    ///
    /// Represents a string of valid UTF-8 characters. CBOR requires that
    /// text strings are encoded in UTF-8, ensuring compatibility across
    /// different systems.
    Text(&'a str),

    /// Array of CBOR data items (major type 4).
    ///
    /// Represents a sequence of CBOR values. Arrays can contain any
    /// CBOR data item, including other arrays, creating nested structures.
    Array(&'a [Value<'a>]),

    /// Map of pairs of CBOR data items (major type 5).
    ///
    /// Represents a collection of key-value pairs where both keys and values
    /// can be any valid CBOR data item. Keys are typically strings, but CBOR
    /// allows for any type to be used as a key.
    Map(&'a [(Value<'a>, Value<'a>)]),

    /// Tagged value (major type 6), consisting of a tag number and the tagged item.
    ///
    /// Represents a data item with a tag that indicates additional semantic meaning.
    /// For example, tag 0 indicates that the following item is a date/time string
    /// in RFC 3339 format.
    Tag(u64, &'a Value<'a>),

    /// Simple value (major type 7), including special values like true, false, null, and undefined.
    ///
    /// Represents values from a predefined set of simple values:
    /// - 20: false
    /// - 21: true
    /// - 22: null
    /// - 23: undefined
    /// - 24-31: reserved
    Simple(u8),

    /// IEEE 754 Double-Precision Float (major type 7).
    ///
    /// Represents a floating-point number following the IEEE 754-2008 standard.
    /// CBOR supports half-precision, single-precision, and double-precision floats,
    /// but this implementation uses double-precision (64-bit) for simplicity.
    Float(f64),
}

impl<'a> Value<'a> {
    /// Creates a CBOR null value (simple value 22).
    ///
    /// In CBOR, null is represented as a simple value with code 22.
    /// This is equivalent to JSON's `null` and can be used to indicate the absence of a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let null_value = Value::null();
    /// ```
    #[inline]
    pub const fn null() -> Self {
        Self::Simple(22)
    }

    /// Creates a CBOR boolean value (simple value 20 for false, 21 for true).
    ///
    /// In CBOR, booleans are represented as simple values with code 20 (false) or 21 (true).
    /// These values correspond to JSON's `false` and `true` values respectively.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let true_value = Value::bool(true);
    /// let false_value = Value::bool(false);
    /// ```
    #[inline]
    pub const fn bool(value: bool) -> Self {
        Self::Simple(if value { 21 } else { 20 })
    }

    /// Creates a CBOR unsigned integer value (major type 0).
    ///
    /// This can represent any positive integer from 0 to 2^64-1 (18,446,744,073,709,551,615).
    /// In CBOR encoding, unsigned integers use major type 0 and are encoded in the most
    /// compact form possible based on their magnitude.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let uint = Value::unsigned(42);
    /// ```
    #[inline]
    pub const fn unsigned(value: u64) -> Self {
        Self::Unsigned(value)
    }

    /// Creates a CBOR negative integer value (major type 1).
    ///
    /// The value is encoded following CBOR's negative integer representation,
    /// where the actual encoded value is -1 minus the stored value.
    /// This can represent any negative integer from -1 to -2^64 (-18,446,744,073,709,551,616).
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let neg = Value::negative(-10);
    /// ```
    #[inline]
    pub const fn negative(value: i64) -> Self {
        // CBOR encodes negative integers as -1 - n, where n is the stored value
        // So for a negative value like -10, we need to store 9 (|-10| - 1)
        Self::Negative((-(value + 1)) as u64)
    }

    /// Creates a CBOR byte string value (major type 2).
    ///
    /// Byte strings in CBOR can contain any sequence of bytes and are designed
    /// for efficiently representing binary data. Unlike text strings, byte strings
    /// have no encoding requirements and can store arbitrary octet sequences.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let bytes = Value::bytes(&[0x01, 0x02, 0x03]);
    /// ```
    #[inline]
    pub const fn bytes(value: &'a [u8]) -> Self {
        Self::Bytes(value)
    }

    /// Creates a CBOR text string value (major type 3).
    ///
    /// Text strings in CBOR must be valid UTF-8 encoded strings. This constructor
    /// takes a Rust string slice, which is guaranteed to be valid UTF-8, ensuring
    /// the resulting CBOR value is compliant with the specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let text = Value::text("Hello, world!");
    /// ```
    #[inline]
    pub const fn text(value: &'a str) -> Self {
        Self::Text(value)
    }

    /// Creates a CBOR array value (major type 4).
    ///
    /// CBOR arrays are ordered sequences of CBOR data items. Each item can be of any
    /// CBOR type, allowing for heterogeneous collections. Arrays in CBOR correspond
    /// to arrays in JSON and similar data formats.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let items = [Value::unsigned(1), Value::text("hello")];
    /// let array = Value::array(&items);
    /// ```
    #[inline]
    pub const fn array(value: Array<'a>) -> Self {
        Self::Array(value)
    }

    /// Creates a CBOR map value (major type 5).
    ///
    /// CBOR maps are collections of key-value pairs where both keys and values can be
    /// any CBOR data item. Maps correspond to objects in JSON but are more flexible,
    /// as keys are not restricted to strings. This implementation represents maps as
    /// slices of key-value tuples.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let pairs = [(Value::text("key"), Value::unsigned(42))];
    /// let map = Value::map(&pairs);
    /// ```
    #[inline]
    pub const fn map(value: Map<'a>) -> Self {
        Self::Map(value)
    }

    /// Creates a CBOR tagged value (major type 6).
    ///
    /// Tagged values in CBOR associate a tag number with a data item to indicate
    /// additional semantic meaning. For example, tag 0 indicates that the following
    /// item is a date/time string in RFC 3339 format. The CBOR specification defines
    /// several standard tags, and applications can define their own.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let datetime = Value::text("2024-01-01T00:00:00Z");
    /// // Tag 0 is standard datetime
    /// let tagged = Value::tag(0, &datetime);
    /// ```
    #[inline]
    pub const fn tag(tag: u64, item: &'a Value<'a>) -> Self {
        Self::Tag(tag, item)
    }

    /// Creates a CBOR floating point value (major type 7, additional info 27 for double precision).
    ///
    /// This constructor creates a double-precision (64-bit) floating-point number
    /// following the IEEE 754-2008 standard. In CBOR encoding, this uses major type 7
    /// with additional information value 27.
    ///
    /// # Examples
    ///
    /// ```
    /// use const_cbor::Value;
    ///
    /// let pi = Value::float(3.14159);
    /// ```
    #[inline]
    pub const fn float(value: f64) -> Self {
        Self::Float(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn test_null_construction() {
        let value = Value::null();
        match value {
            Value::Simple(22) => (),
            _ => panic!("Expected Value::Simple(22), got {:?}", value),
        }
    }

    #[test]
    fn test_bool_construction_true() {
        let value = Value::bool(true);
        match value {
            Value::Simple(21) => (),
            _ => panic!("Expected Value::Simple(21), got {:?}", value),
        }
    }

    #[test]
    fn test_bool_construction_false() {
        let value = Value::bool(false);
        match value {
            Value::Simple(20) => (),
            _ => panic!("Expected Value::Simple(20), got {:?}", value),
        }
    }

    #[test]
    fn test_unsigned_construction() {
        let value = Value::unsigned(42);
        match value {
            Value::Unsigned(42) => (),
            _ => panic!("Expected Value::Unsigned(42), got {:?}", value),
        }
    }

    #[test]
    fn test_negative_construction() {
        let value = Value::negative(-10);
        match value {
            Value::Negative(9) => (), // CBOR encodes -10 as 9
            _ => panic!("Expected Value::Negative(9), got {:?}", value),
        }
    }

    #[test]
    fn test_bytes_construction() {
        let bytes = [0x01, 0x02, 0x03];
        let value = Value::bytes(&bytes);
        match value {
            Value::Bytes(b) => {
                assert_eq!(b, &[0x01, 0x02, 0x03]);
            }
            _ => panic!("Expected Value::Bytes, got {:?}", value),
        }
    }

    #[test]
    fn test_text_construction() {
        let text = "Hello, world!";
        let value = Value::text(text);
        match value {
            Value::Text(t) => {
                assert_eq!(t, "Hello, world!");
            }
            _ => panic!("Expected Value::Text, got {:?}", value),
        }
    }

    #[test]
    fn test_array_construction() {
        let items = [Value::unsigned(1), Value::text("test")];
        let value = Value::array(&items);
        match value {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                match arr[0] {
                    Value::Unsigned(1) => (),
                    _ => panic!("Expected Value::Unsigned(1), got {:?}", arr[0]),
                }
                match arr[1] {
                    Value::Text(t) => assert_eq!(t, "test"),
                    _ => panic!("Expected Value::Text, got {:?}", arr[1]),
                }
            }
            _ => panic!("Expected Value::Array, got {:?}", value),
        }
    }

    #[test]
    fn test_map_construction() {
        let pairs = [(Value::text("key"), Value::unsigned(42))];
        let value = Value::map(&pairs);
        match value {
            Value::Map(m) => {
                assert_eq!(m.len(), 1);
                match m[0].0 {
                    Value::Text(k) => assert_eq!(k, "key"),
                    _ => panic!("Expected Value::Text, got {:?}", m[0].0),
                }
                match m[0].1 {
                    Value::Unsigned(v) => assert_eq!(v, 42),
                    _ => panic!("Expected Value::Unsigned, got {:?}", m[0].1),
                }
            }
            _ => panic!("Expected Value::Map, got {:?}", value),
        }
    }

    #[test]
    fn test_tag_construction() {
        let inner = Value::text("2024-01-01T00:00:00Z");
        let value = Value::tag(0, &inner);
        match value {
            Value::Tag(tag, val) => {
                assert_eq!(tag, 0);
                match val {
                    Value::Text(t) => assert_eq!(t, &"2024-01-01T00:00:00Z"),
                    _ => panic!("Expected Value::Text, got {:?}", val),
                }
            }
            _ => panic!("Expected Value::Tag, got {:?}", value),
        }
    }

    #[test]
    fn test_float_construction() {
        let value = Value::float(3.14159);
        match value {
            Value::Float(f) => {
                assert!((f - 3.14159).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Value::Float, got {:?}", value),
        }
    }
}
