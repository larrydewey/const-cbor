//! Incremental decoding across arbitrarily chunked input, feature `stream`.
//!
//! The caller supplies one fixed scratch buffer; [`StreamDecoder`] copies
//! incoming fragments into it and hands out complete top-level items as
//! zero-copy decoders once enough data has arrived. No heap and no
//! unbounded buffering: the capacity bounds maximum item size by
//! construction.

use crate::decode::Decoder;
use crate::error::Error;

/// Incremental decoder over a caller-provided scratch buffer.
///
/// Feed wire fragments with [`feed`](StreamDecoder::feed); pull completed
/// items with [`take_item`](StreamDecoder::take_item). An item becomes
/// available only once it fits the scratch buffer entirely.
pub struct StreamDecoder<'b> {
    buf: &'b mut [u8],
    fill: usize,
}

impl<'b> StreamDecoder<'b> {
    /// Creates a stream decoder using `scratch` as its only storage.
    #[inline(always)]
    #[must_use]
    pub fn new(scratch: &'b mut [u8]) -> Self {
        Self { buf: scratch, fill: 0 }
    }

    /// Copies as much of `data` into the scratch buffer as fits.
    ///
    /// Returns the number of bytes accepted; if fewer than `data.len()`,
    /// take items out and retry. Feeding an empty slice is always valid.
    pub fn feed(&mut self, data: &[u8]) -> usize {
        let room = self.buf.len().saturating_sub(self.fill);
        let n = room.min(data.len());
        self.buf[self.fill..self.fill + n].copy_from_slice(&data[..n]);
        self.fill += n;
        n
    }

    /// Bytes buffered but not yet handed out as items.
    #[inline(always)]
    #[must_use]
    pub const fn pending(&self) -> usize {
        self.fill
    }

    /// Scratch space left for further feeds.
    #[inline(always)]
    #[must_use]
    pub const fn capacity_left(&self) -> usize {
        self.buf.len() - self.fill
    }

    /// Removes and returns the next complete buffered item, if any.
    ///
    /// * `Ok(None)` — need more input.
    /// * `Err(e)` — structural error in the buffered prefix.
    ///
    /// The returned decoder borrows from `'b`, so it stays valid until you
    /// feed again (which reuses that part of the buffer).
    pub fn take_item(&mut self) -> Result<Option<Decoder<'b>>, Error> {
        if self.fill == 0 {
            return Ok(None);
        }
        // Probe how many bytes the first complete item occupies.
        let mut probe = Decoder::new(&self.buf[..self.fill]);
        match probe.skip() {
            Ok(()) => {}
            Err(Error::TRUNCATED) => return Ok(None),
            Err(e) => return Err(e),
        }
        let total = probe.position();

        // Split the item off the front, keep the remainder in place.
        let rest = core::mem::take(&mut self.buf);
        let (item, tail) = rest.split_at_mut(total);
        let rest_len = self.fill - total;
        tail.copy_within(..rest_len, 0);
        self.buf = tail;
        self.fill = rest_len;
        Ok(Some(Decoder::new(item)))
    }
}

#[cfg(test)]
mod tests {
    use super::StreamDecoder;

    #[test]
    fn items_across_feed_boundaries() {
        let mut scratch = [0u8; 32];
        let mut s = StreamDecoder::new(&mut scratch);
        // Split one item across two feeds.
        assert_eq!(s.feed(&[0x19]), 1);
        assert!(matches!(s.take_item(), Ok(None)));
        assert_eq!(s.feed(&[0x03, 0xe8, 0x01]), 3);
        let mut dec = s.take_item().unwrap().unwrap();
        assert_eq!(dec.u16().unwrap(), 1000);
        assert!(dec.is_empty());
        // The trailing 0x01 is a separate item.
        let mut dec = s.take_item().unwrap().unwrap();
        assert_eq!(dec.u8().unwrap(), 1);
        assert!(s.take_item().unwrap().is_none());
        assert_eq!(s.pending(), 0);
    }

    #[test]
    fn multiple_items_per_feed() {
        let mut scratch = [0u8; 32];
        let mut s = StreamDecoder::new(&mut scratch);
        assert_eq!(s.feed(&[0x01, 0x02, 0x03]), 3);
        for v in 1u8..=3 {
            let mut d = s.take_item().unwrap().unwrap();
            assert_eq!(d.u8().unwrap(), v);
        }
        assert!(s.take_item().unwrap().is_none());
    }

    #[test]
    fn overflow_is_reported_by_partial_accept() {
        let mut scratch = [0u8; 2];
        let mut s = StreamDecoder::new(&mut scratch);
        assert_eq!(s.feed(&[1, 2, 3, 4]), 2);
        assert_eq!(s.capacity_left(), 0);
    }

    #[test]
    fn structural_errors_surface() {
        let mut scratch = [0u8; 4];
        let mut s = StreamDecoder::new(&mut scratch);
        s.feed(&[0x1c]); // reserved additional information
        assert!(matches!(s.take_item(), Err(crate::Error::INVALID_ADDITIONAL_INFO)));
    }
}
