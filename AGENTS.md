# AGENTS.md

## Project

This repository implements a production-quality CBOR library in Rust.

The implementation must fully conform to RFC 8949.

RFC 8949 is the sole normative source for protocol behavior.

Do not derive protocol behavior from existing CBOR libraries or implementations.

The primary optimization objective is compile-time specialization, not runtime
dynamism.

The design should leverage Rust's:

- type system
- const evaluation
- monomorphization
- optimizer

to eliminate unnecessary runtime work.

---

## Priority Order

In descending priority:

1. RFC 8949 correctness
1. Compile-time optimization
1. Zero-cost abstractions
1. no_std compatibility
1. Runtime performance
1. Small binary size
1. Clean API design
1. Documentation quality

Never sacrifice RFC compliance for performance.

Never sacrifice correctness for convenience.

---

## Rust Requirements

Target the newest stable Rust release.

Requirements:

- stable Rust only
- no nightly features
- no unstable compiler behavior

The library must support:

- `no_std` by default
- `alloc` through an optional feature
- `std` through an optional feature

Core functionality must never require `std`.

---

## Dependencies

The core implementation must be completely self-contained.

Allowed libraries:

- `core`
- `alloc`
- `std` (optional)

Third-party crates are prohibited in the core library.

Do not introduce dependencies for functionality that can reasonably
be implemented using Rust's standard libraries.

Development-only dependencies are allowed only for:

- testing
- fuzzing
- benchmarking
- CI tooling

Development dependencies:

- must never appear in public APIs
- must never be required by downstream users

---

## External Code Policy

Do not:

- copy existing CBOR implementations
- translate another CBOR library
- import external algorithms
- reproduce GitHub implementations
- use generated code from CBOR projects

Implement functionality directly from RFC 8949.

Assume no external crates are available.

---

## Feature Flags

Optional functionality must be feature gated.

Suggested features:

- alloc
- std
- serde
- derive
- canonical
- deterministic
- diagnostic
- stream
- tags
- float
- bignum

Requirements:

- every feature must compile independently
- features must not introduce hidden dependencies
- core functionality must remain minimal

---

## Agent Workflow

Before implementing any feature:

1. Read the relevant RFC 8949 sections.
1. Identify all:
   - MUST requirements
   - MUST NOT requirements
   - SHOULD requirements
   - MAY requirements
1. Implement correctness first.
1. Add tests.
1. Optimize only after correctness is established.

Never assume unspecified behavior.

Never skip RFC analysis.

---

## Detailed Documentation

Before modifying code, read the relevant guidance:

### Architecture

- `docs/architecture.md`

### Coding

- `docs/coding.md`

### Optimization

- `docs/optimization.md`

### Binary size

- `docs/binary-size.md`

### Security

- `docs/security.md`

### Testing

- `docs/testing.md`

### RFC process

- `docs/rfc.md`

### Protocol specification

- `specs/SPECIFICATIONS.md`
