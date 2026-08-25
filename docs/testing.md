# Testing

## Testing Philosophy

Tests must establish:

1. RFC 8949 correctness
2. Safety against malformed input
3. Stability of public behavior
4. Confidence for optimization work

Correctness must be proven before optimization.

Every discovered bug must become a regression test.

---

# Required Test Categories

The implementation should include:

## Unit Tests

Test individual components and internal behavior.

Examples:

- byte handling
- integer encoding
- parsing states
- error cases
- utility functions

---

## RFC Example Tests

Implement examples and behaviors derived from RFC 8949.

Tests should reference the relevant RFC requirements where practical.

---

## Roundtrip Tests

Verify:

- encode → decode correctness
- decode → encode behavior where applicable
- preservation of semantic meaning

---

## Malformed Input Tests

Test invalid CBOR inputs, including:

- truncated data
- invalid initial bytes
- invalid lengths
- invalid additional information
- malformed structures
- invalid UTF-8
- invalid numeric encodings

Malformed input must never panic.

---

## Canonical Encoding Tests

Verify:

- canonical ordering
- deterministic output
- canonical numeric representation
- canonical floating-point behavior

---

## Regression Tests

Every bug fix must include a regression test.

Regression tests should prevent the same class of failure from returning.

---

## Property Tests

Use property-based testing where useful.

Examples:

- encoding/decoding invariants
- parser correctness
- boundary conditions
- generated valid values

---

## Fuzz Testing

Fuzzing should target:

- decoders
- parsers
- malformed input handling
- boundary cases
- unsafe code paths

Fuzzing must verify:

- no panics
- no undefined behavior
- deterministic errors
- memory safety

---

# Benchmarks

Performance-sensitive changes require benchmarks.

Benchmark at minimum:

- encoding
- decoding
- streaming decoding
- canonical encoding
- integer encoding
- tag decoding
- map decoding
- allocation count
- binary size
- branch behavior where measurable

---

# Optimization Validation

Every optimization should be measured before and after implementation.

Measurements should consider:

- runtime performance
- allocations
- binary size
- generated code quality
- maintainability

Do not accept complexity without measurable improvement.

---

# Continuous Validation

Tests should run across relevant configurations.

Verify:

- default configuration
- no_std configuration
- alloc-enabled configuration
- optional feature combinations

Every feature must compile and remain independently testable.

---

# Testing SOP (Mandatory)

Every feature — existing or new — must pass through this procedure.
Regressions cannot slip in because a feature is only considered complete
after its tests exist and are green.

## 1. Test before you extend

No new feature work starts until the existing suite is green across the
full feature matrix:

```
cargo test
cargo test --features float
cargo test --features std
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --check
```

## 2. Exhaustive coverage per feature

A feature is "exhaustively tested" when all of the following hold:

1. **Happy path** — every public method works on representative inputs,
   verified byte-for-byte against RFC 8949 expected encodings.
2. **Every error variant** — each `Error` the feature can produce is
   asserted with an exact-match test (`matches!`/`assert_eq`), not just
   `is_err()`.
3. **Boundary values** — at every width boundary (23/24, 255/256,
   u32/u64, i64::MIN, SKIP_LIMIT ± 1, capacity N and N+1).
4. **Malformed input** — truncated prefixes of valid encodings *at every
   byte offset*; reserved fields; wrong-type payloads. Nothing may panic.
5. **Failure atomicity** — failed calls leave encoder cursors and decoder
   positions unchanged where documented.
6. **Roundtrip** — encode → decode → encode is identity for all shapes
   the feature supports, including a deterministic pseudo-random property
   harness (`tests/property_roundtrip.rs`).
7. **Feature interaction** — combinations with `raw`, `stream`,
   `diagnostic`, and float-disabled builds behave as documented.

## 3. Coverage gates

- Tool: `cargo llvm-cov`.
- Gate: **>= 90% line coverage** on `--all-features`, enforced in CI
  (`--fail-under-lines 90`). Target 95–100%.
- Justified exclusions (defensive-only branches unreachable from any
  input) must be called out in review notes, not padded with fake tests.

## 4. New features follow the same SOP

For each new feature:

1. Read the governing RFC sections first; list MUST/MUST NOT/SHOULD.
2. Write conformance vectors from the RFC before implementation.
3. Implement behind its own feature flag, additive to all others.
4. Apply §2 exhaustively, then re-measure coverage (§3).
5. Wire the feature into the CI matrix.

## 5. Fuzzing

`fuzz/fuzz_targets/decode.rs` asserts malformed input never panics.
CI runs a bounded fuzz session (`cargo fuzz run decode -- -runs=100000`);
local development should run longer sessions before releases.
