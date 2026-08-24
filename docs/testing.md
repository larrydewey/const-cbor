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
