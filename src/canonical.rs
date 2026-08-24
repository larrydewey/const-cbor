//! Deterministic map encoding (RFC 8949 §4.2), feature `canonical`.
//!
//! [`MapBuilder`] records the byte spans of each key/value pair while you
//! write them, then reorders the map region into canonical key order at
//! [`finish`](MapBuilder::finish) using a caller-provided scratch slice.
//! Capacity is a const generic, so nothing allocates and the bound is
//! visible in types.

use crate::encode::Encoder;
use crate::error::Error;

/// Builds a map whose keys end up in canonical (RFC 8949 §4.2) order:
/// lexicographic by encoded key bytes.
///
/// Write pairs with [`add`](MapBuilder::add), then finalize with
/// [`finish`](MapBuilder::finish). `N` is the maximum pair count; writing
/// more fails with [`Error::DEPTH_EXCEEDED`].
///
/// Canonical order is bytewise over *encoded* keys, so the length prefix
/// participates: `"zeta"` (4 bytes) precedes `"alpha"` (5 bytes), unlike
/// plain ASCII ordering. Use equal-length keys for intuitive results.
///
/// ```rust
/// use const_cbor::Encoder;
/// use const_cbor::canonical::MapBuilder;
///
/// let mut buf = [0u8; 64];
/// let mut enc = Encoder::new(&mut buf);
/// enc.map_len(2).unwrap();
/// let mut m = MapBuilder::<2>::new(&mut enc);
/// m.add(|k| k.str("bb"), |v| v.u8(1)).unwrap();
/// m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
/// m.finish(&mut [0u8; 8]).unwrap();   // pairs reordered: "aa" first
///
/// let (out, n) = enc.finish();
/// // a2 | "aa":2 | "bb":1   (canonical byte order)
/// assert_eq!(&out[..n], &[0xa2, 0x62, b'a', b'a', 2, 0x62, b'b', b'b', 1]);
/// ```
pub struct MapBuilder<'e, 'b, const N: usize> {
    enc: &'e mut Encoder<'b>,
    /// `(key_start, key_end, value_end)` per entry, in insertion order.
    spans: [(u32, u32, u32); N],
    count: usize,
}

impl<'e, 'b, const N: usize> MapBuilder<'e, 'b, N> {
    /// Adopts the encoder; the map header must already be written.
    #[must_use]
    pub fn new(enc: &'e mut Encoder<'b>) -> Self {
        Self { enc, spans: [(0, 0, 0); N], count: 0 }
    }

    /// Adds one key/value pair.
    ///
    /// Both closures receive the same encoder and must write exactly one
    /// item each; the bytes between their calls are treated as key span,
    /// everything after as value span.
    ///
    /// # Errors
    ///
    /// Propagates encoding errors; [`Error::DEPTH_EXCEEDED`] when more
    /// than `N` pairs are added.
    pub fn add<K, V>(&mut self, key: K, value: V) -> Result<&mut Self, Error>
    where
        for<'x> K: FnOnce(&'x mut Encoder<'b>) -> Result<&'x mut Encoder<'b>, Error>,
        for<'x> V: FnOnce(&'x mut Encoder<'b>) -> Result<&'x mut Encoder<'b>, Error>,
    {
        match self.count < N {
            true => {}
            false => return Err(Error::DEPTH_EXCEEDED),
        }
        let start = self.enc.written();
        let enc: &'_ mut Encoder<'b> = &mut *self.enc;
        key(enc)?;
        let key_end = self.enc.written();
        let enc: &'_ mut Encoder<'b> = &mut *self.enc;
        value(enc)?;
        self.spans[self.count] = (start as u32, key_end as u32, self.enc.written() as u32);
        self.count += 1;
        Ok(self)
    }

    /// Reorders the written pairs into canonical key order.
    ///
    /// If the pairs were added out of order, the map region is rebuilt
    /// through `scratch` (which must hold at least the region's size).
    /// On success the encoder is positioned after the map.
    ///
    /// # Errors
    ///
    /// [`Error::OUT_OF_SPACE`] when `scratch` is too small or spans are
    /// inconsistent.
    pub fn finish(self, scratch: &mut [u8]) -> Result<&'e mut Encoder<'b>, Error> {
        // Canonical order: lexicographic by encoded key bytes (RFC 8949
        // section 4.2). Insertion sort over a fixed index array keeps the
        // work bounded and stable; the immutable buffer borrow ends before
        // the reorder phase below.
        let order: [usize; N] = {
            let buf: &[u8] = self.enc.buf;
            let keys: [&[u8]; N] =
                core::array::from_fn(|i| &buf[self.spans[i].0 as usize..self.spans[i].1 as usize]);
            let mut order = [0usize; N];
            for (i, slot) in order.iter_mut().enumerate() {
                *slot = i;
            }
            for i in 1..self.count {
                let mut j = i;
                while j > 0 && keys[order[j - 1]] > keys[order[j]] {
                    order.swap(j - 1, j);
                    j -= 1;
                }
            }
            order
        };

        if self.count == 0 {
            return Ok(self.enc);
        }
        let region_start = self.spans[0].0 as usize;
        let region_end = self.enc.written();

        let sorted = order[..self.count].windows(2).all(|w| w[0] < w[1]);
        if sorted {
            return Ok(self.enc);
        }

        let region_len = region_end - region_start;
        match region_len <= scratch.len() {
            true => {}
            false => return Err(Error::OUT_OF_SPACE),
        }
        scratch[..region_len].copy_from_slice(&self.enc.buf[region_start..region_end]);

        let mut dst = region_start;
        for &idx in &order[..self.count] {
            let (ks, _, ve) = self.spans[idx];
            let (ks, ve) = (ks as usize, ve as usize);
            let len = ve - ks;
            self.enc.buf[dst..dst + len]
                .copy_from_slice(&scratch[ks - region_start..ve - region_start]);
            dst += len;
        }
        debug_assert_eq!(dst, region_end);
        Ok(self.enc)
    }
}

#[cfg(test)]
mod tests {
    use super::MapBuilder;
    use crate::Encoder;

    #[test]
    fn canonical_reorder_produces_sorted_keys() {
        let mut buf = [0u8; 64];
        let mut enc = Encoder::new(&mut buf);
        enc.map_len(2).unwrap();
        let mut m = MapBuilder::<2>::new(&mut enc);
        m.add(|k| k.str("bb"), |v| v.u8(1)).unwrap();
        m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
        // spans: bb = (1,4,5), aa = (5,8,9); verified by output below.
        let enc = m.finish(&mut [0u8; 8]).unwrap();
        let n = enc.written();
        assert_eq!(&enc.buf[..n], &[0xa2, 0x62, b'a', b'a', 2, 0x62, b'b', b'b', 1]);
    }
}
