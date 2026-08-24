# Binary Size

## Goal

Small binary size is a first-class optimization target.

Binary footprint should be considered alongside:

- performance
- allocations
- compile-time specialization
- maintainability

Do not increase binary size without a clear benefit.

---

# Dependency Impact

Dependencies directly affect binary size.

The core library must remain self-contained.

Avoid introducing dependencies because they may increase:

- final binary size
- compile times
- unused code generation
- downstream footprint

Prefer standard library functionality when practical.

---

# Code Size Awareness

Prefer designs that allow the compiler to remove unused functionality.

Consider:

- feature-gated functionality
- dead-code elimination
- minimal public API surface
- focused modules
- small internal representations

Avoid:

- unnecessary generic expansion
- duplicated implementations
- unused abstractions

---

# Feature Flags

Optional functionality must not inflate users who do not enable it.

Features should:

- compile independently
- isolate optional code paths
- avoid unnecessary dependencies

Users enabling only core CBOR functionality should not pay for unrelated features.

---

# Measurement

Binary size changes should be measured.

When evaluating size-sensitive changes, consider:

- release binary size
- feature combinations
- generated code size
- impact on downstream users

A smaller binary is valuable only when it does not compromise correctness, safety, or maintainability.
