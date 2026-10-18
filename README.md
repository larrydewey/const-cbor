# const-cbor

[![CI](https://github.com/larry-shobaki/const-cbor/actions/workflows/ci.yml/badge.svg)](https://github.com/larry-shobaki/const-cbor/actions)
[![RFC 8949](https://img.shields.io/badge/RFC-8949-blue)](https://www.rfc-editor.org/rfc/rfc8949)
[![no_std](https://img.shields.io/badge/no__std-yes-success)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-red)](https://doc.rust-lang.org/nomicon/)
[![deps](https://img.shields.io/badge/deps-zero-lightgrey)]()

A **const-first**, **allocation-free** CBOR codec implementing
[RFC 8949](https://www.rfc-editor.org/rfc/rfc8949). Encode and decode
CBOR directly between Rust values and byte buffers — no value tree, no
heap, no `unsafe`, no traits, no dependencies.

```
flash on Cortex-M3 (thumbv7m-none-eabi, `profile.size`: opt-level=z,
fat LTO, panic=abort, symbols stripped), linked `.text` + `.rodata`,
inputs black_boxed so nothing is const-folded away:

  core only ................................ 3,148 B
  + float (encode + decode exercised) ...... 4,964 B
  + indefinite ............................. 4,496 B
  + bignum ................................. 3,994 B
  + canonical .............................. 4,000 B
  + deterministic (incl. indefinite) ....... 4,496 B
  + diagnostic ............................. 7,462 B
  + diagnostic + float decimal text ........ 21,578 B
  everything enabled and used .............. 24,102 B

Features you do not enable — or call — cost nothing.

Reproduce with the checked-in probe:
`cargo build --profile size --target thumbv7m-none-eabi --example cbor-size`
then `size target/thumbv7m-none-eabi/size/examples/cbor-size`. The same
probe builds and runs on x86_64 (`cargo run --profile size --example cbor-size`),
where the linked `const_cbor` symbols can be inspected directly with
`nm target/size/examples/cbor-size | grep const_cbor`.
```

## Why another CBOR crate?

| | const-cbor | serde-based stacks |
|---|---|---|
| Dependencies | 0 | dozens (transitively) |
| Heap allocation | never | pervasive |
| `unsafe` | forbidden by lint | present in deps |
| Compile-time work | heads, widths, lengths, whole items via `const fn` | runtime |
| Binary size (Cortex-M) | ~1–4 KB | tens of KB |
| API model | concrete-typed methods, chaining | trait impls + derive |

Need serde? The [`const-cbor-serde`](serde/) companion crate adds
`Serialize`/`Deserialize` adapters over this core — the core itself keeps
zero dependencies.

The design pushes work into the compiler: argument widths, head bytes,
and encoded sizes are `const fn`s that LLVM folds for statically known
inputs — entire items can be built *during* compilation:

```rust
use const_cbor::encode::consts;

/// [-10] — one byte of output, built and checked at compile time.
const NEG_TEN: ([u8; 2], usize) = {
    let mut buf = [0u8; 2];
    let n = consts::put_i64(&mut buf, -10);
    (buf, n)
};
assert_eq!(&NEG_TEN.0[..NEG_TEN.1], &[0x29]);
```

(Overflowing the buffer here is a **compile error**, not a runtime panic.)

## Quick start

```toml
[dependencies]
const-cbor = "0.2"
```

### Encode

```rust
use const_cbor::Encoder;

let mut buf = [0u8; 64];
let mut enc = Encoder::new(&mut buf);

// [1000, "hello"]
enc.array_len(2).unwrap();
enc.u32(1000).unwrap();
enc.str("hello").unwrap();

let n = enc.written();
assert_eq!(&buf[..n], &[0x82, 0x19, 0x03, 0xe8, 0x65,
                        b'h', b'e', b'l', b'l', b'o']);
```

Methods return `Result<&mut Self, _>` so calls chain; containers declare
their length and you write their items in wire order. Every call either
writes completely or writes nothing (`OUT_OF_SPACE` keeps the cursor).

### Decode

```rust
use const_cbor::{Decoder};

let mut dec = Decoder::new(&buf[..n]);
assert_eq!(dec.array_len().unwrap(), 2);
assert_eq!(dec.u32().unwrap(), 1000);
assert_eq!(dec.str().unwrap(), "hello");   // zero-copy borrow
assert!(dec.is_empty());
```

Strings and byte strings borrow from your input buffer. Typed getters are
range-checked, never truncating.

## Features

Nothing beyond the core is compiled unless you ask:

```toml
[dependencies]
const-cbor = { version = "0.2", features = ["float", "indefinite"] }
```

| Feature | Capability | RFC section |
|---|---|---|
| *(default)* | Integers (full u64/i64 space), strings, bytes, arrays, maps, tags, simple values, booleans, null/undefined, structural skip | §3.1, §5 |
| `float` | f16/f32/f64 with provably-lossless width narrowing and canonical NaN | §3.3, App. D |
| `indefinite` | Indefinite-length strings (chunk-wise, zero-copy iterators) and open-ended arrays/maps | §3.2 |
| `raw` | `RawValue` — validated zero-copy handle to any complete item; lazily re-decodable, verbatim re-emittable | §5.4 |
| `stream` | `StreamDecoder`: incremental decode across arbitrary feed boundaries into one caller-provided scratch buffer | — |
| `canonical` | `MapBuilder<N>`: deterministic map key ordering without allocation | §4.2 |
| `diagnostic` | Human-readable rendering of CBOR into RFC 8949 §8 diagnostic notation | §8 |
| `bignum` | Arbitrary-precision integers via tags 2/3, zero-copy magnitudes | §3.4.3 |
| `deterministic` | Strict RFC §4.2 validation: minimal widths, shortest floats, sorted map keys | §4.2 |

## Serde support

The companion crate `serde/` (workspace member) adds `Serialize` /
`Deserialize` adapters over the core codec:

```toml
[dependencies]
const-cbor-serde = { path = "serde" }   # or from the registry
```

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

let bytes = const_cbor_serde::to_vec(&Point { x: 1, y: -2 })?;
let p: Point = const_cbor_serde::from_slice(&bytes)?;
```

Representation follows ecosystem conventions (`serde_cbor` / `ciborium`):
structs are CBOR maps keyed by field name, enums are externally tagged,
sequences use indefinite lengths. The core crate itself remains
dependency-free.
| `std` | Host conveniences; the core never requires it | — |

### Feature examples

Indefinite strings, chunked both ways:

```rust
# use const_cbor::Encoder;
let mut buf = [0u8; 32];
let mut e = Encoder::new(&mut buf);
e.str_indef(&["strea", "ming"]).unwrap();   // (_ "strea", "ming")

let mut d = const_cbor::Decoder::new(&buf);
let mut chunks = d.str_chunks().unwrap();
while let Some(chunk) = chunks.next() {
    // borrows straight from `buf`
}
```

Canonical map ordering without a heap:

```rust
use const_cbor::{Decoder, Encoder};
use const_cbor::canonical::MapBuilder;

let mut buf = [0u8; 64];
let mut enc = Encoder::new(&mut buf);
enc.map_len(2).unwrap();
let mut m = MapBuilder::<2>::new(&mut enc);
m.add(|k| k.str("bb"), |v| v.u8(1)).unwrap();
m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
m.finish(&mut [0u8; 32]).unwrap();          // reorders into canonical order

let mut dec = Decoder::new(&buf);
dec.map_len().unwrap();
assert_eq!(dec.str().unwrap(), "aa");       // "aa" pair now comes first

// Note: canonical order is *bytewise over encoded keys* — the length
// prefix participates. "zeta" (4 bytes) canonically precedes "alpha"
// (5 bytes), unlike plain ASCII ordering.
```

Streaming decode over packet-sized feeds:

```rust
use const_cbor::stream::StreamDecoder;

let mut scratch = [0u8; 256];               // bounds max item size
let mut stream = StreamDecoder::new(&mut scratch);
stream.feed(&wire_fragments[0]);
stream.feed(&wire_fragments[1]);
if let Ok(Some(mut item)) = stream.take_item() {
    // `item` is a Decoder over one complete CBOR data item
}
```

Human-readable diagnostic output:

```rust
use const_cbor::{Decoder, Encoder};

let mut buf = [0u8; 64];
let mut enc = Encoder::new(&mut buf);
enc.array_len(3).unwrap();
enc.str("a").unwrap().i16(-25_000).unwrap().bool(true).unwrap();
let (cbor, n) = enc.finish();

let mut text = [0u8; 128];
let pretty = const_cbor::diagnostic::render(&cbor[..n], &mut text).unwrap();
assert_eq!(pretty, "[\"a\", -25000, true]");
```

Passing unknown fields through untouched:

```rust
let raw = decoder.raw_value()?;     // validates structure only
encoder.raw(&raw)?;                 // re-emits verbatim later
```

## Guarantees

| Property | Guarantee |
|---|---|
| Panics | never at runtime — malformed input is always `Err(Error)` |
| Allocation | never; links only `core`, no allocator touched |
| Unsafe | `#![forbid(unsafe_code)]` |
| Recursion | none — skip uses a fixed 32-entry stack |
| Failure atomicity | failed encoder calls leave the buffer unchanged |
| Zero-copy | string/byte payloads borrow from caller memory |
| Errors | single-byte `Copy` codes implementing `core::error::Error` |

Strictness follows RFC 8949 §4.2.1: non-minimal integer encodings are
accepted on input (preferred form is a SHOULD); genuinely invalid forms —
reserved additional information, two-byte simples below 32, non-UTF-8
text, stray breaks — are rejected.

## Documentation

* [`docs/guide.md`](docs/guide.md) — task-oriented recipes
* [`docs/behavior.md`](docs/behavior.md) — exact runtime semantics
* [`docs/conformance.md`](docs/conformance.md) — clause-by-clause RFC conformance matrix
* [`specs/RFC8949.md`](specs/RFC8949.md) — normative source
* rustdoc: `cargo doc --open`

## Development

```sh
cargo test                          # core test suite
cargo test --all-features           # everything
cargo clippy --all-features --all-targets
cargo fmt --check
cargo run  --example const_eval     # compile-time encoding demo
cargo fuzz run decode               # fuzzing (cargo-fuzz)
```

CI runs tests across every feature combination, clippy with
`-D warnings`, cross-builds for `thumbv7m-none-eabi` /
`aarch64-unknown-none`, and gates release binary size.

Minimum supported Rust: **1.85** (edition 2024, stable only).

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
