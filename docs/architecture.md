# Architecture

## Design Philosophy

The implementation should resemble a compiler more than an interpreter.

The architecture should push decisions into:

- Rust's type system
- compile-time evaluation
- static dispatch
- monomorphization

Runtime behavior should be the minimal work required to process input or produce output.

Prefer explicit representations and predictable control flow over highly dynamic abstractions.

---

# Module Organization

Separate responsibilities into focused modules.

Suggested layout:

    src/

        bytes/

        canonical/

        consts/

        decode/

        encode/

        error/

        float/

        integer/

        map/

        parser/

        stream/

        tag/

        text/

        util/

        value/

Responsibilities should remain cohesive.

Avoid:

- monolithic files
- modules containing unrelated functionality
- duplicated protocol logic across modules

---

# Encoding Architecture

Encoding should provide specialized paths for different CBOR representations.

Prefer:

- explicit encoding implementations
- specialized integer encoders
- dedicated canonical encoding paths
- reusable low-level byte writing primitives

Avoid:

- one generic runtime encoder handling every case
- hidden allocations
- runtime type inspection

---

# Decoder Architecture

The initial CBOR byte is the primary dispatch mechanism.

Treat decoding as a structured dispatch problem.

Prefer:

- direct dispatch
- lookup tables
- exhaustive matching
- deterministic parsing paths

Avoid:

- deeply nested conditional chains
- duplicated parsing logic
- runtime reflection

The decoder should validate input while keeping the parsing model simple and auditable.

---

# Parser Architecture

Parsing should use deterministic state machines where practical.

Prefer:

- explicit parser states
- iterative parsing
- bounded processing
- predictable transitions

Avoid recursion where an iterative implementation provides equivalent behavior.

Streaming decoding should not require complete input buffering.

---

# API Architecture

Provide two levels of API.

## High-Level API

A convenient owned representation:

- `Value` enum
- ergonomic construction and inspection

## Low-Level API

A performance-oriented decoding interface:

- borrowed data
- zero-copy operation
- minimal allocation

The low-level decoder must not require construction of `Value`.

Users should be able to choose between convenience and maximum efficiency.

---

# Trait Design

Traits should describe behavior, not inheritance.

Preferred traits:

- `Encode`
- `Decode`
- `BorrowDecode`
- `CanonicalEncode`
- `StreamDecode`

Prefer:

- static dispatch
- generic implementations
- compile-time specialization

Avoid:

- trait objects
- dynamic dispatch
- runtime polymorphism

---

# Memory Model

The architecture should favor borrowing.

Preferred types:

- `&[u8]`
- `&str`

Zero-copy decoding is preferred whenever correctness permits.

Ownership should only be introduced when required by API guarantees or feature behavior.

Avoid:

- unnecessary cloning
- unnecessary allocation
- hidden ownership transfers

---

# Tag Architecture

Tags are generic protocol mechanisms.

The core library should:

- preserve tag numbers
- support arbitrary tags
- keep tag handling extensible

Do not embed application-specific semantics into the core implementation.

Application-specific tag meanings belong outside the core library.

---

# Canonical Encoding Architecture

Canonical encoding is a first-class architectural concern.

Do not implement canonical encoding as a final transformation step.

Canonical behavior should have dedicated implementation paths for:

- map ordering
- deterministic output
- canonical numeric representation
- canonical floating-point representation

Generic encoding and canonical encoding should remain separate concepts.

---

# Const Generic Usage

Use const generics when they improve API design or eliminate runtime configuration.

Appropriate uses include:

- fixed-size buffers
- parser limits
- nesting depth
- compile-time capacities
- configuration parameters

Avoid adding generic complexity without measurable benefit.
