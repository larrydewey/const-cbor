# Optimization

## Optimization Philosophy

The primary objective is compile-time optimization, not runtime dynamism.

Push as much work as possible into the compiler.

The implementation should rely on:

- Rust's type system
- const evaluation
- monomorphization
- LLVM optimization
- static dispatch

The ideal implementation performs the minimum possible runtime work.

Prefer designs where invalid states, unnecessary branches, and redundant calculations are eliminated before execution.

---

# Compile-Time Work

If something can be determined during compilation, it should not be recomputed during encoding or decoding.

Prefer:

- `const fn`
- const evaluation
- const arrays
- compile-time lookup tables
- compile-time validation
- compile-time dispatch
- compile-time size calculations
- branch elimination

Avoid moving decisions into runtime logic when the compiler can determine them.

---

# Zero-Cost Abstractions

Abstractions must compile down to equivalent or better code than handwritten specialized implementations.

Prefer:

- traits with static dispatch
- generics
- const generics
- monomorphized implementations
- immutable data

Avoid:

- runtime reflection
- `dyn Trait`
- `Box<dyn Trait>`
- `Any`
- dynamic dispatch
- runtime type inspection

---

# Specialization Strategy

Prefer specialized implementations over overly generic runtime paths.

Examples:

- `encode_u8`
- `encode_u16`
- `encode_u32`
- `encode_u64`
- `encode_i8`
- `encode_i16`
- `encode_i32`
- `encode_i64`

Do not funnel all operations through one generic runtime implementation if specialization can remove work.

Allow LLVM to optimize through:

- inlining
- constant propagation
- dead-code elimination
- monomorphization

---

# Decoder Optimization

The initial CBOR byte should be treated as a compile-time dispatch opportunity.

Prefer:

- lookup tables
- direct indexing
- exhaustive matching
- precomputed masks
- specialized decoding paths

Avoid:

- repeated masking
- repeated validation work
- nested condition chains
- duplicated runtime parsing decisions

Hot decoding paths should remain simple and predictable.

---

# Encoding Optimization

Prefer:

- specialized encoders
- direct byte construction
- compile-time known representations
- minimal branching

Avoid:

- unnecessary intermediate representations
- temporary allocations
- generic paths that prevent optimization

---

# Branch Prediction

Optimize hot paths for valid CBOR input.

Prefer:

- contiguous hot paths
- predictable branches
- isolated error handling

Mark unlikely paths with:

```rust
#[cold]
```

Keep malformed-input handling separate from normal decoding paths where practical.

---

# Inlining

Inlining should be guided by measurement.

Small functions:

```rust
#[inline(always)]
```

Medium functions:

```rust
#[inline]
```

Only disable inlining when benchmarks demonstrate an improvement.

Do not add inlining annotations blindly.

---

# Const Generics

Use const generics when they eliminate runtime configuration.

Good candidates:

- fixed-size buffers
- parser limits
- maximum nesting depth
- compile-time capacities
- configuration parameters

Avoid const generics that add complexity without measurable benefit.

---

# Performance Review Checklist

Before completing an implementation, ask:

- Can this become `const`?
- Can this computation move to compile time?
- Can runtime branching be reduced?
- Can dynamic dispatch become static dispatch?
- Can allocation be removed?
- Can copying be eliminated?
- Can dispatch become table-driven?
- Can LLVM optimize this more effectively?

Repeat until no obvious improvements remain.

---

# Optimization Validation

Every optimization must be measured.

Before and after comparison should consider:

- runtime performance
- generated code quality
- binary size
- allocations
- maintainability

Do not accept complexity without measurable benefit.
