# Security

## Untrusted Input

CBOR input must be treated as untrusted data.

The implementation must safely handle:

- malformed encodings
- truncated input
- invalid values
- adversarial input
- deeply nested structures

Never assume input is valid.

---

# Decoder Safety

Malformed CBOR must:

- return structured errors
- never panic
- never cause undefined behavior
- never silently recover

The decoder must validate:

- input boundaries
- lengths
- integer conversions
- UTF-8 text strings
- nesting limits
- tag representations
- floating-point encodings

---

# Error Handling

Errors must be:

- deterministic
- allocation-free
- explicit
- compact

Never:

- allocate while reporting malformed input
- store dynamic error messages
- hide validation failures

Error behavior should be predictable for both callers and fuzzing systems.

---

# Unsafe Code Policy

Unsafe code is permitted only when all requirements are satisfied:

1. Safe Rust cannot achieve equivalent performance.
2. The performance improvement is measurable.
3. Soundness is formally documented.
4. A safe API is exposed.
5. Every unsafe block includes a `Safety` section.
6. Benchmarks justify its existence.

Otherwise, use safe Rust.

---

# Unsafe Code Requirements

Unsafe code must be:

- isolated
- minimal
- easy to review
- surrounded by safety documentation

Every unsafe operation must explain:

- why unsafe is required
- what invariants must hold
- why the operation is sound

---

# Memory Safety

The implementation must preserve Rust safety guarantees.

Avoid:

- unnecessary unsafe code
- unchecked assumptions
- unchecked indexing without proof
- lifetime violations
- hidden aliasing requirements

Prefer safe abstractions unless unsafe provides a measurable and justified improvement.

---

# Robustness

The implementation should prioritize:

1. RFC correctness
2. memory safety
3. predictable failure behavior
4. performance

Never trade correctness or safety for speed.
