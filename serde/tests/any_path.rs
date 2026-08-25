//! `deserialize_any` coverage across every major type plus serde error
//! trait plumbing.

use const_cbor_serde::{from_slice, to_vec};
use serde::Deserialize;
use serde::de::{self, Visitor};
use std::fmt;

/// Captures whichever visit_* method the deserializer calls.
#[derive(Debug, PartialEq)]
enum Any {
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    Seq(Vec<Any>),
    Map(Vec<(Any, Any)>),
    Unit,
}

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'x> Visitor<'x> for V {
            type Value = Any;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any cbor")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Any::Bool(v))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Any::U64(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Any::I64(v))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Any::F64(v))
            }
            fn visit_borrowed_str<E>(self, v: &'x str) -> Result<Self::Value, E> {
                Ok(Any::Str(v.to_owned()))
            }
            fn visit_borrowed_bytes<E>(self, v: &'x [u8]) -> Result<Self::Value, E> {
                Ok(Any::Bytes(v.to_vec()))
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(Any::Unit)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(Any::Unit)
            }
            fn visit_some<D2: serde::Deserializer<'x>>(
                self,
                d: D2,
            ) -> Result<Self::Value, D2::Error> {
                d.deserialize_any(V)
            }
            fn visit_seq<A: de::SeqAccess<'x>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(el) = seq.next_element::<Any>()? {
                    out.push(el);
                }
                Ok(Any::Seq(out))
            }
            fn visit_map<A: de::MapAccess<'x>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(k) = map.next_key::<Any>()? {
                    let v = map.next_value::<Any>()?;
                    out.push((k, v));
                }
                Ok(Any::Map(out))
            }
            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E> {
                Ok(Any::U64(u64::try_from(v).unwrap()))
            }
            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E> {
                Ok(Any::I64(i64::try_from(v).unwrap()))
            }
        }
        d.deserialize_any(V)
    }
}

fn any(bytes: &[u8]) -> Any {
    from_slice(bytes).unwrap()
}

#[test]
fn scalars_via_any() {
    assert_eq!(any(&to_vec(&7u8).unwrap()), Any::U64(7));
    assert_eq!(any(&to_vec(&u64::MAX).unwrap()), Any::U64(u64::MAX));
    assert_eq!(any(&to_vec(&(-5i32)).unwrap()), Any::I64(-5));
    assert_eq!(any(&to_vec(&true).unwrap()), Any::Bool(true));
    assert_eq!(any(&to_vec(&false).unwrap()), Any::Bool(false));
    assert_eq!(any(&to_vec(&"text").unwrap()), Any::Str("text".into()));
    // Native byte string: encode via core directly (h'0102').
    assert_eq!(any(&[0x42, 0x01, 0x02]), Any::Bytes(vec![1, 2]));
    match any(&to_vec(&1.5f64).unwrap()) {
        Any::F64(v) => assert_eq!(v, 1.5),
        o => panic!("{o:?}"),
    }
    assert_eq!(any(&[0xf6]), Any::Unit); // null
    assert_eq!(any(&[0xf7]), Any::Unit); // undefined
    assert_eq!(any(&[0xf0]), Any::U64(16)); // simple(16)
}

#[test]
fn containers_via_any() {
    let bytes = to_vec(&vec![1u8, 2]).unwrap();
    assert_eq!(any(&bytes), Any::Seq(vec![Any::U64(1), Any::U64(2)]));

    // Definite array too.
    assert_eq!(any(&[0x82, 0x01, 0x02]), Any::Seq(vec![Any::U64(1), Any::U64(2)]));

    let m = std::collections::BTreeMap::from([("k".to_owned(), 3u8)]);
    let bytes = to_vec(&m).unwrap();
    assert_eq!(any(&bytes), Any::Map(vec![(Any::Str("k".into()), Any::U64(3))]));

    // Nested.
    let nested = vec![vec![1u8], vec![2]];
    let bytes = to_vec(&nested).unwrap();
    assert_eq!(
        any(&bytes),
        Any::Seq(vec![Any::Seq(vec![Any::U64(1)]), Any::Seq(vec![Any::U64(2)]),])
    );
}

#[test]
fn tags_unwrap_transparently_in_any() {
    // tag(1, 100) unwraps to the inner number.
    assert_eq!(any(&[0xc1, 0x18, 0x64]), Any::U64(100));
}

#[cfg(feature = "bignum")]
#[test]
fn bignums_surface_in_any() {
    use serde::de::Visitor;

    /// Captures raw u128/i128 without narrowing.
    #[derive(Debug, PartialEq)]
    enum Wide {
        U(u128),
        I(i128),
    }
    impl<'de> Deserialize<'de> for Wide {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V<E>(core::marker::PhantomData<E>);
            impl<'x, E: de::Error> Visitor<'x> for V<E> {
                type Value = Wide;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("number")
                }
                fn visit_u128<Er>(self, v: u128) -> Result<Self::Value, Er> {
                    Ok(Wide::U(v))
                }
                fn visit_i128<Er>(self, v: i128) -> Result<Self::Value, Er> {
                    Ok(Wide::I(v))
                }
                fn visit_u64<Er>(self, v: u64) -> Result<Self::Value, Er> {
                    Ok(Wide::U(u128::from(v)))
                }
                fn visit_i64<Er>(self, v: i64) -> Result<Self::Value, Er> {
                    Ok(Wide::I(i128::from(v)))
                }
            }
            d.deserialize_any(V::<D::Error>(core::marker::PhantomData))
        }
    }
    // tag(2) h'01000000000000000000' = 2^64.
    let w: Wide = from_slice(&[0xc2, 0x49, 0x01, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(w, Wide::U(u128::from(u64::MAX) + 1));
    // tag(3) below i64::MIN.
    let w: Wide = from_slice(&[0xc3, 0x49, 0x01, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(w, Wide::I(-i128::from(u64::MAX) - 2));
}

#[test]
fn option_none_is_unit_any() {
    let bytes = to_vec(&Option::<u8>::None).unwrap();
    assert_eq!(any(&bytes), Any::Unit);
    let bytes = to_vec(&Some(9u8)).unwrap();
    assert_eq!(any(&bytes), Any::U64(9));
}

// ------------------------------------------------------- error plumbing

#[test]
fn serde_error_trait_constructors() {
    use const_cbor_serde::Error;

    fn via_ser<E: serde::ser::Error>() -> E {
        E::custom("ser side")
    }
    fn via_de<E: de::Error>() -> E {
        E::custom("de side")
    }
    let e: Error = via_ser();
    assert_eq!(e.to_string(), "ser side");
    let e: Error = via_de();
    assert_eq!(e.to_string(), "de side");
    // invalid_type etc route through custom().
    fn invalid<E: de::Error>() -> E {
        de::Error::invalid_type(de::Unexpected::Bool(true), &"string")
    }
    let e: Error = invalid();
    assert!(e.to_string().contains("invalid type"));
}
