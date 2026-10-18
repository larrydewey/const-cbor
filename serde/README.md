# const-cbor-serde

[Serde](https://serde.rs) `Serialize` / `Deserialize` adapters for
[`const-cbor`](https://github.com/larrydewey/const-cbor), a const-first,
allocation-free CBOR (RFC 8949) codec.

The core crate stays dependency-free; this companion owns all trait
plumbing and is the only place `serde` appears in your graph.

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

let bytes = const_cbor_serde::to_vec(&Point { x: 1, y: -2 })?;
let p: Point = const_cbor_serde::from_slice(&bytes)?;
```

## Representation

RFC 8949 does not define struct/enum mappings, so this crate follows the
ecosystem conventions of `serde_cbor` / `ciborium` for interoperability:

| Rust | CBOR |
|---|---|
| structs / maps | map keyed by field name |
| enums | externally tagged: unit variants as bare strings, others as `{ name: payload }` |
| sequences | indefinite-length array |
| `Option::None`, unit | `null` |
| `char` | single-character text string |

Strings and byte slices deserialize zero-copy, borrowing from the input.
Enable the `bignum` feature to route `i128`/`u128` through CBOR tags 2/3
when values exceed 64 bits.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
