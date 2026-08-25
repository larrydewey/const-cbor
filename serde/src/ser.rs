//! `serde::Serializer` adapter over [`const_cbor::Encoder`].

use crate::error::{Error, Result};
use const_cbor::Encoder;
use serde::Serialize;

/// Serializes serde values into a caller-provided fixed buffer.
///
/// Sequences and maps use indefinite lengths (the core encoder requires
/// known lengths for definite forms), so the output is well-formed CBOR
/// but not necessarily preferred serialization; feed the result through
/// `const_cbor::deterministic::check` if you need §4.2 guarantees.
pub struct Serializer<'b> {
    pub(crate) enc: Encoder<'b>,
}

impl<'b> Serializer<'b> {
    /// Creates a serializer writing into `buf`.
    #[must_use]
    pub fn new(buf: &'b mut [u8]) -> Self {
        Self { enc: Encoder::new(buf) }
    }

    /// Bytes written so far.
    #[must_use]
    pub fn written(&self) -> usize {
        self.enc.written()
    }
}

/// All sequence-like collectors share one implementation: indefinite
/// array closed on drop-free `end`.
pub struct SeqSerializer<'s, 'b> {
    ser: &'s mut Serializer<'b>,
}

impl<'s, 'b> serde::ser::SerializeSeq for SeqSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.enc.end_indef()?;
        Ok(())
    }
}

impl<'s, 'b> serde::ser::SerializeTuple for SeqSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl<'s, 'b> serde::ser::SerializeTupleStruct for SeqSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl<'s, 'b> serde::ser::SerializeTupleVariant for SeqSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        // Close the indefinite payload array; the variant's outer
        // one-entry map is definite and already complete.
        self.ser.enc.end_indef()?;
        Ok(())
    }
}

pub struct MapSerializer<'s, 'b> {
    ser: &'s mut Serializer<'b>,
}

impl<'s, 'b> serde::ser::SerializeMap for MapSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        self.ser.enc.end_indef()?;
        Ok(())
    }
}

impl<'s, 'b> serde::ser::SerializeStruct for MapSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        self.ser.enc.str(key)?;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        serde::ser::SerializeMap::end(self)
    }
}

impl<'s, 'b> serde::ser::SerializeStructVariant for MapSerializer<'s, 'b> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        serde::ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        // Close the indefinite field map; the variant's outer one-entry
        // map is definite and already complete.
        self.ser.enc.end_indef()?;
        Ok(())
    }
}

impl<'s, 'b> serde::Serializer for &'s mut Serializer<'b> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SeqSerializer<'s, 'b>;
    type SerializeTuple = SeqSerializer<'s, 'b>;
    type SerializeTupleStruct = SeqSerializer<'s, 'b>;
    type SerializeTupleVariant = SeqSerializer<'s, 'b>;
    type SerializeMap = MapSerializer<'s, 'b>;
    type SerializeStruct = MapSerializer<'s, 'b>;
    type SerializeStructVariant = MapSerializer<'s, 'b>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.enc.bool(v)?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.enc.i64(v)?;
        Ok(())
    }

    #[cfg(feature = "bignum")]
    fn serialize_i128(self, v: i128) -> Result<()> {
        if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
            return self.serialize_i64(v as i64);
        }
        if v >= 0 {
            let mag = (v as u128).to_be_bytes();
            self.enc.bignum_uint(&mag)?;
        } else {
            let mag = (!(v as u128)).to_be_bytes();
            self.enc.bignum_nint(&mag)?;
        }
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.enc.u8(v)?;
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.enc.u16(v)?;
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.enc.u32(v)?;
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.enc.uint(v)?;
        Ok(())
    }

    #[cfg(feature = "bignum")]
    fn serialize_u128(self, v: u128) -> Result<()> {
        if v <= u64::MAX as u128 {
            return self.serialize_u64(v as u64);
        }
        self.enc.bignum_uint(&v.to_be_bytes())?;
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.enc.f32(v)?;
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.enc.f64(v)?;
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        let mut tmp = [0u8; 4];
        self.enc.str(v.encode_utf8(&mut tmp))?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.enc.str(v)?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.enc.bytes(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.enc.null()?;
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.enc.null()?;
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.enc.str(variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.enc.map_len(1)?;
        self.enc.str(variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.enc.begin_array()?;
        Ok(SeqSerializer { ser: self })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        // { variant: [ ... ] } as two nested opens.
        self.enc.map_len(1)?;
        self.enc.str(variant)?;
        self.enc.begin_array()?;
        Ok(SeqSerializer { ser: self })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.enc.begin_map()?;
        Ok(MapSerializer { ser: self })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.enc.begin_map()?;
        Ok(MapSerializer { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.enc.map_len(1)?;
        self.enc.str(variant)?;
        self.enc.begin_map()?;
        Ok(MapSerializer { ser: self })
    }
}
