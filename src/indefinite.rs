//! Indefinite-length items (RFC 8949 §3.2), feature `indefinite`.
//!
//! Strings may be emitted and consumed chunk-wise; arrays and maps may be
//! left open-ended and terminated with a break byte. Everything stays
//! zero-copy and bounded: the decoder never buffers, the encoder tracks
//! open scopes in a fixed [`ENCODE_DEPTH_LIMIT`] slot stack.

use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::error::Error;
use crate::head::{self, Major};

/// Maximum number of simultaneously open indefinite scopes on [`Encoder`].
pub const ENCODE_DEPTH_LIMIT: usize = 16;

// ---------------------------------------------------------------- encoder

impl Encoder<'_> {
    /// Begins an indefinite-length byte string; append chunks with
    /// [`bytes_chunk`](Encoder::bytes_chunk), close with [`end_bytes`](Encoder::end_bytes).
    pub fn begin_bytes(&mut self) -> Result<&mut Self, Error> {
        self.begin(Major::Bytes)
    }

    /// Begins an indefinite-length text string.
    pub fn begin_str(&mut self) -> Result<&mut Self, Error> {
        self.begin(Major::Text)
    }

    /// Begins an open-ended array; write items, then call [`end_indef`](Encoder::end_indef).
    pub fn begin_array(&mut self) -> Result<&mut Self, Error> {
        self.begin(Major::Array)
    }

    /// Begins an open-ended map; write pairs, then call [`end_indef`](Encoder::end_indef).
    pub fn begin_map(&mut self) -> Result<&mut Self, Error> {
        self.begin(Major::Map)
    }

    fn begin(&mut self, major: Major) -> Result<&mut Self, Error> {
        self.open_scope(major)?;
        // The indefinite marker is an additional-information *field* of 31
        // in the initial byte itself, not an encoded argument of 31.
        match self.reserve(1) {
            Ok(()) => {}
            Err(e) => {
                self.indef_sp -= 1;
                return Err(e);
            }
        }
        self.buf[self.pos] = major.bits() | 0x1f;
        self.pos += 1;
        Ok(self)
    }

    /// Appends one definite chunk to the open indefinite *byte* string.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] if no string scope is open.
    pub fn bytes_chunk(&mut self, b: &[u8]) -> Result<&mut Self, Error> {
        self.require_kind(Major::Bytes)?;
        self.put_payload(Major::Bytes, b)?;
        Ok(self)
    }

    /// Appends one definite chunk to the open indefinite *text* string.
    ///
    /// # Errors
    ///
    /// As [`Encoder::bytes_chunk`].
    pub fn str_chunk(&mut self, s: &str) -> Result<&mut Self, Error> {
        self.require_kind(Major::Text)?;
        self.put_payload(Major::Text, s.as_bytes())?;
        Ok(self)
    }

    /// Closes the innermost open indefinite item with a break byte.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] when nothing indefinite is open.
    pub fn end_indef(&mut self) -> Result<&mut Self, Error> {
        match self.pop_scope() {
            Some(_) => {
                self.write_break()?;
                Ok(self)
            }
            None => Err(Error::TYPE_MISMATCH),
        }
    }

    /// Closes the innermost indefinite scope, requiring a byte string.
    pub fn end_bytes(&mut self) -> Result<&mut Self, Error> {
        self.end_of_kind(Major::Bytes)
    }

    /// Closes the innermost indefinite scope, requiring a text string.
    pub fn end_str(&mut self) -> Result<&mut Self, Error> {
        self.end_of_kind(Major::Text)
    }

    /// Convenience: complete indefinite byte string from separate chunks.
    pub fn bytes_indef(&mut self, chunks: &[&[u8]]) -> Result<&mut Self, Error> {
        self.begin_bytes()?;
        for c in chunks {
            self.bytes_chunk(c)?;
        }
        self.end_bytes()
    }

    /// Convenience: complete indefinite text string from separate chunks.
    pub fn str_indef(&mut self, chunks: &[&str]) -> Result<&mut Self, Error> {
        self.begin_str()?;
        for c in chunks {
            self.str_chunk(c)?;
        }
        self.end_str()
    }

    fn end_of_kind(&mut self, major: Major) -> Result<&mut Self, Error> {
        self.require_kind(major)?;
        self.end_indef()
    }

    #[inline]
    fn require_kind(&self, major: Major) -> Result<(), Error> {
        match self.open_kind() == Some(major) {
            true => Ok(()),
            false => Err(Error::TYPE_MISMATCH),
        }
    }

    #[inline]
    fn open_scope(&mut self, major: Major) -> Result<(), Error> {
        let sp = usize::from(self.indef_sp);
        match sp < ENCODE_DEPTH_LIMIT {
            true => {
                self.indef[sp] = major as u8;
                self.indef_sp += 1;
                Ok(())
            }
            false => Err(Error::DEPTH_EXCEEDED),
        }
    }

    #[inline]
    fn open_kind(&self) -> Option<Major> {
        match self.indef_sp {
            0 => None,
            n => Some(Major::from_initial(self.indef[usize::from(n) - 1] << 5)),
        }
    }

    #[inline]
    fn pop_scope(&mut self) -> Option<Major> {
        let kind = self.open_kind();
        self.indef_sp -= u8::from(kind.is_some());
        kind
    }

    #[inline]
    fn write_break(&mut self) -> Result<(), Error> {
        self.reserve(1)?;
        self.buf[self.pos] = head::BREAK;
        self.pos += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------- decoder

/// Iterator over the definite chunks of an indefinite-length string.
///
/// Yields borrowed chunks until the break byte; then reports completion
/// through [`StringChunks::finished`]. Malformed input surfaces as
/// `finished() == false` so callers can turn it into [`Error::TRUNCATED`].
///
/// The struct is deliberately `Copy`: iterators over borrowed chunks are
/// cheap handles, and resuming from a copy is well-defined.
#[allow(clippy::copy_iterator)]
#[derive(Clone, Copy, Debug)]
pub struct StringChunks<'a, T> {
    rest: &'a [u8],
    state: State,
    _marker: core::marker::PhantomData<fn() -> T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Open,
    Broken,
    Malformed,
}

impl<T> StringChunks<'_, T> {
    /// `true` once the terminating break was reached cleanly.
    #[must_use]
    pub const fn finished(&self) -> bool {
        matches!(self.state, State::Broken)
    }

    /// Bytes of input not yet consumed by this iterator, including any
    /// unread break byte.
    #[must_use]
    pub const fn unconsumed(&self) -> usize {
        self.rest.len()
    }
}

#[allow(clippy::copy_iterator)]
impl<'a> Iterator for StringChunks<'a, &'a [u8]> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.advance(|rest| {
            let mut dec = Decoder::new(rest);
            let raw = dec.bytes_of(Major::Bytes).ok()?;
            Some((raw, dec.position()))
        })
    }
}

#[allow(clippy::copy_iterator)]
impl<'a> Iterator for StringChunks<'a, &'a str> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance(|rest| {
            let mut dec = Decoder::new(rest);
            let raw = dec.bytes_of(Major::Text).ok()?;
            let s = core::str::from_utf8(raw).ok()?;
            Some((s, dec.position()))
        })
    }
}

impl<'a, T> StringChunks<'a, T> {
    fn advance(&mut self, read: impl FnOnce(&'a [u8]) -> Option<(T, usize)>) -> Option<T> {
        match self.state {
            State::Open => {}
            _ => return None,
        }
        match self.rest.first() {
            Some(&b) if head::is_break(b) => {
                self.state = State::Broken;
                self.rest = &[];
                None
            }
            None => {
                self.state = State::Malformed;
                None
            }
            Some(_) => match read(self.rest) {
                Some((chunk, used)) => {
                    self.rest = &self.rest[used..];
                    Some(chunk)
                }
                None => {
                    self.state = State::Malformed;
                    None
                }
            },
        }
    }
}

impl<'a> Decoder<'a> {
    /// Iterates the chunks of an indefinite-length byte string.
    ///
    /// Consumes only the header; advance the iterator to consume chunks
    /// and the break. After iteration, [`StringChunks::finished`] tells
    /// whether the item terminated correctly.
    ///
    /// # Errors
    ///
    /// [`Error::TYPE_MISMATCH`] unless the next item is an indefinite
    /// byte string.
    pub fn bytes_chunks(&mut self) -> Result<StringChunks<'a, &'a [u8]>, Error> {
        self.begin_chunks(Major::Bytes).map(|start| StringChunks {
            rest: &self.input[start..],
            state: State::Open,
            _marker: core::marker::PhantomData,
        })
    }

    /// Iterates the chunks of an indefinite-length text string.
    ///
    /// # Errors
    ///
    /// As [`Decoder::bytes_chunks`], for text strings.
    pub fn str_chunks(&mut self) -> Result<StringChunks<'a, &'a str>, Error> {
        self.begin_chunks(Major::Text).map(|start| StringChunks {
            rest: &self.input[start..],
            state: State::Open,
            _marker: core::marker::PhantomData,
        })
    }

    /// Reconciles the decoder's position with a fully-iterated chunk
    /// stream, consuming its break byte.
    ///
    /// # Errors
    ///
    /// [`Error::TRUNCATED`] unless the iterator reached its break cleanly;
    /// [`Error::TYPE_MISMATCH`] if the iterator belongs to different input.
    pub fn sync_chunks<T>(&mut self, it: &StringChunks<'a, T>) -> Result<(), Error> {
        match it.finished() {
            true => {}
            false => return Err(Error::TRUNCATED),
        }
        // The iterator's tail is always a suffix of this decoder's input.
        self.pos = self.input.len() - it.rest.len();
        Ok(())
    }

    #[inline]
    fn begin_chunks(&mut self, want: Major) -> Result<usize, Error> {
        let pos = self.pos;
        let (major, _, indef) = self.next_head_ext()?;
        match major == want && indef {
            true => Ok(pos + 1),
            false => Err(Error::TYPE_MISMATCH),
        }
    }
}
