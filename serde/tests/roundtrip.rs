//! Exhaustive roundtrip coverage: every serde data-model element, enum
//! variant shapes, containers, options, zero-copy checks, and error paths.

use const_cbor_serde::{from_slice, to_slice, to_vec};
use serde::{Deserialize, Serialize};
use serde_bytes::{ByteBuf, Bytes};
use std::collections::BTreeMap;

fn roundtrip<T>(v: T)
where
    T: for<'de> Deserialize<'de> + Serialize + PartialEq + std::fmt::Debug,
{
    let bytes = to_vec(&v).unwrap();
    let back: T = from_slice(&bytes).unwrap();
    assert_eq!(v, back);
}

#[test]
fn primitives() {
    assert_eq!(to_vec(&true).unwrap(), vec![0xf5]);
    assert_eq!(to_vec(&false).unwrap(), vec![0xf4]);
    assert_eq!(to_vec(&()).unwrap(), vec![0xf6]);
    assert_eq!(to_vec(&7u8).unwrap(), vec![0x07]);
    assert_eq!(to_vec(&1000u32).unwrap(), vec![0x19, 0x03, 0xe8]);
    assert_eq!(to_vec(&-1i8).unwrap(), vec![0x20]);
    assert_eq!(to_vec(&-1000i16).unwrap(), vec![0x39, 0x03, 0xe7]);
    assert_eq!(to_vec(&1.5f64).unwrap(), vec![0xf9, 0x3e, 0x00]);
    assert_eq!(to_vec("hi").unwrap(), vec![0x62, b'h', b'i']);
    assert_eq!(to_vec(&'A').unwrap(), vec![0x61, b'A']);
}

#[test]
fn primitive_roundtrips_all_widths() {
    roundtrip(0u8);
    roundtrip(255u8);
    roundtrip(65_535u16);
    roundtrip(u64::MAX);
    roundtrip(i64::MIN);
    roundtrip(-1i32);
    roundtrip(f64::MIN);
    roundtrip(true);
    roundtrip(String::from("hello world"));
    roundtrip('λ');
}

#[test]
fn option_and_unit() {
    roundtrip(None::<u32>);
    roundtrip(Some(42u32));
    roundtrip(vec![None::<u8>, Some(1)]);
    // None is null (f6), Some is transparent.
    assert_eq!(to_vec(&Option::<u8>::None).unwrap(), vec![0xf6]);
    assert_eq!(to_vec(&Some(1u8)).unwrap(), vec![0x01]);
}

#[test]
fn sequences_and_maps() {
    roundtrip(vec![1u8, 2, 3]);
    roundtrip(Vec::<String>::new());
    roundtrip(BTreeMap::from([("a".to_owned(), 1), ("bb".to_owned(), 2)]));
    roundtrip(vec![ByteBuf::from(b"\x00\xff")]);
    roundtrip((1u8, String::from("two"), 3.0));
    // Indefinite array encoding for Vec.
    let bytes = to_vec(&vec![1u8, 2]).unwrap();
    assert_eq!(bytes, vec![0x9f, 0x01, 0x02, 0xff]);
}

#[test]
fn nested_structures() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Inner {
        values: Vec<u32>,
        meta: BTreeMap<String, u8>,
    }
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Outer {
        inner: Inner,
        tag: Option<String>,
    }
    let v = Outer {
        inner: Inner { values: vec![1, 2, 3], meta: BTreeMap::from([("k".into(), 9)]) },
        tag: Some("t".into()),
    };
    let bytes = to_vec(&v).unwrap();
    let back: Outer = from_slice(&bytes).unwrap();
    assert_eq!(v, back);

    // Deeply nested collections.
    let deep: BTreeMap<String, Vec<Option<BTreeMap<u8, u8>>>> =
        BTreeMap::from([("root".into(), vec![Some(BTreeMap::from([(1, 2)])), None])]);
    let bytes = to_vec(&deep).unwrap();
    let back = from_slice(&bytes).unwrap();
    assert_eq!(deep, back);
}

#[test]
fn newtype_structs_transparent() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Meters(u64);
    let bytes = to_vec(&Meters(5)).unwrap();
    assert_eq!(bytes, vec![0x05]); // transparent wrapper
    let back: Meters = from_slice(&bytes).unwrap();
    assert_eq!(back, Meters(5));
}

#[test]
fn enums_all_variant_shapes() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum E {
        Bare,
        Newtype(u32),
        Tuple(u8, String),
        Struct { a: u8, b: bool },
    }
    for e in [E::Bare, E::Newtype(7), E::Tuple(1, "x".into()), E::Struct { a: 2, b: true }] {
        let bytes = to_vec(&e).unwrap();
        let back: E = from_slice(&bytes).unwrap();
        assert_eq!(e, back, "{bytes:?}");
    }
    // Unit variants are bare strings.
    assert_eq!(to_vec(&E::Bare).unwrap(), [0x64, b'B', b'a', b'r', b'e']); // text(4) "Bare"

    // Newtype variants are one-entry maps with definite length.
    let bytes = to_vec(&E::Newtype(7)).unwrap();
    assert_eq!(&bytes[..3], &[0xa1, 0x67, b'N']); // map(1), "Newtype"
}

#[test]
fn nan_roundtrips_bit_exact() {
    let bytes = to_vec(&f64::NAN).unwrap();
    assert_eq!(bytes, vec![0xf9, 0x7e, 0x00]); // canonical NaN
    let back: f64 = from_slice(&bytes).unwrap();
    assert!(back.is_nan());
}

#[test]
fn borrowed_zero_copy() {
    let bytes = to_vec(&("text", Bytes::new(b"bin"))).unwrap();
    let (s, b): (&str, &serde_bytes::Bytes) = from_slice(&bytes).unwrap();
    assert_eq!(s, "text");
    assert_eq!(&**b, b"bin");
    // The string borrow really points into `bytes`.
    let text_ptr = &bytes[2] as *const u8;
    assert_eq!(s.as_ptr(), text_ptr);
}

#[test]
fn trailing_bytes_rejected() {
    let mut bytes = to_vec(&1u8).unwrap();
    bytes.push(0x02);
    let r: Result<u8, _> = from_slice(&bytes);
    assert!(r.is_err());
    let r: Result<u8, _> = from_slice(&[0x01, 0x02]);
    assert!(r.is_err(), "trailing bytes must be rejected");
}

#[test]
fn type_errors_surface() {
    let bytes = to_vec("str").unwrap();
    let r: Result<u8, _> = from_slice(&bytes);
    assert!(matches!(r, Err(const_cbor_serde::Error::Cbor(const_cbor::Error::TYPE_MISMATCH))));

    let r: Result<Vec<u8>, _> = from_slice(&[0x01]);
    assert!(r.is_err());

    let empty: &[u8] = &[];
    let r: Result<u8, _> = from_slice(empty);
    assert!(matches!(r, Err(const_cbor_serde::Error::Cbor(const_cbor::Error::TRUNCATED))));
}

#[test]
fn out_of_space_reported() {
    struct Big;
    impl serde::Serialize for Big {
        fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let bytes = [0u8; 300];
            ser.serialize_bytes(&bytes)
        }
    }
    let mut tiny = [0u8; 10];
    assert!(matches!(
        to_slice(&Big, &mut tiny),
        Err(const_cbor_serde::Error::Cbor(const_cbor::Error::OUT_OF_SPACE))
    ));
}
