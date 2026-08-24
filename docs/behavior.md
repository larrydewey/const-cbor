# Runtime Behavior

This document specifies exactly how `const-cbor` behaves at runtime:
guarantees, failure modes, and edge-case semantics. Everything here is
enforced by tests in `tests/`.

---

## Global Guarantees

| Property          | Guarantee                                                        |
|-------------------|------------------------------------------------------------------|
| Panics            | **Never** at runtime. Malformed input always returns `Err`.      |
| Heap allocation   | **Never**. The crate links only `core`; no allocator is touched. |
| Unsafe code       | **Forbidden** (`#![forbid(unsafe_code)]`).                       |
| Recursion         | **None.** `Decoder::skip` is iterative with a fixed 32-entry stack. |
| Dependencies      | **Zero.**                                                        |
| Blocking / syscalls | **None.** Pure functions over byte slices.                    |

The only panicking code lives in `encode::consts`, whose panics are a
*feature*: they convert buffer overflow into a compile-time error during
`const` evaluation. It must not be called with runtime-variable buffers.

---

## Encoding Behavior

### Cursor discipline (failure atomicity)

Every encoder method either writes its complete item and advances the
cursor, or writes **nothing** and leaves the cursor untouched
(`OUT_OF_SPACE`). A failed call never corrupts prior output:

```rust
let mut buf = [0u8; 3];
let mut enc = Encoder::new(&mut buf);
enc.u8(7).unwrap();                      // cursor = 1
assert!(enc.str("toolong").is_err());    // rejected whole
assert_eq!(enc.written(), 1);            // still 1, item reusable
```

### Preferred serialization (RFC 8949 §4.2)

The encoder always emits the shortest form:

* integers use inline arguments below 24, then 1/2/4/8-byte extensions;
* negative integers are encoded as `-1 - n` via bitwise NOT, so
  `i64::MIN` is handled without overflow;
* floats narrow to half/single width **only when the widening equals the
  original value exactly** (round-then-verify); anything else widens;
* NaN encodes canonically as `f9 7e 00`;
* nonempty containers open a scope; you write their items yourself.

### Chaining semantics

Methods return `Result<&mut Self, Error>` so calls chain. After an error
you may retry or continue writing smaller items; the encoder remains valid
until `finish()` consumes it.

### Compile-time encoding

`encode::consts::{put_u64, put_i64}` panic on overflow instead of returning
errors, which turns insufficient buffers into compile errors inside `const`
blocks. Length helpers (`uint_len`, `i64_len`, `chunk_len`) are plain const
functions usable anywhere.

---

## Decoding Behavior

### Error precedence

For each item the decoder checks, in order:

1. `TRUNCATED` — input ends mid-head or mid-payload;
2. `INVALID_ADDITIONAL_INFO` — reserved ai values 28–31;
3. `TYPE_MISMATCH` — next item's major type ≠ requested type;
4. value-domain errors: `INTEGER_OUT_OF_RANGE`, `INVALID_UTF8`,
   `INVALID_SIMPLE`, `FLOAT_UNSUPPORTED`.

A type mismatch consumes nothing; the decoder stays positioned.

### Strictness policy (RFC 8949 §4.2.1)

| Case                                | Behavior                                   |
|-------------------------------------|--------------------------------------------|
| Non-minimal integer argument        | **Accepted** (preferred form is a SHOULD). |
| Two-byte simple value < 32          | **Rejected** (`INVALID_SIMPLE`) — forbidden by §3.3. |
| Reserved ai 28–30                   | Rejected.                                  |
| Indefinite length (ai = 31)         | Rejected as invalid additional info.       |
| Invalid UTF-8 text payload          | Rejected (`INVALID_UTF8`), payload not borrowed. |
| Declared length exceeding input     | Rejected (`TRUNCATED`).                    |

### Zero-copy contract

`str()` and `bytes()` return borrows into the caller's input slice with
lifetime `'a`, independent of the decoder's own borrow. No copying occurs;
UTF-8 validation happens before the borrow escapes.

### Typed narrowing

`u8()`–`u32()` reject values exceeding the target range
(`INTEGER_OUT_OF_RANGE`) rather than truncating. `i64()` accepts both major
types 0 and 1; magnitudes beyond `-i64::MIN` are out of range.

### Lookahead and nulls

`peek_major()` inspects the next major type without consuming.
`is_null()` consumes the item only if it *is* null; otherwise it leaves
position and returns `Ok(false)`.

### Skipping

`skip()` consumes exactly one complete item of any type. Nested containers
are traversed iteratively using a fixed 32-slot counter stack; nesting
deeper than [`SKIP_LIMIT`] fails with `DEPTH_EXCEEDED` rather than risking
stack exhaustion. Tags count as one level (tagged item follows immediately).
Floats and all other leaves skip without interpretation, so skipping works
identically with the `float` feature disabled.

---

## Feature Matrix

| Feature | Effect when enabled | Cost when disabled |
|---|---|---|
| *(default)* | Integers, strings, bytes, containers, tags, simples | — |
| `float` | `Encoder::{f16,f32,f64}`, `Decoder::{f32,f64}` | Float methods don't exist; float *items* still skippable |
| `indefinite` | chunked/open-ended items (§3.2); ai=31 accepted where legal | ai=31 rejected with `INDEFINITE_UNSUPPORTED` |
| `raw` | `RawValue` capture/re-emit; implies `indefinite` | — |
| `stream` | `StreamDecoder` over caller scratch; implies `indefinite` | — |
| `canonical` | `MapBuilder<N>` deterministic key ordering | — |
| `diagnostic` | `render()` / `write_to()`: CBOR → RFC 8949 §8 diagnostic notation; iterative over fixed stack; floats need `float`, else render as `float(0x…)` | — |
| `std` | Reserved for host conveniences; core never requires it | No effect |

New error codes introduced alongside: `UNEXPECTED_BREAK`,
`INDEFINITE_UNSUPPORTED`, plus reuse of `DEPTH_EXCEEDED` for encoder scope
overflow (fixed 16-level indefinite nesting) and `MapBuilder` capacity.

---

## Size and Performance Profile

Measured on Cortex-M3 (`thumbv7m-none-eabi`, opt-z, LTO, abort-on-panic):
~1.2 KB of `.text` for the full feature set. All operations are branchy
byte loops over slices; there are no allocations, locks, or dynamic
dispatch anywhere on any path.
