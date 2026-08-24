# Coding

## General Rust Style

Write code that is:

- explicit
- predictable
- easy to audit
- easy to maintain

Prefer:

- exhaustive match statements
- explicit control flow
- composition over inheritance
- immutable data
- focused functions
- clear ownership boundaries

Avoid:

- unnecessary abstractions
- hidden behavior
- duplicated implementations
- surprising ownership transfers

---

# Abstraction Rules

Traits should describe behavior.

Traits are not an inheritance mechanism.

Prefer:

- small focused traits
- static dispatch
- generic implementations
- compile-time specialization

Avoid:

- trait objects
- dynamic dispatch
- runtime polymorphism

Do not introduce abstraction layers that obscure protocol behavior.

---

# Macros

Avoid macros unless they remove obvious repetition.

Macros must:

- improve maintainability
- preserve readability
- avoid hiding protocol logic

Do not use macros to obscure encoding or decoding behavior.

Protocol rules should remain visible in normal Rust code whenever possible.

---

# Memory Management

Avoid allocation whenever possible.

Prefer borrowed data:

- `&[u8]`
- `&str`

Use owned data only when required.

Use `Cow` only when unavoidable.

Never clone data without a clear reason.

Avoid:

- unnecessary `Vec` creation
- unnecessary copies
- hidden allocations

---

# Lifetimes

Use Rust lifetimes to preserve zero-copy behavior.

Prefer:

- borrowing over ownership
- references over cloned data
- lifetime-driven APIs

Zero-copy decoding should be used whenever correctness permits.

---

# Error Handling

Errors must be:

- compact
- deterministic
- allocation-free
- explicit
- `Copy` whenever practical

Never:

- store `String` inside errors
- allocate error messages
- panic because of malformed CBOR

Malformed input must always produce structured errors.

Errors should communicate what failed without requiring dynamic formatting.

---

# Unsafe Code

Unsafe code is permitted only when all conditions are satisfied.

Requirements:

1. Safe Rust cannot achieve equivalent performance.
2. The performance improvement is measurable.
3. Soundness is formally documented.
4. A safe API boundary is exposed.
5. Every unsafe block includes a `Safety` section.
6. Benchmarks justify the unsafe implementation.

Otherwise, use safe Rust.

Unsafe code must remain:

- isolated
- minimal
- reviewable

---

# Documentation

Every public item must include documentation.

Documentation should explain:

- purpose
- behavior
- ownership rules
- edge cases
- examples where useful

Complex algorithms require implementation notes.

Optimizations require rationale.

Unsafe blocks require safety explanations.

---

# API Design Rules

Public APIs should be:

- predictable
- explicit
- minimal
- ergonomic
- panic-free
- allocation-free where possible

Avoid:

- hidden allocations
- implicit ownership transfers
- surprising behavior

APIs should make performance characteristics visible to users.

---

# Function Design

Keep functions focused.

Prefer:

- small units of logic
- explicit inputs and outputs
- reusable primitives

Avoid:

- functions that mix unrelated responsibilities
- deeply nested logic
- unclear side effects

---

# Naming

Names should describe behavior and intent.

Prefer names that make protocol behavior obvious.

Avoid:

- ambiguous abbreviations
- names that hide allocation behavior
- names that imply stronger guarantees than provided

---

# Code Review Expectations

Before accepting code, verify:

- Is ownership clear?
- Are allocations necessary?
- Are errors explicit?
- Is unsafe justified?
- Is protocol behavior readable?
- Is the implementation maintainable?
