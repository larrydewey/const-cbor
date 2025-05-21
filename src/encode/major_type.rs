//! CBOR major types as defined in RFC 7049.
//!
//! The major type is encoded in the high-order 3 bits of the first byte
//! of a data item, and indicates the basic type of the data item.

pub type MajorType = u8;

/// Major type 0: Unsigned integer (0..23, U8..U64)
pub const UNSIGNED: u8 = 0;
/// Major type 1: Negative integer (-1..-18446744073709551616)
pub const NEGATIVE: u8 = 1;
/// Major type 2: Byte string (0..2^64-1 bytes)
pub const BYTES: u8 = 2;
/// Major type 3: Text string (0..2^64-1 bytes)
pub const TEXT: u8 = 3;
/// Major type 4: Array of data items
pub const ARRAY: u8 = 4;
/// Major type 5: Map of pairs of data items
pub const MAP: u8 = 5;
/// Major type 6: Tagged data items
pub const TAG: u8 = 6;
/// Major type 7: Simple values, floating point, and special values
pub const SIMPLE: u8 = 7;
/// Major type 9: Floating point numbers
pub const FLOAT: u8 = 9;
