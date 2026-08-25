use const_cbor_serde::{from_slice, to_vec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Shape {
    Circle { r: f64 },
    Unit,
    Wrapped(u64),
}

#[test]
fn structs_roundtrip() {
    let p = Point { x: 1, y: -2 };
    let bytes = to_vec(&p).unwrap();
    let back: Point = from_slice(&bytes).unwrap();
    assert_eq!(p, back);
}

#[test]
fn enums_roundtrip() {
    for s in [Shape::Unit, Shape::Wrapped(42), Shape::Circle { r: 2.5 }] {
        let bytes = to_vec(&s).unwrap();
        let back: Shape = from_slice(&bytes).unwrap();
        assert_eq!(s, back);
    }
}

#[test]
fn seqs_roundtrip() {
    let v: Vec<String> = vec!["a".into(), "b".into()];
    let bytes = to_vec(&v).unwrap();
    let back: Vec<String> = from_slice(&bytes).unwrap();
    assert_eq!(v, back);

    let m = std::collections::BTreeMap::from([(1u8, "one"), (2, "two")]);
    let bytes = to_vec(&m).unwrap();
    let back: std::collections::BTreeMap<u8, &str> = from_slice(&bytes).unwrap();
    assert_eq!(m, back);
}

#[test]
fn options_and_primitives() {
    {
        let bytes = to_vec(&Some(-7i32)).unwrap();
        let back: Option<i32> = from_slice(&bytes).unwrap();
        assert_eq!(back, Some(-7));
    }
    {
        let bytes = to_vec(&3.5f64).unwrap();
        let back: f64 = from_slice(&bytes).unwrap();
        assert_eq!(back, 3.5);
    }
    {
        let bytes = to_vec(&"hello").unwrap();
        let back: &str = from_slice(&bytes).unwrap();
        assert_eq!(back, "hello");
    }
    {
        let bytes = to_vec(&'x').unwrap();
        let back: char = from_slice(&bytes).unwrap();
        assert_eq!(back, 'x');
    }
    assert_eq!(to_vec(&true).unwrap(), vec![0xf5]);
    assert_eq!(to_vec(&None::<u8>).unwrap(), vec![0xf6]);
    assert_eq!(to_vec(&()).unwrap(), vec![0xf6]);
    let b: Option<u8> = from_slice(&[0x05]).unwrap();
    assert_eq!(b, Some(5));
}
