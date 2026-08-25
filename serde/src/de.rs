//! `serde::Deserializer` adapter over [`const_cbor::Decoder`].

use crate::error::{Error, Result};
use const_cbor::{Decoder, Major};
use serde::de::{self, VariantAccess, Visitor};

/// Deserializes serde values from a borrowed CBOR input.
///
/// Strings and byte slices deserialize zero-copy, borrowing from the
/// original input.
pub struct Deserializer<'de> {
    pub(crate) input: &'de [u8],
    pub(crate) pos: usize,
}

impl<'de> Deserializer<'de> {
    /// Creates a deserializer reading from the start of `input`.
    #[must_use]
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &'de [u8] {
        &self.input[self.pos..]
    }

    /// Runs one core-decoder operation, tracking cursor movement.
    fn with_decoder<T>(&mut self, f: impl FnOnce(&mut Decoder<'de>) -> Result<T>) -> Result<T> {
        let mut dec = Decoder::new(self.rest());
        // Give the closure the full remaining slice but preserve absolute
        // positions on return.
        let outcome = f(&mut dec);
        self.pos += dec.position();
        outcome
    }
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let Some(initial) = self.rest().first().copied() else {
            return Err(Error::Cbor(const_cbor::Error::TRUNCATED));
        };
        match Major::from_initial(initial) {
            Major::Unsigned => {
                let v = self.with_decoder(|d| d.u64().map_err(Error::from))?;
                visitor.visit_u64(v)
            }
            Major::Negative => {
                let v = self.with_decoder(|d| d.i64().map_err(Error::from))?;
                visitor.visit_i64(v)
            }
            Major::Bytes => {
                let v = self.with_decoder(|d| d.bytes().map_err(Error::from))?;
                visitor.visit_borrowed_bytes(v)
            }
            Major::Text => {
                let v = self.with_decoder(|d| d.str().map_err(Error::from))?;
                visitor.visit_borrowed_str(v)
            }
            Major::Array => self.deserialize_seq(visitor),
            Major::Map => self.deserialize_map(visitor),
            Major::Tag => tag_any(self, visitor),
            Major::Simple => simple_any(self, visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let b = self.with_decoder(|d| d.bool().map_err(Error::from))?;
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.i8().map_err(Error::from))?;
        visitor.visit_i8(v)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.i16().map_err(Error::from))?;
        visitor.visit_i16(v)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.i32().map_err(Error::from))?;
        visitor.visit_i32(v)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.i64().map_err(Error::from))?;
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.u8().map_err(Error::from))?;
        visitor.visit_u8(v)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.u16().map_err(Error::from))?;
        visitor.visit_u16(v)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.u32().map_err(Error::from))?;
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.u64().map_err(Error::from))?;
        visitor.visit_u64(v)
    }

    #[cfg(feature = "bignum")]
    #[cfg(feature = "bignum")]
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Basic integers stay preferred-encoded even through the 128-bit
        // serde API; only out-of-range magnitudes use tags 2/3.
        if matches!(
            self.rest().first().map(|b| Major::from_initial(*b)),
            Some(Major::Unsigned | Major::Negative)
        ) {
            let v = self.with_decoder(|d| d.i64().map_err(Error::from))?;
            return visitor.visit_i128(i128::from(v));
        }
        let (negative, mag) = bignum_magnitude(self)?;
        let mut n: i128 = 0;
        if mag.len() > 16 || (negative && mag.len() == 16 && mag[0] > 0x80) {
            return Err(Error::custom("bignum exceeds i128 range"));
        }
        for &b in mag {
            n = (n << 8) | i128::from(b);
        }
        let v: i128 = if negative { -1 - n } else { n };
        visitor.visit_i128(v)
    }

    #[cfg(feature = "bignum")]
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if matches!(self.rest().first().map(|b| Major::from_initial(*b)), Some(Major::Unsigned)) {
            let v = self.with_decoder(|d| d.u64().map_err(Error::from))?;
            return visitor.visit_u128(u128::from(v));
        }
        let (negative, mag) = bignum_magnitude(self)?;
        if negative || mag.len() > 16 {
            return Err(Error::custom("bignum exceeds u128 range"));
        }
        let mut n: u128 = 0;
        for &b in mag {
            n = (n << 8) | u128::from(b);
        }
        visitor.visit_u128(n)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.f32().map_err(Error::from))?;
        visitor.visit_f32(v)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.with_decoder(|d| d.f64().map_err(Error::from))?;
        visitor.visit_f64(v)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = self.with_decoder(|d| d.str().map_err(Error::from))?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::custom("expected single-character string")),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = self.with_decoder(|d| d.str().map_err(Error::from))?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let b = self.with_decoder(|d| d.bytes().map_err(Error::from))?;
        visitor.visit_borrowed_bytes(b)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let is_null = self.with_decoder(|d| d.is_null().map_err(Error::from))?;
        match is_null {
            true => visitor.visit_none(),
            false => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.with_decoder(consume_null_like)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let remaining = open_container(self, Major::Array)?;
        visitor.visit_seq(SeqAccess { de: self, remaining })
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let remaining = open_container(self, Major::Map)?;
        visitor.visit_map(MapAccess { de: self, remaining })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Structs accept either the map form (field names as keys) or the
        // compact seq form (positional fields).
        match self.rest().first() {
            Some(b) if Major::from_initial(*b) == Major::Map => self.deserialize_map(visitor),
            Some(b) if Major::from_initial(*b) == Major::Array => self.deserialize_seq(visitor),
            Some(_) => Err(Error::custom("expected struct as map or array")),
            None => Err(Error::Cbor(const_cbor::Error::TRUNCATED)),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let Some(initial) = self.rest().first().copied() else {
            return Err(Error::Cbor(const_cbor::Error::TRUNCATED));
        };
        match Major::from_initial(initial) {
            // Bare string: unit variant.
            Major::Text => visitor.visit_enum(UnitVariantAccess { de: self }),
            // One-entry definite map: { variant: payload }.
            Major::Map if initial & 0x1f != 31 => {
                let pairs = self.with_decoder(|d| d.map_len().map_err(Error::from))?;
                if pairs != 1 {
                    return Err(Error::custom("externally tagged enum needs a one-entry map"));
                }
                visitor.visit_enum(PayloadVariantAccess {
                    de: self,
                    marker: core::marker::PhantomData,
                })
            }
            _ => Err(Error::custom("expected enum variant")),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.rest().first().map(|b| Major::from_initial(*b)) {
            Some(Major::Text) => self.deserialize_str(visitor),
            Some(Major::Unsigned) => self.deserialize_u64(visitor),
            _ => Err(Error::custom("expected identifier")),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.with_decoder(|d| d.skip().map_err(Error::from))?;
        visitor.visit_unit()
    }
}

// ---------------------------------------------------------------- helpers

/// Consumes `null` or `undefined`.
fn consume_null_like(dec: &mut Decoder<'_>) -> Result<()> {
    if dec.is_null()? {
        return Ok(());
    }
    dec.undefined().map_err(Error::from)
}

/// Opens a container: definite returns its item count, indefinite consumes
/// its header and returns `None`.
fn open_container<'de>(de: &mut Deserializer<'de>, want: Major) -> Result<Option<usize>> {
    let Some(initial) = de.rest().first().copied() else {
        return Err(Error::Cbor(const_cbor::Error::TRUNCATED));
    };
    if initial & 0x1f == 31 && Major::from_initial(initial) == want {
        de.pos += 1; // consume the indefinite header byte
        return Ok(None);
    }
    let n = de.with_decoder(move |d| {
        match want {
            Major::Array => d.array_len(),
            _ => d.map_len(),
        }
        .map_err(|e| match e {
            const_cbor::Error::TYPE_MISMATCH => Error::custom("expected container"),
            other => other.into(),
        })
    })?;
    Ok(Some(n))
}

/// Major type 7 leaf dispatch.
fn simple_any<'de, V>(de: &mut Deserializer<'de>, visitor: V) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    let initial = de.rest().first().copied().unwrap_or(0xe0 | 31);
    match initial & 0x1f {
        20 | 21 => {
            let b = de.with_decoder(|d| d.bool().map_err(Error::from))?;
            visitor.visit_bool(b)
        }
        22 | 23 => {
            de.with_decoder(consume_null_like)?;
            visitor.visit_unit()
        }
        25..=27 => {
            let v = de.with_decoder(|d| d.f64().map_err(Error::from))?;
            visitor.visit_f64(v)
        }
        _ => {
            let s = de.with_decoder(|d| d.simple().map_err(Error::from))?;
            visitor.visit_u8(s.0)
        }
    }
}

/// Tag handling in the `any` path: bignum tags become numbers, everything
/// else unwraps transparently.
fn tag_any<'de, V>(de: &mut Deserializer<'de>, visitor: V) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    use serde::Deserializer as _;
    let tag = de.with_decoder(|d| d.tag().map_err(Error::from))?;
    #[cfg(feature = "bignum")]
    if matches!(tag, 2..=3) {
        let negative = tag == 3;
        let raw = de.with_decoder(|d| d.bytes().map_err(Error::from))?;
        let mag = const_cbor::bignum::normalized(raw);
        if mag.len() > 16 {
            return Err(Error::custom("bignum exceeds 128 bits"));
        }
        let mut n: u128 = 0;
        for &b in mag {
            n = (n << 8) | u128::from(b);
        }
        if negative {
            let v = -1 - i128::try_from(n).map_err(|_| Error::custom("bignum overflow"))?;
            return visitor.visit_i128(v);
        }
        return visitor.visit_u128(n);
    }
    let _: u64 = tag;
    de.deserialize_any(visitor)
}

#[cfg(feature = "bignum")]
fn bignum_magnitude<'de>(de: &mut Deserializer<'de>) -> Result<(bool, &'de [u8])> {
    let tag = de.with_decoder(|d| d.tag().map_err(Error::from))?;
    let negative = match tag {
        2 => false,
        3 => true,
        _ => return Err(Error::custom("expected bignum tag")),
    };
    let raw = de.with_decoder(|d| d.bytes().map_err(Error::from))?;
    Ok((negative, const_cbor::bignum::normalized(raw)))
}

// ------------------------------------------------------------- accessors

/// One open sequence being deserialized.
struct SeqAccess<'de, 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: Option<usize>,
}

/// One open map being deserialized.
struct MapAccess<'de, 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: Option<usize>,
}

impl<'de, 'a> de::SeqAccess<'de> for SeqAccess<'de, 'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.remaining {
            Some(0) => Ok(None),
            Some(ref mut n) => {
                *n -= 1;
                seed.deserialize(&mut *self.de).map(Some)
            }
            None => {
                if self.de.take_break_raw() {
                    // Mark drained so `Drop` does not double-consume.
                    self.remaining = Some(0);
                    Ok(None)
                } else {
                    seed.deserialize(&mut *self.de).map(Some)
                }
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.remaining
    }
}

/// Visitors may stop before the end of an indefinite container (tuples
/// know their length); closing the item is our job, not theirs.
impl Drop for SeqAccess<'_, '_> {
    fn drop(&mut self) {
        if self.remaining.is_none() {
            let _ = self.de.take_break_raw();
        }
    }
}

impl<'de, 'a> de::MapAccess<'de> for MapAccess<'de, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.remaining {
            Some(0) => Ok(None),
            Some(ref mut n) => {
                *n -= 1;
                seed.deserialize(&mut *self.de).map(Some)
            }
            None => {
                if self.de.take_break_raw() {
                    // Mark drained so `Drop` does not double-consume.
                    self.remaining = Some(0);
                    Ok(None)
                } else {
                    seed.deserialize(&mut *self.de).map(Some)
                }
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        self.remaining.map(|n| n / 2)
    }
}

/// Same early-exit closeout as sequences: an abandoned indefinite map
/// must still consume its terminating break.
impl Drop for MapAccess<'_, '_> {
    fn drop(&mut self) {
        if self.remaining.is_none() {
            let _ = self.de.take_break_raw();
        }
    }
}

impl Deserializer<'_> {
    /// Consumes a break marker if the next byte is one.
    fn take_break_raw(&mut self) -> bool {
        if self.rest().first() == Some(&const_cbor::head::BREAK) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

/// Enum variants encoded as a bare text string (unit variants).
struct UnitVariantAccess<'de, 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'de, 'a> de::EnumAccess<'de> for UnitVariantAccess<'de, 'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let name = seed.deserialize(&mut *self.de)?;
        Ok((name, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for UnitVariantAccess<'de, 'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        Err(Error::custom("unexpected newtype variant in string form"))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::custom("unexpected tuple variant in string form"))
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::custom("unexpected struct variant in string form"))
    }
}

/// Enum variants encoded as `{ name: payload }`; the map header was
/// consumed by `deserialize_enum`, the name by `variant_seed`.
struct PayloadVariantAccess<'de, 'a> {
    de: &'a mut Deserializer<'de>,
    marker: core::marker::PhantomData<&'de ()>,
}

impl<'de, 'a> de::EnumAccess<'de> for PayloadVariantAccess<'de, 'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let name = seed.deserialize(&mut *self.de)?;
        Ok((name, PayloadVariantAccess { de: self.de, marker: core::marker::PhantomData }))
    }
}

impl<'de, 'a> VariantAccess<'de> for PayloadVariantAccess<'de, 'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.de.with_decoder(consume_null_like)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        use serde::Deserializer as _;
        (&mut *self.de).deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        use serde::Deserializer as _;
        (&mut *self.de).deserialize_struct("", fields, visitor)
    }
}
