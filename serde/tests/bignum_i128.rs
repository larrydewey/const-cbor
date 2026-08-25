#![cfg(feature = "bignum")]
//! i128/u128 through bignum tags 2/3 (feature `bignum`).

use const_cbor_serde::{from_slice, to_vec};

fn roundtrip(v: i128) {
    let bytes = to_vec(&v).unwrap();
    println!("{v} -> {bytes:02x?}");
    let back: i128 = match from_slice(&bytes) {
        Ok(b) => b,
        Err(e) => panic!("v={v} bytes={bytes:02x?} err={e:?}"),
    };
    assert_eq!(back, v, "bytes {bytes:02x?}");
}

#[test]
fn i128_full_range() {
    roundtrip(0);
    roundtrip(-1);
    roundtrip(1);
    // Beyond u64/i64 forces the bignum path.
    roundtrip(i64::MAX as i128 + 1);
    roundtrip(i128::from(i64::MIN) - 1);
    roundtrip(i128::MAX);
    roundtrip(i128::MIN);
}

#[test]
fn small_values_stay_basic_integers() {
    // Preferred serialization: values within i64 never use tags 2/3.
    let bytes = to_vec(&42i128).unwrap();
    assert_eq!(bytes, vec![0x18, 0x2a]);
    let bytes = to_vec(&(-42i128)).unwrap();
    assert_eq!(bytes, vec![0x38, 0x29]);
}

#[test]
fn u128_roundtrip() {
    for v in [0u128, 1, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
        let bytes = to_vec(&v).unwrap();
        let back: u128 = from_slice(&bytes).unwrap();
        assert_eq!(back, v, "bytes {bytes:02x?}");
    }
}

#[test]
fn any_path_yields_128bit() {
    // deserialize_any via an enum-free route: JSON-ish Value isn't used;
    // instead check through a newtype that captures any number.
    #[derive(Debug)]
    struct AnyNum(i128);
    impl<'de> serde::Deserialize<'de> for AnyNum {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V<E>(core::marker::PhantomData<E>);
            impl<'x, Er: serde::de::Error> serde::de::Visitor<'x> for V<Er> {
                type Value = AnyNum;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("number")
                }
                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                    Ok(AnyNum(i128::from(v)))
                }
                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
                    Ok(AnyNum(i128::from(v)))
                }
                fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E> {
                    Ok(AnyNum(v as i128))
                }
                fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E> {
                    Ok(AnyNum(v))
                }
            }
            d.deserialize_any(V::<D::Error>(core::marker::PhantomData))
        }
    }
    let big = to_vec(&(i64::MAX as i128 + 5)).unwrap();
    // Tag 2 wrapping h'800000000000000004'.
    assert_eq!(big[0], 0xc2);
    let n: AnyNum = from_slice(&big).unwrap();
    assert_eq!(n.0, i64::MAX as i128 + 5);

    let neg = to_vec(&(i128::from(i64::MIN) - 5)).unwrap();
    assert_eq!(neg[0], 0xc3);
    let n: AnyNum = from_slice(&neg).unwrap();
    assert_eq!(n.0, i128::from(i64::MIN) - 5);
}
