//! Bignums (RFC 8949 §3.4.3), feature `bignum`.
//!
//! Tags 2 and 3 extend the generic data model with arbitrarily sized
//! integers. The magnitude travels as a byte string interpreted as an
//! unsigned integer in network byte order: tag 2 carries `n`, tag 3
//! carries `-1 - n`.
//!
//! Encoding normalizes to preferred serialization by stripping leading
//! zeroes (so magnitude `0` encodes as the empty byte string); decoding
//! accepts leading zeroes unconditionally, as the RFC requires.

use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::error::Error;
use crate::head::Major;

/// Tag number of an unsigned bignum (`n`).
pub const TAG_UNSIGNED: u64 = 2;
/// Tag number of a negative bignum (`-1 - n`).
pub const TAG_NEGATIVE: u64 = 3;

/// Strips leading zeroes, yielding the preferred-serialization magnitude.
///
/// Zero-valued input becomes an empty slice, matching RFC 8949 §3.4.3.
#[must_use]
pub fn normalized(magnitude_be: &[u8]) -> &[u8] {
    let first = magnitude_be.iter().position(|&b| b != 0);
    match first {
        Some(i) => &magnitude_be[i..],
        None => &[],
    }
}

impl Encoder<'_> {
    /// Encodes an unsigned bignum: tag 2 wrapping the big-endian magnitude.
    ///
    /// The magnitude is normalized to preferred serialization (leading
    /// zeroes stripped); any `u64` value passed here is deliberately kept
    /// as a bignum rather than rewritten to a basic integer, so callers
    /// can force the extended representation when needed.
    ///
    /// # Errors
    ///
    /// Propagates [`Error::OUT_OF_SPACE`] and encoding errors from the
    /// wrapped tag and byte string items.
    pub fn bignum_uint(&mut self, magnitude_be: &[u8]) -> Result<&mut Self, Error> {
        self.bignum(TAG_UNSIGNED, magnitude_be)
    }

    /// Encodes a negative bignum: tag 3 wrapping the big-endian magnitude,
    /// representing `-1 - n`.
    ///
    /// # Errors
    ///
    /// As [`bignum_uint`](Self::bignum_uint).
    pub fn bignum_nint(&mut self, magnitude_be: &[u8]) -> Result<&mut Self, Error> {
        self.bignum(TAG_NEGATIVE, magnitude_be)
    }

    fn bignum(&mut self, tag: u64, magnitude_be: &[u8]) -> Result<&mut Self, Error> {
        self.tag(tag)?;
        let magnitude = normalized(magnitude_be);
        // A failed payload write leaves the tag already emitted; mirror the
        // encoder's documented per-item atomicity by rolling back instead.
        let start = crate::head::head_len(tag);
        if self.bytes(magnitude).is_err() {
            self.pos -= start;
            return Err(Error::OUT_OF_SPACE);
        }
        Ok(self)
    }
}

impl<'a> Decoder<'a> {
    /// Decodes an unsigned bignum, borrowing its big-endian magnitude.
    ///
    /// Leading zeroes are accepted per RFC 8949 §3.4.3 and returned
    /// verbatim; use [`normalized`] to strip them.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] unless the next item is tag 2 wrapping a
    /// byte string.
    pub fn bignum_unsigned(&mut self) -> Result<&'a [u8], Error> {
        self.bignum_magnitude(TAG_UNSIGNED)
    }

    /// Decodes a negative bignum, borrowing its big-endian magnitude
    /// (value is `-1 - n`).
    ///
    /// # Errors
    ///
    /// As [`bignum_unsigned`](Self::bignum_unsigned), for tag 3.
    pub fn bignum_nint(&mut self) -> Result<&'a [u8], Error> {
        self.bignum_magnitude(TAG_NEGATIVE)
    }

    fn bignum_magnitude(&mut self, want: u64) -> Result<&'a [u8], Error> {
        let tag = self.tag()?;
        match tag == want {
            true => {}
            false => return Err(Error::TYPE_MISMATCH),
        }
        // Any contained item other than a byte string is invalid.
        self.bytes_of(Major::Bytes)
    }

    /// Decodes an unsigned bignum into `u64`.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] on wrong tag or non-byte-string content;
    /// [`Error::INTEGER_OUT_OF_RANGE`] when the magnitude exceeds
    /// `u64::MAX`.
    pub fn bignum_u64(&mut self) -> Result<u64, Error> {
        let mag = self.bignum_unsigned()?;
        be_to_u64(mag).ok_or(Error::INTEGER_OUT_OF_RANGE)
    }

    /// Decodes either bignum tag into `i128`.
    ///
    /// This is strictly the bignum representation: basic integers are not
    /// accepted here, so protocols can distinguish representations.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] on non-bignum items or non-byte-string
    /// content; [`Error::INTEGER_OUT_OF_RANGE`] outside the `i128` range.
    pub fn bignum_i128(&mut self) -> Result<i128, Error> {
        const MAX_MAGNITUDE_BITS: usize = 127;
        let tag = self.tag()?;
        let mag = self.bytes_of(Major::Bytes)?;
        // Reject magnitudes wider than i128 before shifting.
        let Some(&first) = normalized(mag).first() else {
            return Ok(match tag {
                TAG_UNSIGNED => 0,
                _ => -1,
            });
        };
        let bits = (mag.len() - 1) * 8 + (8 - first.leading_zeros() as usize);
        if bits > MAX_MAGNITUDE_BITS {
            return Err(Error::INTEGER_OUT_OF_RANGE);
        }
        let mut n: u128 = 0;
        for &b in mag {
            n = (n << 8) | u128::from(b);
        }
        match tag {
            TAG_UNSIGNED => Ok(n as i128),
            _ => Ok(-1 - n as i128),
        }
    }
}

/// Interprets a big-endian slice as `u64`, refusing overflow.
fn be_to_u64(bytes: &[u8]) -> Option<u64> {
    let trimmed = normalized(bytes);
    match trimmed.len() {
        0 => Some(0),
        n @ 1..=8 => {
            let mut v: u64 = 0;
            for &b in bytes.iter().rev().take(n) {
                v = (v << 8) | u64::from(b);
            }
            Some(v)
        }
        _ => None,
    }
}
