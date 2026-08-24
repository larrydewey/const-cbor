# RFC Compliance

## Normative Source

RFC 8949 is the sole normative source for CBOR protocol behavior.

All protocol decisions must be traceable to RFC 8949.

This repository maintains a local specification references:

```
specs/
```

The local specification file should be reviewed before implementing protocol
behavior.

`specs/RFC8949.md` exists to:

- summarize relevant RFC requirements
- organize protocol details for implementation
- provide section references
- capture implementation notes
- connect specification requirements to tests

The local specification file does not override RFC 8949.

If any discrepancy exists between:

- `specs/RFC8949.md`
- RFC 8949

RFC 8949 takes precedence.

Do not derive behavior from:

- existing CBOR libraries
- existing language implementations
- ecosystem conventions
- undocumented compatibility assumptions

Existing implementations may be used for interoperability testing, but they
are not authoritative.

When implementation convenience conflicts with RFC 8949:

RFC 8949 wins.

---

## Specification Workflow

Before implementing any protocol behavior:

1. Read the relevant section in:

       specs/RFC8949.md

1. Verify the referenced RFC 8949 sections.
1. Extract all normative requirements.
1. Classify requirements as:

   - MUST
   - MUST NOT
   - SHOULD
   - SHOULD NOT
   - MAY

1. Identify edge cases and invalid behavior.
1. Design the implementation around those requirements.
1. Add tests covering the behavior.
1. Optimize only after correctness is established.

Never implement protocol behavior based only on existing implementations.

Never assume unspecified behavior.

Never skip RFC analysis.

---

## Requirement Handling

### MUST

Every MUST requirement must be implemented.

Missing a MUST requirement is an RFC compliance failure.

---

### MUST NOT

Every MUST NOT requirement must be enforced.

The implementation must prevent prohibited behavior rather than
relying on callers to avoid it.

---

### SHOULD

SHOULD requirements should be implemented unless there is a documented
reason not to.

When intentionally not implementing a SHOULD requirement, document:

- the reason
- the compatibility impact
- the alternative behavior

---

### SHOULD NOT

Avoid behavior discouraged by RFC 8949 unless there is a strong documented
reason.

---

### MAY

MAY behavior is optional.

When unsupported:

- document the decision
- ensure interoperability is preserved
- avoid surprising behavior

---

## Encoding Compliance

Encoders must follow RFC 8949 encoding rules.

Verify:

- major type encoding
- additional information encoding
- integer representation
- length encoding
- tag representation
- floating-point representation

Do not invent alternate encodings.

---

## Decoding Compliance

Decoders must correctly process valid CBOR and reject invalid CBOR.

Verify:

- valid major types
- valid additional information
- valid lengths
- integer boundaries
- UTF-8 validity
- nesting behavior
- tag handling
- floating-point values

Malformed input must produce structured errors.

Never silently recover from invalid encodings.

---

## Canonical Encoding Compliance

Canonical encoding is a protocol behavior, not a cosmetic transformation.

Canonical implementations must follow RFC-defined requirements for:

- deterministic output
- map ordering
- numeric representation
- floating-point representation

Do not implement canonical encoding as an afterthought.

---

## Floating-Point Compliance

Floating-point handling must preserve RFC-defined behavior.

Support:

- half precision
- single precision
- double precision
- NaN
- Infinity
- negative zero
- subnormal values

Do not normalize values unless required by RFC-defined canonical behavior.

---

## Tag Compliance

Tags are generic CBOR mechanisms.

The core library must:

- preserve tag numbers
- support arbitrary tags
- avoid application-specific interpretation

Application semantics must remain outside the core library.

---

## Specification Traceability

Protocol-sensitive code should document RFC references.

Use references in:

- implementation comments
- public documentation
- test names

Complex protocol decisions should be explainable by pointing to the relevant
RFC section.

Example:

```
// RFC 8949 Section X.Y:
// Explanation of the required behavior.
```

---

## Specification Maintenance

When implementing a protocol feature:

- update `specs/RFC8949.md` if the relevant behavior is missing
- keep section references accurate
- add links between requirements and tests where useful

The specification file should evolve alongside the implementation.

Changes to `specs/RFC8949.md` must preserve alignment with RFC 8949.

---

## RFC Testing

Tests should map directly to protocol requirements.

Required coverage includes:

- RFC examples
- valid encodings
- invalid encodings
- canonical behavior
- deterministic behavior
- edge cases
- interoperability cases

Every RFC compliance issue discovered must become a regression test.

---

## Conflict Resolution

When requirements conflict, prioritize:

1. RFC 8949 correctness
1. Security and memory safety
1. API stability
1. Performance
1. Binary size

Never trade protocol correctness for optimization..
