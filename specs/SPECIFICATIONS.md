# RFC Compliance

## Normative Specifications

This repository implements multiple IETF standards.

Each feature must be implemented according to the RFC that defines it.

The local specifications are maintained under:

```text
specs/
```

Current specifications include:

| RFC      | Subject                                               |
| -------- | ----------------------------------------------------- |
| RFC 8949 | Concise Binary Object Representation (CBOR)           |
| RFC 8943 | CBOR Tags for Object Identifiers                      |
| RFC 8746 | CBOR Tags for Typed Arrays                            |
| RFC 8742 | Concise Binary Object Representation (CBOR) Sequences |
| RFC 8610 | Concise Data Definition Language (CDDL)               |
| RFC 8392 | CBOR Web Token (CWT)                                  |
| RFC 8152 | CBOR Object Signing and Encryption (COSE)             |

Each local specification summarizes the RFC, records implementation notes,
provides section references, and links requirements to tests.

The local specification files never override the RFCs themselves.

If any discrepancy exists between a local specification and its RFC, the RFC
takes precedence.

---

## Normative Authority

Normative behavior must always be derived from the RFC defining that feature.

Examples:

* Core CBOR encoding and decoding → RFC 8949
* Object Identifier tags → RFC 8943
* Typed array tags → RFC 8746
* CBOR Sequences → RFC 8742
* CDDL syntax and validation → RFC 8610
* CBOR Web Tokens → RFC 8392
* COSE structures and cryptographic processing → RFC 8152

Never derive protocol behavior from:

* existing libraries
* existing implementations
* ecosystem conventions
* undocumented compatibility assumptions

Existing implementations may be used for interoperability testing but are
never authoritative.

When implementation convenience conflicts with an RFC:

The RFC wins.

---

## Specification Workflow

Before implementing any protocol feature:

1. Identify the RFC responsible for that feature.

1. Read the corresponding document in:

   ```text
   specs/
   ```

1. Verify the referenced RFC sections.

1. Extract every normative requirement.

1. Classify requirements as:

   * MUST
   * MUST NOT
   * SHOULD
   * SHOULD NOT
   * MAY

1. Identify invalid behavior and edge cases.

1. Design the implementation around those requirements.

1. Add tests covering the behavior.

1. Optimize only after correctness has been established.

Never implement protocol behavior solely from existing implementations.

Never assume unspecified behavior.

Never skip RFC analysis.

---

## Requirement Handling

### MUST

Every MUST requirement shall be implemented.

Failure to implement a MUST requirement constitutes an RFC compliance failure.

---

### MUST NOT

Every MUST NOT requirement shall be enforced.

The implementation must reject prohibited behavior rather than relying on
callers to avoid it.

---

### SHOULD

SHOULD requirements should be implemented unless there is a documented
technical reason not to.

When intentionally deviating from a SHOULD requirement, document:

* the reason
* compatibility impact
* alternative behavior

---

### SHOULD NOT

Avoid SHOULD NOT behavior unless there is a documented justification.

---

### MAY

MAY behavior is optional.

When unsupported:

* document the decision
* preserve interoperability
* avoid surprising behavior

---

# RFC 8949 — Core CBOR Compliance

## Encoding Compliance

Encoders shall follow RFC 8949 encoding rules.

Verify:

* major type encoding
* additional information encoding
* integer representation
* length encoding
* tag representation
* floating-point representation

Do not invent alternate encodings.

---

## Decoding Compliance

Decoders shall correctly process valid CBOR and reject malformed CBOR.

Verify:

* major types
* additional information
* lengths
* integer boundaries
* UTF-8 validity
* nesting
* tags
* floating-point values

Malformed input must produce structured errors.

Never silently recover from invalid encodings.

---

## Deterministic Encoding

Where deterministic or canonical encoding is implemented, follow RFC 8949
requirements exactly.

Verify:

* deterministic map ordering
* shortest numeric encodings
* shortest length encodings
* floating-point encoding rules

---

## Floating-Point Compliance

Support:

* half precision
* single precision
* double precision
* NaN
* Infinity
* negative zero
* subnormal values

Do not normalize values except where RFC 8949 requires deterministic
encoding.

---

## Generic Tag Support

The core CBOR implementation shall:

* preserve arbitrary tag numbers
* encode arbitrary tags
* decode arbitrary tags
* avoid application-specific interpretation

Application semantics belong outside the core library.

---

# RFC 8943 — Object Identifier Tags

Support for Object Identifier tags shall conform to RFC 8943.

Verify:

* correct tag numbers
* correct OID encoding
* correct relative OID encoding
* validation of malformed encodings

Do not reinterpret OID semantics within the core CBOR implementation.

---

# RFC 8746 — Typed Arrays

Typed array tags shall conform to RFC 8746.

Verify:

* element widths
* endianness
* tag handling
* array length validation
* deterministic encoding where applicable

Reject malformed typed-array encodings.

---

# RFC 8742 — CBOR Sequences

CBOR Sequence handling shall conform to RFC 8742.

Verify:

* concatenated CBOR items
* incremental decoding
* end-of-stream behavior
* malformed sequence detection

Do not confuse CBOR Sequences with arrays.

---

# RFC 8610 — CDDL

Schema handling shall conform to RFC 8610.

Verify:

* grammar parsing
* type rules
* occurrence operators
* control operators
* generic parameters
* group handling

Schema validation behavior shall match RFC-defined semantics.

---

# RFC 8392 — CBOR Web Tokens

CWT support shall conform to RFC 8392.

Verify:

* claim encoding
* registered claim keys
* token validation
* time-based claims
* interoperability with COSE

Application policy remains outside the core implementation.

---

# RFC 8152 — COSE

COSE support shall conform to RFC 8152.

Verify:

* protected headers
* unprotected headers
* signature structures
* MAC structures
* encryption structures
* recipient structures
* key handling
* algorithm identifiers

Cryptographic behavior shall follow the RFC exactly.

Do not substitute non-standard encodings.

---

# Specification Traceability

Protocol-sensitive code should document RFC references.

Use references in:

* implementation comments
* public documentation
* test names

Example:

```rust
// RFC 8949 Section 3.1:
// Unsigned integers use major type 0.
```

Each protocol decision should be explainable by pointing directly to the
relevant RFC section.

---

# Specification Maintenance

When implementing a protocol feature:

* update the corresponding document under `specs/`
* keep section references accurate
* maintain links between requirements and tests

Local specification files should evolve alongside the implementation while
remaining aligned with their corresponding RFCs.

---

# RFC Testing

Tests should map directly to RFC requirements.

Coverage should include:

* RFC examples
* valid encodings
* invalid encodings
* deterministic behavior
* edge cases
* interoperability
* regression tests

Every discovered RFC compliance issue must become a regression test.

---

# Conflict Resolution

When requirements conflict, prioritize:

1. RFC correctness
2. Security and memory safety
3. API stability
4. Performance
5. Binary size

Never trade protocol correctness for optimization.
