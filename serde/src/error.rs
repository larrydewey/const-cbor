//! Error type bridging [`const_cbor::Error`] and serde's trait errors.

use alloc::string::String;
use core::fmt;

/// A serialization or deserialization failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The underlying codec rejected the operation.
    Cbor(const_cbor::Error),
    /// A serde-level problem: unsupported data-model element, unexpected
    /// input shape, or a visitor's custom message.
    Custom(String),
}

impl Error {
    /// Serde's canonical custom-error constructor.
    #[must_use]
    pub fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(alloc::format!("{msg}"))
    }

    /// Helper for data-model elements this crate deliberately does not
    /// represent.
    #[allow(dead_code)]
    pub(crate) fn unsupported(what: &str) -> Self {
        Self::custom(alloc::format!("unsupported: {what}"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cbor(e) => write!(f, "cbor: {}", e.name()),
            Self::Custom(m) => f.write_str(m),
        }
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::custom(msg)
    }
}

impl From<const_cbor::Error> for Error {
    fn from(e: const_cbor::Error) -> Self {
        Self::Cbor(e)
    }
}

/// Shorthand used throughout the serializer/deserializer.
pub(crate) type Result<T> = core::result::Result<T, Error>;
