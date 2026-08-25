//! Compact, allocation-free error codes.
//!
//! Every fallible operation in this crate returns [`Error`], a single byte
//! identifying the failure class. Errors are `Copy`, `Eq`, `Ord`, and carry
//! no formatting machinery; use [`Error::name`] for a static description.

use core::fmt;

/// A CBOR processing failure, represented as a compact numeric code.
///
/// Variant names use `SCREAMING_CASE` because they behave like named
/// constants: each is a stable, single-byte protocol-level code.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Error {
    /// Output buffer had insufficient space for the encoded item.
    OUT_OF_SPACE = 0,
    /// Input ended before the current item was complete.
    TRUNCATED = 1,
    /// Initial byte encodes an additional-information value reserved for
    /// indefinite lengths or future use (28–30), or a 64-bit argument on a
    /// platform narrower than `u64`.
    INVALID_ADDITIONAL_INFO = 2,
    /// Value did not fit the requested Rust integer type.
    INTEGER_OUT_OF_RANGE = 3,
    /// Encountered item type differs from the expected type.
    TYPE_MISMATCH = 4,
    /// Text string payload is not valid UTF-8 (RFC 3629).
    INVALID_UTF8 = 5,
    /// Two-byte simple-value encoding used for a value below 32
    /// (RFC 8949 §3.3 forbids this form).
    INVALID_SIMPLE = 6,
    /// Floating-point support is disabled (`float` feature off) but the
    /// item is major type 7 with a floating-point representation.
    FLOAT_UNSUPPORTED = 7,
    /// A skipped structure nested deeper than [`crate::decode::SKIP_LIMIT`].
    DEPTH_EXCEEDED = 8,
    /// A break byte appeared where no indefinite item was open.
    UNEXPECTED_BREAK = 9,
    /// Indefinite-length support is disabled (`indefinite` feature off).
    INDEFINITE_UNSUPPORTED = 10,
    /// Item violates RFC 8949 section 4.2 preferred/deterministic
    /// serialization: non-minimal argument width, non-shortest float,
    /// indefinite length, unsorted or duplicate map keys, or a non-preferred
    /// bignum. Produced only by [`crate::deterministic`] (`deterministic`
    /// feature); ordinary decoding accepts all well-formed forms.
    NOT_CANONICAL = 11,
}

impl Error {
    /// Static name of this error code.
    #[inline(always)]
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::OUT_OF_SPACE => "out of space",
            Self::TRUNCATED => "truncated input",
            Self::INVALID_ADDITIONAL_INFO => "invalid additional information",
            Self::INTEGER_OUT_OF_RANGE => "integer out of range",
            Self::TYPE_MISMATCH => "type mismatch",
            Self::INVALID_UTF8 => "invalid utf-8",
            Self::INVALID_SIMPLE => "invalid simple value",
            Self::FLOAT_UNSUPPORTED => "float unsupported",
            Self::DEPTH_EXCEEDED => "nesting depth exceeded",
            Self::UNEXPECTED_BREAK => "unexpected break",
            Self::INDEFINITE_UNSUPPORTED => "indefinite lengths unsupported",
            Self::NOT_CANONICAL => "not canonical",
        }
    }

    /// Short reason string suitable for logging without formatting.
    #[inline(always)]
    #[must_use]
    pub const fn describe(self) -> (&'static str, u8) {
        (self.name(), self as u8)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// `core::error::Error` is stable since Rust 1.81, so the trait impl needs
// no std dependency at all.
impl core::error::Error for Error {}
