//! Zero-copy raw item handles, feature `raw`.
//!
//! [`RawValue`] is a borrow of one complete, well-formed CBOR item — the
//! allocation-free stand-in for a generic value tree. It can be inspected,
//! re-decoded lazily, or re-emitted verbatim.

use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::error::Error;

/// A borrowed slice containing exactly one complete CBOR data item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawValue<'a>(&'a [u8]);

impl<'a> RawValue<'a> {
    /// The encoded bytes of the item.
    #[inline(always)]
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.0
    }

    /// Encoded length in bytes.
    #[inline(always)]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: a `RawValue` contains at least one byte.
    #[inline(always)]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// A decoder positioned at the start of this item.
    ///
    /// The bytes are known well-formed (validated when the handle was
    /// created), so decoding cannot fail structurally; only typed
    /// mismatches can.
    #[inline(always)]
    #[must_use]
    pub fn decoder(&self) -> Decoder<'a> {
        Decoder::new(self.0)
    }
}

impl core::ops::Deref for RawValue<'_> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.0
    }
}

impl Encoder<'_> {
    /// Re-emits a raw item verbatim — no parsing, no width normalization.
    ///
    /// Note: verbatim means a non-preferred encoding inside `value` is
    /// copied as-is (RFC 8949 §4.2.1 permits this).
    pub fn raw(&mut self, value: &RawValue<'_>) -> Result<&mut Self, Error> {
        let Some(end) = self.pos.checked_add(value.len()) else {
            return Err(Error::OUT_OF_SPACE);
        };
        if end > self.buf.len() {
            return Err(Error::OUT_OF_SPACE);
        }
        self.buf[self.pos..end].copy_from_slice(value.as_bytes());
        self.pos = end;
        Ok(self)
    }
}

impl Decoder<'_> {
    /// Captures the next complete item without interpreting it.
    ///
    /// The item is structurally validated — that is what lets [`RawValue`]
    /// guarantee safe lazy decoding later.
    ///
    /// # Errors
    ///
    /// Any structural error, including [`Error::DEPTH_EXCEEDED`] beyond
    /// [`SKIP_LIMIT`](crate::decode::SKIP_LIMIT).
    pub fn raw_value(&mut self) -> Result<RawValue<'_>, Error> {
        let start = self.pos;
        self.skip()?;
        Ok(RawValue(&self.input[start..self.pos]))
    }
}
