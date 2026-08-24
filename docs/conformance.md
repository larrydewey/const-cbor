# RFC 8949 Conformance Matrix

Normative source: RFC 8949 (`specs/RFC8949.md`). Status values:
**full** (implemented and tested), **partial**, **out of scope**,
**n/a** (not applicable to a non-streaming codec).

---

## Section 3 — Encoding Specification

### §3.1 Major types

| Item | Status | Notes |
|---|---|---|
| 0 unsigned integers, full u64 range | full | preferred widths; `Encoder::u64` |
| 1 negative integers, n up to 2^64-1 | full | `nint` covers beyond i64::MIN; typed `i*` range-checked |
| 2 byte strings, definite length | full | zero-copy on input |
| 3 text strings, definite length | full | UTF-8 validated per §3.1 |
| 4 arrays, definite length | full | header + caller-written items |
| 5 maps, definite length | full | pairs counted; key ordering is caller's concern |
| 6 tags | full | number only; semantics external per spec |
| 7 simple values / floats | full / partial | simples complete; floats behind `float` |

### §3.2 Indefinite lengths

**Full** behind the `indefinite` feature (previously excluded).

| Requirement | Status | API |
|---|---|---|
| Chunked byte/text strings on output | full | `Encoder::{begin_bytes,begin_str,str_chunk,bytes_chunk,end_str,end_bytes}` and one-shot `{str_indef,bytes_indef}` |
| Open-ended arrays/maps + break | full | `Encoder::{begin_array,begin_map,end_indef}` with a 16-scope stack |
| Chunk-wise zero-copy string input | full | `Decoder::{str_chunks,bytes_chunks}` iterators + `sync_chunks` |
| Break handling in structural skip | full | nested indefinite scopes bounded by `SKIP_LIMIT`; stray break → `UNEXPECTED_BREAK` |

Without the feature the decoder rejects ai = 31
(`INDEFINITE_UNSUPPORTED`).

### §3.3 Simple values and floats

| Requirement | Status | Notes |
|---|---|---|
| One-byte simples 0–23 | full | inline ai |
| Two-byte form for 32–255 | full | |
| Two-byte form **forbidden** below 32 | full | rejected with `INVALID_SIMPLE` on decode, `INVALID_SIMPLE` on encode |
| false/true/null/undefined | full | dedicated methods |
| f16 decoding via Appendix D tables | full | exact widening |
| f16 encoding only when lossless | full | round-then-verify narrowing |
| NaN → canonical `f9 7e00` under CDE | full | encoder emits canonical quiet NaN half |

### §3.4 Tagging of items

| Requirement | Status | Notes |
|---|---|---|
| Prefix tag numbers, preserve content | full | `tag()` + following item |
| Known tags 0/1/2/3 semantic helpers | out of scope | application layer; trivially composed from primitives |

---

## Section 4 — Serialization Considerations

### §4.1 Preferred serialization

| Rule | Status |
|---|---|
| Shortest integer argument form | full (always emitted) |
| Floats in shortest exactly-preserving width | full (`float` feature) |
| Canonical NaN | full |
| Map key ordering | **full** behind `canonical`: `MapBuilder<N>` records pair spans, sorts by encoded-key bytes (insertion sort, const-generic cap), rebuilds the region through caller scratch — no allocation |
| Map key deduplication | caller's concern (no map storage exists to detect duplicates) |

### §4.2.1 Validity of non-preferred forms on input

Non-shortest integer arguments are **accepted** (RFC makes preferred a
SHOULD). Non-shortest float forms are accepted and widened.

---

### §8 Diagnostic notation

| Requirement | Status | Notes |
|---|---|---|
| Rendering all definite-length items | full (`diagnostic` feature) | `diagnostic::render` into fixed buffer or any `core::fmt::Write` |
| Indefinite markers `_` | full (`indefinite` + `diagnostic`) | `[_ `, `{_ `, `(_ ` |
| Escapes in text strings | full | `\" \\ \n \r \t`, `\u00XX` for control bytes |
| Byte strings as h'hex' | full | lowercase |
| Float text (`NaN`, `Infinity`) | full with `float`; bit-pattern form otherwise | |

## Section 5 — Creating CBOR-Based Protocols

| Concern | Status | Notes |
|---|---|---|
| §5.3 well-formedness errors are structured | full | every failure mode enumerated in `Error` |
| §5.4 extensibility (unknown tags/items) | full | `skip()` consumes any item; `tag()` exposes numbers |
| §5.5 numbers beyond i64/u64 | partial | `u64`/`nint` cover the full CBOR integer space; big-num tags (2/3) are caller-composed bytes |
| §5.6 duplicate map keys | caller's concern | no map representation exists |

---

## Appendix D — Half precision

| Vector class | Status |
|---|---|
| Widening of normals/subnormals/zero/inf/NaN | full (tested against RFC table) |
| Exact-only narrowing incl. subnormal boundary | full (round-then-verify) |
| Ties-to-even rounding | full |

---

## Former Exclusions — Now Included

The original four scope exclusions have all been implemented within the
no-allocation constraint, each feature-gated so default builds stay
minimal:

1. **Indefinite lengths** (§3.2) → `indefinite` feature (see above).
2. **Generic value access** → `raw` feature: `RawValue` is a zero-copy,
   structurally-validated handle to any complete item; it can be
   lazily re-decoded (`RawValue::decoder`) or re-emitted verbatim
   (`Encoder::raw`). This replaces an allocating DOM for the dominant
   use cases (unknown-field pass-through, deferred interpretation).
3. **Streaming decode** → `stream` feature: `StreamDecoder` accepts wire
   fragments in arbitrary chunks over one caller-provided scratch buffer
   and yields complete items as they arrive. Capacity bounds item size by
   construction instead of relying on the heap.
4. **Deterministic maps** → `canonical` feature: `MapBuilder<N>` as
   described under §4.1.

Remaining out of scope (by design, unchanged):

* A generic allocated value tree (contradicts no-alloc).
* Unbounded streaming without caller-supplied buffers (same).
* Big-number tag helpers (2/3): composable from bytes primitives.
