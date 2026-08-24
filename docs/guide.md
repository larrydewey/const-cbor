# User Guide

End-to-end recipes for `const-cbor`. All examples compile as doctests or
in `examples/`.

---

## Installation

```toml
[dependencies]
const-cbor = "0.1"
const-cbor = { version = "0.1", features = ["float"] }  # with floats
```

No features are enabled by default; the core works without them.

---

## Encoding

### One item into a stack buffer

```rust
use const_cbor::Encoder;

let mut buf = [0u8; 16];
let mut enc = Encoder::new(&mut buf);
enc.u32(1000).unwrap();               // [0x19, 0x03, 0xe8]
let n = enc.written();
let (out, n) = enc.finish();          // out[..n] is the item
```

### Chaining a structure

Methods return `Result<&mut Self, _>`; containers declare their length,
then you write items in order:

```rust
# use const_cbor::{Encoder, Error};
fn write_sensor_report(enc: &mut Encoder<'_>) -> Result<&mut Encoder<'_>, Error> {
    enc.map_len(3)?            // {"id": .., "temp": .., "ok": true}
        .str("id")?.u16(0xBEEF)?
        .str("temp")?.i16(-125)?
        .str("ok")?.bool(true)
}
```

### Nested containers

Open inner scopes inline — order of calls is wire order:

```rust
# use const_cbor::{Encoder};
let mut buf = [0u8; 64];
let mut e = Encoder::new(&mut buf);
e.array_len(2).unwrap()        // ["a", ["b", "c"]]
    .str("a").unwrap()
    .array_len(2).unwrap()
    .str("b").unwrap().str("c").unwrap();
```

### Tags

`tag(n)` writes only the tag header; encode the tagged item right after:

```rust
# use const_cbor::{Encoder};
let mut buf = [0u8; 8];
let mut e = Encoder::new(&mut buf);
e.tag(1).unwrap()              // 1(epoch) 1363896240
    .u32(1363896240).unwrap();
```

Tag *semantics* (dates, big numbers, URIs…) are application concerns per
RFC 8949 §3.4; this crate preserves only the number.

### Compile-time encoding

For fixed payloads evaluated during compilation:

```rust
use const_cbor::encode::consts;

const NEG_TEN: ([u8; 2], usize) = {
    let mut buf = [0u8; 2];
    let n = consts::put_i64(&mut buf, -10);
    (buf, n)
}; // NEG_TEN.0[..1] == [0x29]
```

Overflow panics here become **compile errors**. Size helpers
(`uint_len`, `i64_len`, `chunk_len`) size buffers statically.

---

## Decoding

### Typed access

Each method consumes exactly one item and checks its type:

```rust
# use const_cbor::{Decoder};
let data = [0x19, 0x03, 0xe8]; // 1000
let mut dec = Decoder::new(&data);
assert_eq!(dec.u16().unwrap(), 1000);
assert!(dec.is_empty());
```

Narrowing is checked, never truncating: decoding `[300]` via `.u8()` is
`INTEGER_OUT_OF_RANGE`.

### Zero-copy strings and bytes

Borrows point into your input buffer:

```rust
# use const_cbor::{Decoder};
let input = [0x65, b'h', b'e', b'l', b'l', b'o'];
let mut dec = Decoder::new(&input);
let s: &str = dec.str().unwrap();
assert_eq!(s, "hello");
```

### Containers: read the header, then read items in wire order

```rust
# use const_cbor::{Decoder};
// {"temp": -25}
let mut dec = Decoder::new(&[0xa1, 0x64, b't', b'e', b'm', b'p', 0x38, 0x18]);
assert_eq!(dec.map_len().unwrap(), 1);
assert_eq!(dec.str().unwrap(), "temp");
assert_eq!(dec.i8().unwrap(), -25);
```

Use `peek_major()` for self-describing payloads to branch on the next
item's type.

### Skipping unknown fields

Forward compatibility in one call — any item, any depth:

```rust
# use const_cbor::{Decoder};
// [1, {"legacy": [255, "ignored"]}]
let item = [0x82, 0x01, 0xa1, 0x66, b'l', b'e', b'g', b'a', b'c', b'y',
            0x82, 0x18, 0xff, 0x67, b'i', b'g', b'n', b'o', b'r', b'e', b'd'];
let mut dec = Decoder::new(&item);
assert_eq!(dec.array_len().unwrap(), 2);
assert_eq!(dec.u8().unwrap(), 1);
dec.skip().unwrap();               // unknown map skipped whole
assert!(dec.is_empty());
```

Nesting beyond 32 levels returns `DEPTH_EXCEEDED` instead of overflowing
the stack — safe on bare-metal targets.

### Tags on input

```rust
# use const_cbor::{Decoder};
let mut dec = Decoder::new(&[0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]);
assert_eq!(dec.tag().unwrap(), 1);       // semantic layer decides what 1 means
assert_eq!(dec.u32().unwrap(), 1363896240);
```

---

## Errors

Every failure is one small enum value (`Copy`, no allocation):

| Code | Meaning |
|---|---|
| `OUT_OF_SPACE` | output buffer too small |
| `TRUNCATED` | input ended mid-item |
| `INVALID_ADDITIONAL_INFO` | reserved ai 28–31 |
| `INTEGER_OUT_OF_RANGE` | value exceeds requested Rust type |
| `TYPE_MISMATCH` | wrong major type for method |
| `INVALID_UTF8` | text payload violates RFC 3629 |
| `INVALID_SIMPLE` | two-byte simple < 32 (forbidden by §3.3) |
| `FLOAT_UNSUPPORTED` | float item without the `float` feature |
| `DEPTH_EXCEEDED` | skip nesting > 32 |

Full behavioral rules: [`behavior.md`](behavior.md).

---

## Embedded / no_std notes

* The crate is `#![no_std]`; nothing ever allocates or blocks.
* Verified to build for `thumbv7m-none-eabi`; full feature set costs
  ~1.2 KB of flash.
* Prefer `#[panic_handler]`-safe targets: the crate never panics at
  runtime regardless.
