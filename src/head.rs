//! The CBOR head: initial byte, major types, and argument encoding.
//!
//! Every CBOR data item starts with an initial byte whose top three bits
//! select the major type and whose low five bits are the *additional
//! information* (`ai`). Values of `ai` below 24 encode the argument inline;
//! 24–27 indicate that the argument follows in 1, 2, 4, or 8 bytes; 28–30
//! are reserved; 31 signals indefinite length (not supported by this crate).
//!
//! All functions here are `const fn`, so head construction and size
//! computation can be evaluated at compile time.

use crate::error::Error;

/// Major type of a CBOR data item (RFC 8949 §3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Major {
    /// Unsigned integer (`n >= 0`).
    Unsigned = 0,
    /// Negative integer (`-1 - n`).
    Negative = 1,
    /// Byte string.
    Bytes = 2,
    /// Text string.
    Text = 3,
    /// Array of data items.
    Array = 4,
    /// Map of key/value pairs.
    Map = 5,
    /// Tagged item (semantic tagging).
    Tag = 6,
    /// Simple values and floating-point numbers.
    Simple = 7,
}

impl Major {
    /// Extracts the major type from an initial byte.
    #[inline(always)]
    #[must_use]
    pub const fn from_initial(initial: u8) -> Self {
        match initial >> 5 {
            0 => Self::Unsigned,
            1 => Self::Negative,
            2 => Self::Bytes,
            3 => Self::Text,
            4 => Self::Array,
            5 => Self::Map,
            6 => Self::Tag,
            _ => Self::Simple,
        }
    }

    /// The three-bit major-type field of the initial byte.
    #[inline(always)]
    #[must_use]
    pub const fn bits(self) -> u8 {
        (self as u8) << 5
    }
}

/// A major type 7 simple value (RFC 8949 §3.3), excluding floats.
///
/// Well-known simple values have associated constants; any other value in
/// `0..=255` can be wrapped directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Simple(pub u8);

impl Simple {
    /// Simple value `false`.
    pub const FALSE: Self = Self(20);
    /// Simple value `true`.
    pub const TRUE: Self = Self(21);
    /// Simple value `null`.
    pub const NULL: Self = Self(22);
    /// Simple value `undefined`.
    pub const UNDEFINED: Self = Self(23);

    /// The two-byte simple-value form is only legal for values >= 32;
    /// values below 32 must be encoded in the initial byte itself.
    #[inline(always)]
    #[must_use]
    pub const fn valid_two_byte(self) -> bool {
        self.0 >= 32
    }
}

/// Number of bytes an argument of this width adds after the initial byte.
#[inline(always)]
#[must_use]
#[allow(clippy::unreadable_literal)]
pub const fn ext_len(ai: u8) -> usize {
    match ai {
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => 0,
    }
}

/// Preferred additional-information selector for a `u64` argument.
#[inline(always)]
#[must_use]
pub const fn ai_of(v: u64) -> u8 {
    if v < 24 {
        v as u8
    } else if v <= 0xff {
        24
    } else if v <= 0xffff {
        25
    } else if v <= 0xffff_ffff {
        26
    } else {
        27
    }
}

/// Total encoded length of a head carrying argument `v`
/// (initial byte plus any extension bytes).
#[inline(always)]
#[must_use]
pub const fn head_len(v: u64) -> usize {
    1 + ext_len(ai_of(v))
}

/// Encoded length of the unsigned integer item `v` (preferred form).
#[inline(always)]
#[must_use]
pub const fn uint_len(v: u64) -> usize {
    head_len(v)
}

/// Encoded length of the negative integer item `-1 - n` (preferred form).
#[inline(always)]
#[must_use]
pub const fn neg_len(n: u64) -> usize {
    uint_len(n)
}

/// Writes a complete head into `buf` and returns the number of bytes written.
///
/// # Errors
///
/// Returns [`Error::OUT_OF_SPACE`] when `buf` is shorter than
/// [`head_len`] for this argument.
#[inline]
pub const fn write_head(buf: &mut [u8], major: Major, arg: u64) -> Result<usize, Error> {
    let ai = ai_of(arg);
    let ext = ext_len(ai);
    if buf.len() < 1 + ext {
        return Err(Error::OUT_OF_SPACE);
    }
    buf[0] = major.bits() | ai;
    let mut i = 0;
    while i < ext {
        // Big-endian argument bytes; shift amount is bounded by `ext`.
        buf[1 + i] = (arg >> ((ext - 1 - i) * 8)) as u8;
        i += 1;
    }
    Ok(1 + ext)
}

/// Writes a head with an *exact* additional-information selector into `buf`
/// and returns the number of bytes written.
///
/// Unlike [`write_head`] this never chooses a narrower encoding; it is used
/// for major type 7 floating-point items, whose width must be explicit.
/// `ai` must be 25, 26, or 27.
#[cfg_attr(not(feature = "float"), allow(dead_code))]
#[inline]
pub(crate) const fn write_head_exact(buf: &mut [u8], arg: u64, ai: u8) -> Result<usize, Error> {
    match ai {
        25..=27 => {}
        _ => return Err(Error::INVALID_ADDITIONAL_INFO),
    }
    let ext = ext_len(ai);
    if buf.len() < 1 + ext {
        return Err(Error::OUT_OF_SPACE);
    }
    buf[0] = Major::Simple.bits() | ai;
    let mut i = 0;
    while i < ext {
        buf[1 + i] = (arg >> ((ext - 1 - i) * 8)) as u8;
        i += 1;
    }
    Ok(1 + ext)
}

/// The break byte terminating an indefinite-length item (RFC 8949 §3.2).
pub const BREAK: u8 = 0xff;

/// `true` when `initial` is the break byte.
#[inline(always)]
#[must_use]
pub const fn is_break(initial: u8) -> bool {
    initial == BREAK
}

/// Parses a head from `input`, returning `(major, argument, bytes_read, indefinite)`.
///
/// `indefinite` is `true` for ai = 31 (open-ended items); the argument is 0.
/// Reserved ai values 28–30 remain rejected. Non-minimal argument encodings
/// are accepted: RFC 8949 §4.2.1 makes preferred serialization a SHOULD,
/// not a well-formedness requirement.
#[inline]
pub(crate) fn read_head_ext(input: &[u8]) -> Result<(Major, u64, usize, bool), Error> {
    let Some(&initial) = input.first() else {
        return Err(Error::TRUNCATED);
    };
    let major = Major::from_initial(initial);
    let ai = initial & 0x1f;
    match ai {
        0..=23 => Ok((major, u64::from(ai), 1, false)),
        24..=27 => {
            let len = ext_len(ai);
            let end = len.checked_add(1).ok_or(Error::INVALID_ADDITIONAL_INFO)?;
            if input.len() < end {
                return Err(Error::TRUNCATED);
            }
            let mut arg: u64 = 0;
            let mut i = 0;
            while i < len {
                arg = (arg << 8) | u64::from(input[1 + i]);
                i += 1;
            }
            Ok((major, arg, end, false))
        }
        28..=30 => Err(Error::INVALID_ADDITIONAL_INFO),
        _ => Ok((major, 0, 1, true)),
    }
}

/// Parses a head, rejecting indefinite lengths (ai = 31).
#[inline]
pub(crate) fn read_head(input: &[u8]) -> Result<(Major, u64, usize), Error> {
    let (major, arg, used, indef) = read_head_ext(input)?;
    match indef {
        true => Err(Error::INDEFINITE_UNSUPPORTED),
        false => Ok((major, arg, used)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Major, Simple, ai_of, ext_len, head_len, read_head, write_head};

    #[test]
    fn preferred_argument_widths() {
        assert_eq!(ai_of(0), 0);
        assert_eq!(ai_of(23), 23);
        assert_eq!(ai_of(24), 24);
        assert_eq!(ai_of(0xff), 24);
        assert_eq!(ai_of(0x100), 25);
        assert_eq!(ai_of(0xffff), 25);
        assert_eq!(ai_of(0x1_0000), 26);
        assert_eq!(ai_of(0xffff_ffff), 26);
        assert_eq!(ai_of(0x1_0000_0000), 27);
        assert_eq!(ai_of(u64::MAX), 27);
        assert_eq!((head_len(23), head_len(24)), (1, 2));
        assert_eq!(head_len(u64::MAX), 9);
        assert_eq!(ext_len(31), 0);
    }

    #[test]
    fn head_roundtrip() {
        let mut buf = [0u8; 9];
        for arg in [0u64, 23, 24, 255, 256, 65_535, 65_536, u64::from(u32::MAX), u64::MAX] {
            let n = write_head(&mut buf, Major::Unsigned, arg).unwrap();
            let (major, parsed, used) = read_head(&buf).unwrap();
            assert_eq!(major, Major::Unsigned);
            assert_eq!((parsed, used), (arg, n));
        }
    }

    #[test]
    fn head_out_of_space() {
        let mut buf = [0u8; 1];
        assert_eq!(write_head(&mut buf, Major::Map, 300), Err(crate::Error::OUT_OF_SPACE));
    }

    #[test]
    fn major_dispatch() {
        assert_eq!(Major::from_initial(0x18), Major::Unsigned);
        assert_eq!(Major::from_initial(0x39), Major::Negative);
        assert_eq!(Major::from_initial(0xf4), Major::Simple);
        assert_eq!(Major::from_initial(0xff), Major::Simple);
    }

    #[test]
    fn simple_two_byte_validity() {
        assert!(!Simple(31).valid_two_byte());
        assert!(Simple(32).valid_two_byte());
    }
}
