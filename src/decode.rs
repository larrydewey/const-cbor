//! CBOR decoding from borrowed input.
//!
//! [`Decoder`] walks a `&[u8]` input and exposes one method per CBOR type.
//! Text strings and byte strings are returned as borrows of the input —
//! no copying, no allocation. All methods return typed results; malformed
//! input never panics.

use crate::error::Error;
use crate::head::{Major, Simple, read_head};

/// Maximum container nesting tolerated by [`Decoder::skip`].
pub const SKIP_LIMIT: usize = 32;

/// Deserializes CBOR items from a fixed input slice.
#[derive(Clone, Copy, Debug)]
pub struct Decoder<'a> {
    pub(crate) input: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Decoder<'a> {
    /// Creates a decoder reading from the start of `input`.
    #[inline]
    #[must_use]
    pub const fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Number of bytes consumed so far.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Bytes left unread.
    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    /// `true` when every input byte has been consumed.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Major type of the next item without consuming it.
    #[inline]
    #[must_use]
    pub fn peek_major(&self) -> Option<Major> {
        self.input.get(self.pos).map(|&b| Major::from_initial(b))
    }

    /// `true` when the next byte is the break marker terminating an
    /// indefinite-length item (RFC 8949 §3.2); does not consume anything.
    #[inline]
    #[must_use]
    pub fn peek_break(&self) -> bool {
        matches!(self.input.get(self.pos), Some(&b) if crate::head::is_break(b))
    }

    /// Raw initial byte of the next item, if any input remains; does not
    /// consume anything. Lets callers inspect the additional-information
    /// field (`byte & 0x1f`) before committing to a typed getter.
    #[inline]
    #[must_use]
    pub fn peek_head_byte(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    /// Consumes the header of an indefinite-length item of major type
    /// `want` (RFC 8949 §3.2), i.e. an initial byte with additional
    /// information 31.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] unless the next item matches; structural
    /// errors propagate.
    #[cfg(any(feature = "indefinite", feature = "diagnostic"))]
    pub fn expect_indef(&mut self, want: Major) -> Result<(), Error> {
        let (major, _, indef) = self.next_head_ext()?;
        match major == want && indef {
            true => Ok(()),
            false => Err(Error::TYPE_MISMATCH),
        }
    }

    /// Consumes a break marker if present; reports whether one was there.
    ///
    /// # Errors
    ///
    /// [`Error::UNEXPECTED_BREAK`] is never produced here; the `Err` half
    /// exists for API symmetry and currently cannot occur.
    #[inline]
    pub fn take_break(&mut self) -> Result<bool, Error> {
        if self.peek_break() {
            self.pos += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Reads and consumes the next head.
    #[inline]
    fn next_head(&mut self) -> Result<(Major, u64), Error> {
        let (major, arg, used) = read_head(&self.input[self.pos..])?;
        self.pos += used;
        Ok((major, arg))
    }

    /// Reads the next head, allowing indefinite lengths (used by the
    /// `indefinite` and `diagnostic` features).
    #[cfg(any(feature = "indefinite", feature = "diagnostic"))]
    #[inline]
    pub(crate) fn next_head_ext(&mut self) -> Result<(Major, u64, bool), Error> {
        let (major, arg, used, indef) = crate::head::read_head_ext(&self.input[self.pos..])?;
        self.pos += used;
        Ok((major, arg, indef))
    }

    /// Requires the next item to be major type `want`, returning its argument.
    #[inline]
    fn expect_head(&mut self, want: Major) -> Result<u64, Error> {
        let (major, arg) = self.next_head()?;
        match major == want {
            true => Ok(arg),
            false => Err(Error::TYPE_MISMATCH),
        }
    }

    /// Converts an argument into a `usize` content length within input bounds.
    #[inline]
    pub(crate) fn content_len(&self, arg: u64) -> Result<usize, Error> {
        let len = usize::try_from(arg).map_err(|_| Error::TRUNCATED)?;
        match len <= self.remaining() {
            true => Ok(len),
            false => Err(Error::TRUNCATED),
        }
    }

    /// Decodes an unsigned integer, failing if it exceeds `T::MAX`.
    #[inline]
    fn uint_as<T: TryFrom<u64>>(&mut self) -> Result<T, Error> {
        let v = self.expect_head(Major::Unsigned)?;
        T::try_from(v).map_err(|_| Error::INTEGER_OUT_OF_RANGE)
    }

    /// Decodes a signed integer (`-1 - n` mapping), range-checked.
    #[inline]
    fn int_as<T: TryFrom<i64>>(&mut self) -> Result<T, Error> {
        match self.peek_major() {
            Some(Major::Unsigned) => {
                let v = self.uint_as::<i64>()?;
                T::try_from(v).map_err(|_| Error::INTEGER_OUT_OF_RANGE)
            }
            _ => {
                let n = self.expect_head(Major::Negative)?;
                // `-1 - n >= i64::MIN` requires n <= i64::MAX.
                match i64::try_from(n).is_ok() {
                    true => T::try_from(-1 - n as i64).map_err(|_| Error::INTEGER_OUT_OF_RANGE),
                    false => Err(Error::INTEGER_OUT_OF_RANGE),
                }
            }
        }
    }

    /// Decodes an unsigned integer that fits in `u8`.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] for non-integers; [`Error::INTEGER_OUT_OF_RANGE`]
    /// when the value exceeds `u8::MAX`.
    #[inline]
    pub fn u8(&mut self) -> Result<u8, Error> {
        self.uint_as()
    }

    /// Decodes an unsigned integer that fits in `u16`.
    ///
    /// # Errors
    ///
    /// As [`Decoder::u8`], with the wider target range.
    #[inline]
    pub fn u16(&mut self) -> Result<u16, Error> {
        self.uint_as()
    }

    /// Decodes an unsigned integer that fits in `u32`.
    ///
    /// # Errors
    ///
    /// As [`Decoder::u8`], with the wider target range.
    #[inline]
    pub fn u32(&mut self) -> Result<u32, Error> {
        self.uint_as()
    }

    /// Decodes any CBOR unsigned integer into `u64`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] for other major types.
    #[inline]
    pub fn u64(&mut self) -> Result<u64, Error> {
        self.expect_head(Major::Unsigned)
    }

    /// Decodes a negative or non-negative integer into `i8`.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] for non-integers;
    /// [`Error::INTEGER_OUT_OF_RANGE`] outside the `i8` range.
    #[inline]
    pub fn i8(&mut self) -> Result<i8, Error> {
        self.int_as()
    }

    /// Decodes a signed integer into `i16`.
    ///
    /// # Errors
    ///
    /// As [`Decoder::i8`], with the wider target range.
    #[inline]
    pub fn i16(&mut self) -> Result<i16, Error> {
        self.int_as()
    }

    /// Decodes a signed integer into `i32`.
    ///
    /// # Errors
    ///
    /// As [`Decoder::i8`], with the wider target range.
    #[inline]
    pub fn i32(&mut self) -> Result<i32, Error> {
        self.int_as()
    }

    /// Decodes any CBOR integer (major types 0 and 1) into `i64`.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] for non-integers;
    /// [`Error::INTEGER_OUT_OF_RANGE`] below `i64::MIN`.
    #[inline]
    pub fn i64(&mut self) -> Result<i64, Error> {
        self.int_as()
    }

    /// Borrows a byte string's contents from the input.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] if not a byte string; [`Error::TRUNCATED`]
    /// if the declared length exceeds the input.
    #[inline]
    pub fn bytes(&mut self) -> Result<&'a [u8], Error> {
        self.bytes_of(Major::Bytes)
    }

    /// Borrows a text string from the input after validating UTF-8.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] if not a text string; [`Error::INVALID_UTF8`]
    /// if the payload violates RFC 3629; [`Error::TRUNCATED`] on short input.
    #[inline]
    pub fn str(&mut self) -> Result<&'a str, Error> {
        let raw = self.bytes_of(Major::Text)?;
        core::str::from_utf8(raw).map_err(|_| Error::INVALID_UTF8)
    }

    #[inline]
    pub(crate) fn bytes_of(&mut self, major: Major) -> Result<&'a [u8], Error> {
        let arg = self.expect_head(major)?;
        let len = self.content_len(arg)?;
        let start = self.pos;
        self.pos += len;
        Ok(&self.input[start..start + len])
    }

    /// Decodes `true` or `false`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] for any other item.
    #[inline]
    pub fn bool(&mut self) -> Result<bool, Error> {
        let s = self.simple_value()?;
        match s.0 {
            20 => Ok(false),
            21 => Ok(true),
            _ => Err(Error::TYPE_MISMATCH),
        }
    }

    /// Decodes `null`; returns whether the value was null without consuming
    /// anything else. Use [`Decoder::peek_major`] for generic lookahead.
    ///
    /// # Errors
    ///
    /// Propagates decode errors from malformed heads only.
    #[inline]
    pub fn is_null(&mut self) -> Result<bool, Error> {
        match self.peek_simple() {
            Some(s) if s.0 == Simple::NULL.0 => {
                self.next_head()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Decodes `undefined`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] for any other item.
    #[inline]
    pub fn undefined(&mut self) -> Result<(), Error> {
        let s = self.simple_value()?;
        match s.0 == Simple::UNDEFINED.0 {
            true => Ok(()),
            false => Err(Error::TYPE_MISMATCH),
        }
    }

    /// Decodes any simple value (including booleans and null).
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] for floats and non-major-7 items;
    /// [`Error::INVALID_SIMPLE`] for two-byte encodings of values below 32.
    #[inline]
    pub fn simple(&mut self) -> Result<Simple, Error> {
        self.simple_value()
    }

    #[inline]
    fn peek_simple(&self) -> Option<Simple> {
        match self.input.get(self.pos) {
            Some(&b) if b >> 5 == 7 && b & 0x1f < 24 => Some(Simple(b & 0x1f)),
            _ => None,
        }
    }

    #[inline]
    fn simple_value(&mut self) -> Result<Simple, Error> {
        // Distinguish one-byte from two-byte form: values below 32 are
        // forbidden in the two-byte encoding (RFC 8949 §3.3).
        let two_byte = matches!(
            self.input.get(self.pos),
            Some(b) if b >> 5 == 7 && (b & 0x1f) == 24
        );
        // Floating-point widths (ai 25-27) carry IEEE 754 payloads, never
        // simple values (RFC 8949 §3.3); the argument bits must not be
        // reinterpreted as a simple value.
        if matches!(
            self.input.get(self.pos),
            Some(b) if b >> 5 == 7 && matches!(b & 0x1f, 25..=27)
        ) {
            self.next_head()?;
            return Err(Error::FLOAT_UNSUPPORTED);
        }
        let (major, arg) = self.next_head()?;
        if major != Major::Simple {
            return Err(Error::TYPE_MISMATCH);
        }
        match (arg, two_byte) {
            (0..=23, false) | (32..=255, true) => Ok(Simple(arg as u8)),
            (_, true) => Err(Error::INVALID_SIMPLE),
            // ai 25–27 carry floating-point payloads, not simple values.
            _ => Err(Error::FLOAT_UNSUPPORTED),
        }
    }

    /// Reads a tag number; the tagged item follows immediately.
    ///
    /// Tag semantics are application concerns (RFC 8949 §3.4); this returns
    /// only the number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] if the item is not a tag.
    #[inline]
    pub fn tag(&mut self) -> Result<u64, Error> {
        self.expect_head(Major::Tag)
    }

    /// Reads an array header, returning the number of following items.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] if the item is not an array.
    #[inline]
    pub fn array_len(&mut self) -> Result<usize, Error> {
        let n = self.expect_head(Major::Array)?;
        usize::try_from(n).map_err(|_| Error::INTEGER_OUT_OF_RANGE)
    }

    /// Reads a map header, returning the number of following key/value pairs.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TYPE_MISMATCH`] if the item is not a map.
    #[inline]
    pub fn map_len(&mut self) -> Result<usize, Error> {
        let n = self.expect_head(Major::Map)?;
        usize::try_from(n).map_err(|_| Error::INTEGER_OUT_OF_RANGE)
    }

    /// Consumes exactly one complete data item, whatever its type.
    ///
    /// Nested containers are traversed iteratively up to [`SKIP_LIMIT`]
    /// levels deep; deeper input yields [`Error::DEPTH_EXCEEDED`].
    ///
    /// # Errors
    ///
    /// Any structural error ([`Error::TRUNCATED`],
    /// [`Error::INVALID_ADDITIONAL_INFO`], [`Error::INVALID_SIMPLE`],
    /// [`Error::DEPTH_EXCEEDED`]).
    pub fn skip(&mut self) -> Result<(), Error> {
        // A virtual root count of 1 makes loop termination uniform. The
        // sentinel marks an open-ended (indefinite) scope that closes on a
        // break byte instead of a counter reaching zero.
        //
        // Counters are u32: a container declaring more than `u32::MAX`
        // items hits `TRUNCATED` long before the counter could matter
        // (each item occupies at least one byte), so narrowing the counter
        // cannot change any outcome — and halves the stack's zeroing cost.
        type Count = u32;
        const OPEN: Count = Count::MAX;
        const CLAMP: u64 = (Count::MAX - 1) as u64;
        let mut stack = [0u32; SKIP_LIMIT];
        stack[0] = 1;
        let mut sp = 1usize;
        loop {
            match self.input.get(self.pos) {
                None => return Err(Error::TRUNCATED),
                Some(&b) if crate::head::is_break(b) => {
                    self.pos += 1;
                    match sp > 1 && stack[sp - 1] == OPEN {
                        true => sp -= 1,
                        false => return Err(Error::UNEXPECTED_BREAK),
                    }
                }
                Some(_) => {
                    let (major, arg, indef) = {
                        #[cfg(feature = "indefinite")]
                        {
                            self.next_head_ext()?
                        }
                        #[cfg(not(feature = "indefinite"))]
                        {
                            let (m, a) = self.next_head()?;
                            (m, a, false)
                        }
                    };
                    if indef {
                        #[cfg(not(feature = "indefinite"))]
                        return Err(Error::INDEFINITE_UNSUPPORTED);
                        #[cfg(feature = "indefinite")]
                        match major {
                            Major::Bytes | Major::Text => self.skip_chunks(major)?,
                            Major::Array | Major::Map => {
                                sp = Self::push(&mut stack, sp, OPEN)?;
                                continue;
                            }
                            _ => return Err(Error::INVALID_ADDITIONAL_INFO),
                        }
                    }
                    match major {
                        // A tag wraps exactly one more item (RFC 8949
                        // section 3.4): defer the counter decrement by
                        // looping straight to the next item, so the
                        // tagged payload is consumed as part of this one.
                        Major::Tag => continue,
                        Major::Unsigned | Major::Negative | Major::Simple => {}
                        Major::Bytes | Major::Text => {
                            let len = self.content_len(arg)?;
                            self.pos += len;
                        }
                        Major::Array | Major::Map => {
                            let items = match major {
                                Major::Map => arg.saturating_mul(2),
                                _ => arg,
                            };
                            // Clamp before narrowing: see the u32 note on
                            // the stack declaration above.
                            if items > 0 {
                                sp = Self::push(&mut stack, sp, items.min(CLAMP) as Count)?;
                                continue;
                            }
                        }
                    }
                }
            }
            // Closed item or closed scope: unwind completed scopes.
            while sp > 0 {
                if stack[sp - 1] == OPEN {
                    break; // waits for its break byte
                }
                let top = &mut stack[sp - 1];
                *top -= 1;
                if *top != 0 {
                    break;
                }
                sp -= 1;
            }
            if sp == 0 {
                return Ok(());
            }
        }
    }

    /// Consumes definite chunks of an indefinite string until its break.
    #[cfg(feature = "indefinite")]
    fn skip_chunks(&mut self, major: Major) -> Result<(), Error> {
        loop {
            match self.input.get(self.pos) {
                None => return Err(Error::TRUNCATED),
                Some(&b) if crate::head::is_break(b) => {
                    self.pos += 1;
                    return Ok(());
                }
                Some(_) => {
                    let arg = self.expect_head(major)?;
                    let len = self.content_len(arg)?;
                    self.pos += len;
                }
            }
        }
    }

    #[cold]
    #[inline(never)]
    fn push(stack: &mut [u32; SKIP_LIMIT], sp: usize, count: u32) -> Result<usize, Error> {
        match sp < SKIP_LIMIT {
            true => {
                stack[sp] = count;
                Ok(sp + 1)
            }
            false => Err(Error::DEPTH_EXCEEDED),
        }
    }
}

#[cfg(feature = "float")]
mod float_impls {
    use super::{Decoder, Error};
    use crate::float::f32_of_f16_bits;
    use crate::head::{Major, ext_len};

    impl Decoder<'_> {
        /// Decodes a half-, single-, or double-precision float into `f32`,
        /// widening as needed.
        ///
        /// # Errors
        ///
        /// [`Error::TYPE_MISMATCH`] for non-float items.
        pub fn f32(&mut self) -> Result<f32, Error> {
            match self.float_bits()? {
                (2, h) => Ok(f32_of_f16_bits(h as u16)),
                (4, s) => Ok(f32::from_bits(s as u32)),
                _ => Err(Error::TYPE_MISMATCH),
            }
        }

        /// Decodes any float width into `f64`.
        ///
        /// # Errors
        ///
        /// As [`Decoder::f32`].
        pub fn f64(&mut self) -> Result<f64, Error> {
            match self.float_bits()? {
                (2, h) => Ok(f64::from(f32_of_f16_bits(h as u16))),
                (4, s) => Ok(f64::from(f32::from_bits(s as u32))),
                (_, d) => Ok(f64::from_bits(d)),
            }
        }

        /// Reads a float item, returning `(argument_bytes, payload_bits)`.
        fn float_bits(&mut self) -> Result<(usize, u64), Error> {
            let pos = self.pos;
            let (major, arg) = self.next_head()?;
            if major != Major::Simple {
                return Err(Error::TYPE_MISMATCH);
            }
            let ai = self.input[pos] & 0x1f;
            match ai {
                25..=27 => Ok((ext_len(ai), arg)),
                _ => Err(Error::TYPE_MISMATCH),
            }
        }
    }
}
