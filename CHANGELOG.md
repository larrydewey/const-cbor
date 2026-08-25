# Changelog

## 0.3.0 — 2026-08-25

### Added
- `bignum` feature: RFC 8949 §3.4.3 tags 2/3 with zero-copy magnitudes,
  preferred-serialization normalization, `u64`/`i128` typed getters.
- `deterministic` feature (implies `indefinite`): strict RFC §4.2
  validation — minimal argument widths, shortest floats, strictly sorted
  map keys, canonical bignums; new `Error::NOT_CANONICAL` code (11).
- `Decoder::peek_break`, `peek_head_byte`, `take_break`, `expect_indef`
  low-level inspection/consumption helpers.
- `const-cbor-serde` companion crate (workspace member): serde
  Serializer/Deserializer adapters, zero-copy borrows, ecosystem-
  compatible enum representation; optional `bignum` passthrough.
- Size probe example (`examples/cbor-size.rs`) with reproducible
  `[profile.size]`; recalibrated size matrix (core 3,148 B, all-features
  24,102 B on Cortex-M3) and CI gate (<26 KB linked).
- Testing SOP (docs/testing.md); coverage + fuzz gates in CI.

### Fixed
- `Decoder::skip`/`raw_value` truncated tagged items: a tag decremented
  the depth counter before its payload was consumed, so `skip` returned
  mid-item (e.g. `C1` instead of `C1 02`).
- `Decoder::simple`/`bool` misread float items whose IEEE 754 payload
  bits resembled small simple values (`f9 0014` decoded as `true`);
  float widths now always yield `FLOAT_UNSUPPORTED`.
- Diagnostic rendering of one-byte unknown simples (`f0`) errored with
  `TYPE_MISMATCH` instead of rendering `simple(16)`.
- `Encoder::f64(NAN)` emitted a full double bit pattern instead of the
  canonical half `0x7e00` required by §4.2.2.

### Changed
- `Decoder::skip` depth counters narrowed to u32 (outcome-preserving;
  halves stack zeroing). One pre-existing test encoded a malformed map
  that only passed due to the tag bug above; corrected to valid input.
