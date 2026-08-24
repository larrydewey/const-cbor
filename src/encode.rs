//! CBOR encoding into caller-provided buffers.
//!
//! [`Encoder`] advances through a fixed `&mut [u8]` output and exposes one
//! method per CBOR type. Methods return `Result<&mut Self, _>` so calls
//! chain naturally; nothing allocates and nothing panics at runtime.
//!
//! The submodule [`consts`] mirrors the encoder with `const fn`s that
//! report insufficient space by panic, making whole encodings evaluable at
//! compile time.

use crate::error::Error;
use crate::head::{Major, Simple, write_head};

/// Serializes CBOR items into a fixed buffer.
///
/// On success each method advances the cursor by exactly the item's
/// preferred-encoding length. On failure ([`Error::OUT_OF_SPACE`]) nothing
/// is written for that item, so the encoder stays valid and reusable.
#[derive(Debug)]
pub struct Encoder<'b> {
    pub(crate) buf: &'b mut [u8],
    pub(crate) pos: usize,
    /// Open indefinite scopes, innermost last (`indefinite` feature).
    #[cfg(feature = "indefinite")]
    pub(crate) indef: [u8; 16],
    #[cfg(feature = "indefinite")]
    pub(crate) indef_sp: u8,
}

impl<'b> Encoder<'b> {
    /// Creates an encoder writing into `buf`, starting at offset 0.
    #[inline]
    #[must_use]
    pub const fn new(buf: &'b mut [u8]) -> Self {
        {
            #[cfg(feature = "indefinite")]
            let this = Self { buf, pos: 0, indef: [0; 16], indef_sp: 0 };
            #[cfg(not(feature = "indefinite"))]
            let this = Self { buf, pos: 0 };
            this
        }
    }

    /// Number of bytes successfully written so far.
    #[inline]
    #[must_use]
    pub const fn written(&self) -> usize {
        self.pos
    }

    /// Number of bytes still available for writing.
    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    /// Consumes the encoder, returning the buffer and the bytes written.
    #[inline]
    #[must_use]
    pub const fn finish(self) -> (&'b [u8], usize) {
        (self.buf, self.pos)
    }

    /// Fails unless `n` more bytes fit.
    #[inline]
    pub(crate) const fn reserve(&self, n: usize) -> Result<(), Error> {
        match n > self.remaining() {
            true => Err(Error::OUT_OF_SPACE),
            false => Ok(()),
        }
    }

    /// Writes a head with a preferred-width argument.
    #[inline]
    pub(crate) fn put_head(&mut self, major: Major, arg: u64) -> Result<(), Error> {
        let n = write_head(self.tail(), major, arg)?;
        self.pos += n;
        Ok(())
    }

    #[inline]
    fn tail(&mut self) -> &mut [u8] {
        &mut self.buf[self.pos..]
    }

    /// Writes a length-prefixed payload (byte/text string content).
    #[inline]
    pub(crate) fn put_payload(&mut self, major: Major, payload: &[u8]) -> Result<&mut Self, Error> {
        let len = u64::try_from(payload.len()).map_err(|_| Error::OUT_OF_SPACE)?;
        self.reserve(crate::head::head_len(len) + payload.len())?;
        let head = crate::head::head_len(len);
        let _ = write_head(self.tail(), major, len)?;
        self.pos += head;
        self.copy_payload(payload);
        Ok(self)
    }

    #[inline]
    fn copy_payload(&mut self, payload: &[u8]) {
        let end = self.pos + payload.len();
        self.buf[self.pos..end].copy_from_slice(payload);
        self.pos = end;
    }

    /// Encodes any unsigned integer using its preferred width.
    ///
    /// Prefer the typed helpers (`u8`, `u16`, …); they compile to
    /// identical code and document the value's range.
    #[inline]
    pub fn uint(&mut self, v: u64) -> Result<&mut Self, Error> {
        self.put_head(Major::Unsigned, v)?;
        Ok(self)
    }

    /// Encodes `-1 - n` for magnitude `n` (RFC 8949 §3.1).
    ///
    /// Magnitudes above `i64::MIN`'s range remain representable in CBOR,
    /// so this accepts the full `u64` range.
    #[inline]
    pub fn nint(&mut self, n: u64) -> Result<&mut Self, Error> {
        self.put_head(Major::Negative, n)?;
        Ok(self)
    }

    /// Encodes a signed integer.
    #[inline]
    pub fn i64(&mut self, v: i64) -> Result<&mut Self, Error> {
        match v >= 0 {
            true => self.uint(v as u64),
            false => self.nint(!(v as u64)),
        }
    }

    /// Encodes `v`.
    #[inline]
    pub fn u8(&mut self, v: u8) -> Result<&mut Self, Error> {
        self.uint(u64::from(v))
    }

    /// Encodes `v`.
    #[inline]
    pub fn u16(&mut self, v: u16) -> Result<&mut Self, Error> {
        self.uint(u64::from(v))
    }

    /// Encodes `v`.
    #[inline]
    pub fn u32(&mut self, v: u32) -> Result<&mut Self, Error> {
        self.uint(u64::from(v))
    }

    /// Encodes `v`.
    #[inline]
    pub fn u64(&mut self, v: u64) -> Result<&mut Self, Error> {
        self.uint(v)
    }

    /// Encodes `v`.
    #[inline]
    pub fn i8(&mut self, v: i8) -> Result<&mut Self, Error> {
        self.i64(i64::from(v))
    }

    /// Encodes `v`.
    #[inline]
    pub fn i16(&mut self, v: i16) -> Result<&mut Self, Error> {
        self.i64(i64::from(v))
    }

    /// Encodes `v`.
    #[inline]
    pub fn i32(&mut self, v: i32) -> Result<&mut Self, Error> {
        self.i64(i64::from(v))
    }

    /// Encodes a byte string header followed by its contents.
    #[inline]
    pub fn bytes(&mut self, b: &[u8]) -> Result<&mut Self, Error> {
        self.put_payload(Major::Bytes, b)
    }

    /// Encodes a text string header followed by its UTF-8 contents.
    #[inline]
    pub fn str(&mut self, s: &str) -> Result<&mut Self, Error> {
        self.put_payload(Major::Text, s.as_bytes())
    }

    /// Encodes `true` or `false`.
    #[inline]
    pub fn bool(&mut self, v: bool) -> Result<&mut Self, Error> {
        self.simple(match v {
            true => Simple::TRUE,
            false => Simple::FALSE,
        })
    }

    /// Encodes `null`.
    #[inline]
    pub fn null(&mut self) -> Result<&mut Self, Error> {
        self.simple(Simple::NULL)
    }

    /// Encodes `undefined`. RFC 8949 §5.7 discourages its use on the wire,
    /// but the encoding is supported.
    #[inline]
    pub fn undefined(&mut self) -> Result<&mut Self, Error> {
        self.simple(Simple::UNDEFINED)
    }

    /// Encodes a simple value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::INVALID_SIMPLE`] when the value would require the
    /// two-byte form while being below 32; RFC 8949 §3.3 forbids that form.
    #[inline]
    pub fn simple(&mut self, s: Simple) -> Result<&mut Self, Error> {
        match s.0 < 24 || s.valid_two_byte() {
            true => {
                self.put_head(Major::Simple, u64::from(s.0))?;
                Ok(self)
            }
            false => Err(Error::INVALID_SIMPLE),
        }
    }

    /// Emits a tag header; encode the tagged item immediately after.
    ///
    /// Tag semantics live outside this crate (RFC 8949 §3.4): only the tag
    /// number travels on the wire.
    #[inline]
    pub fn tag(&mut self, number: u64) -> Result<&mut Self, Error> {
        self.put_head(Major::Tag, number)?;
        Ok(self)
    }

    /// Emits an array header for `len` following items.
    #[inline]
    pub fn array_len(&mut self, len: usize) -> Result<&mut Self, Error> {
        match u64::try_from(len) {
            Ok(n) => self.put_head(Major::Array, n)?,
            Err(_) => return Err(Error::OUT_OF_SPACE),
        }
        Ok(self)
    }

    /// Emits a map header for `pairs` following key/value pairs.
    #[inline]
    pub fn map_len(&mut self, pairs: usize) -> Result<&mut Self, Error> {
        match u64::try_from(pairs) {
            Ok(n) => self.put_head(Major::Map, n)?,
            Err(_) => return Err(Error::OUT_OF_SPACE),
        }
        Ok(self)
    }
}

#[cfg(feature = "float")]
mod float_impls {
    use super::{Encoder, Error};
    use crate::float::{f16_bits_of, f32_of_f16_bits};
    use crate::head::write_head_exact;

    impl Encoder<'_> {
        /// Encodes an `f32` using the shortest representation that
        /// preserves the value exactly (RFC 8949 §4.2.2).
        pub fn f32(&mut self, v: f32) -> Result<&mut Self, Error> {
            match f16_bits_of(v) {
                Some(half) => self.float_exact(u64::from(half), 25),
                None => self.float_exact(u64::from(v.to_bits()), 26),
            }
        }

        /// Encodes an `f64` using the shortest exact representation.
        pub fn f64(&mut self, v: f64) -> Result<&mut Self, Error> {
            if let Some(half) = f16_bits_of(v as f32) {
                if f64::from(f32_of_f16_bits(half)) == v {
                    return self.float_exact(u64::from(half), 25);
                }
            }
            if !v.is_nan() && f64::from(v as f32) == v {
                return self.float_exact(u64::from((v as f32).to_bits()), 26);
            }
            self.float_exact(v.to_bits(), 27)
        }

        /// Encodes an `f32` unconditionally as IEEE 754 binary16.
        ///
        /// Non-representable values lose information; prefer [`f32`](Self::f32)
        /// for automatic width selection.
        pub fn f16(&mut self, v: f32) -> Result<&mut Self, Error> {
            let half = f16_bits_of(v).unwrap_or(0x7e00);
            self.float_exact(u64::from(half), 25)
        }

        #[inline]
        fn float_exact(&mut self, bits: u64, ai: u8) -> Result<&mut Self, Error> {
            let n = write_head_exact(self.tail(), bits, ai)?;
            self.pos += n;
            Ok(self)
        }
    }
}

/// Compile-time serialization helpers (`const fn`, panic on overflow).
///
/// Inside `const` evaluation a panic is a compile error, which turns these
/// into checked builders for constant CBOR items:
///
/// ```
/// use const_cbor::encode::consts;
///
/// const HEARTBEAT: ([u8; 3], usize) = {
///     let mut buf = [0u8; 3];
///     let n = consts::put_u64(&mut buf, 0);
///     (buf, n)
/// };
/// assert_eq!(&HEARTBEAT.0[..HEARTBEAT.1], &[0x00]);
/// ```
pub mod consts {
    use crate::error::Error;
    use crate::head::{Major, write_head};

    /// Preferred encoded length of unsigned integer `v`.
    #[inline]
    #[must_use]
    pub const fn uint_len(v: u64) -> usize {
        crate::head::uint_len(v)
    }

    /// Preferred encoded length of signed integer `v`.
    #[inline]
    #[must_use]
    pub const fn i64_len(v: i64) -> usize {
        match v >= 0 {
            true => uint_len(v as u64),
            false => uint_len(!(v as u64)),
        }
    }

    /// Encoded length of a string/bytes item with `payload_len` content bytes.
    #[inline]
    #[must_use]
    pub const fn chunk_len(payload_len: usize) -> usize {
        uint_len(payload_len as u64) + payload_len
    }

    /// Writes unsigned integer `v` into `buf`; returns bytes written.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is too small — a compile error under `const` eval.
    #[inline]
    #[must_use]
    pub const fn put_u64(buf: &mut [u8], v: u64) -> usize {
        expect(write_head(buf, Major::Unsigned, v))
    }

    /// Writes signed integer `v` into `buf`; returns bytes written.
    ///
    /// # Panics
    ///
    /// Panics if `buf` is too small — a compile error under `const` eval.
    #[inline]
    #[must_use]
    pub const fn put_i64(buf: &mut [u8], v: i64) -> usize {
        match v >= 0 {
            true => put_u64(buf, v as u64),
            false => expect(write_head(buf, Major::Negative, !(v as u64))),
        }
    }

    /// Converts an error to a panic (a compile error during const eval).
    #[inline]
    const fn expect(r: Result<usize, Error>) -> usize {
        match r {
            Ok(n) => n,
            Err(_) => panic!("const-cbor: output buffer too small"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;
    use super::consts::{chunk_len, i64_len, put_i64, put_u64, uint_len};
    use crate::Error;

    #[test]
    fn integers_preferred_widths() {
        let mut b = [0u8; 20];
        let mut e = Encoder::new(&mut b);
        e.u8(23).unwrap().u8(24).unwrap().u32(1000).unwrap();
        e.u64(1_000_000).unwrap().u64(u64::MAX).unwrap();
        assert_eq!(e.written(), 20, "1 + 2 + 3 + 5 + 9 bytes expected");
        assert_eq!(
            &b,
            &[
                0x17, 0x18, 0x18, 0x19, 0x03, 0xe8, 0x1a, 0x00, 0x0f, 0x42, 0x40, 0x1b, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            ]
        );
    }

    #[test]
    fn negative_mapping_rfc_examples() {
        let cases: [(i64, &[u8]); 4] =
            [(-1, &[0x20]), (-10, &[0x29]), (-100, &[0x38, 0x63]), (-1000, &[0x39, 0x03, 0xe7])];
        for (v, want) in cases {
            let mut b = [0u8; 9];
            let mut e = Encoder::new(&mut b);
            e.i64(v).unwrap();
            let n = e.written();
            assert_eq!(&b[..n], want);
        }
        // i64::MIN maps through bitwise NOT without overflow.
        let mut b = [0u8; 9];
        let mut e = Encoder::new(&mut b);
        e.i64(i64::MIN).unwrap();
        assert_eq!(e.written(), 9);
        // Magnitudes beyond i64 are legal CBOR via nint.
        let mut b2 = [0u8; 9];
        let mut e2 = Encoder::new(&mut b2);
        e2.nint(u64::MAX).unwrap();
        assert_eq!(e2.written(), 9);
    }

    #[test]
    fn strings_and_bytes() {
        let mut b = [0u8; 32];
        let mut e = Encoder::new(&mut b);
        e.str("hello").unwrap().bytes(&[1, 2, 3]).unwrap();
        assert_eq!(e.written(), 6 + 4);
        assert_eq!(&b[..6], &[0x65, b'h', b'e', b'l', b'l', b'o']);
    }

    #[test]
    fn headers_and_tags() {
        let mut b = [0u8; 16];
        let mut e = Encoder::new(&mut b);
        e.array_len(3).unwrap().map_len(2).unwrap().tag(1).unwrap().u32(1).unwrap();
        assert_eq!(e.written(), 4);
        assert_eq!(&b[..4], &[0x83, 0xa2, 0xc1, 0x01]);
    }

    #[test]
    fn simple_values() {
        let mut b = [0u8; 4];
        let mut e = Encoder::new(&mut b);
        e.bool(true).unwrap().null().unwrap();
        assert!(e.simple(crate::Simple(31)).is_err_and(|err| err == Error::INVALID_SIMPLE));
        assert!(e.simple(crate::Simple(99)).is_ok());
        // Ending the borrow explicitly documents that the buffer is final.
        let Encoder { buf, .. } = e;
        let _ = buf;
        assert_eq!(&b, &[0xf5, 0xf6, 0xf8, 99]);
    }

    #[test]
    fn out_of_space_preserves_cursor() {
        let mut b = [0u8; 3];
        let mut e = Encoder::new(&mut b);
        e.u8(7).unwrap();
        assert!(e.str("toolong").is_err_and(|err| err == Error::OUT_OF_SPACE));
        assert_eq!(e.written(), 1);
    }

    #[test]
    fn const_helpers_match_runtime() {
        assert_eq!(uint_len(23), 1);
        assert_eq!(i64_len(-1000), 3);
        assert_eq!(chunk_len(300), 303);
        let mut b = [0u8; 9];
        let n = put_i64(&mut b, -1000);
        assert_eq!(n, 3);
        assert_eq!(put_u64(&mut b[n..], 0), 1);
    }
}
