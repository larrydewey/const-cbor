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

---

# Current Measurements (recalibrated 2026-08)

Measured with the checked-in probe `examples/cbor-size.rs`:

- target: `thumbv7m-none-eabi`
- profile: `[profile.size]` (opt-level = "z", fat LTO, panic = abort,
  codegen-units = 1, symbols stripped)
- metric: linked `.text` + `.rodata`
- all inputs `black_box`ed so no tier is const-folded away
- every enabled feature's code path is actually executed

| Tier | Bytes (.text + .rodata) |
|---|---|
| core only | 3,148 |
| + float | 4,964 |
| + indefinite | 4,496 |
| + bignum | 3,994 |
| + canonical | 4,000 |
| + deterministic (implies indefinite) | 4,496 |
| + diagnostic | 7,462 |
| diagnostic + float (decimal text) | 21,578 |
| everything | 24,102 |

Probe hygiene requirements (these keep the numbers honest):

- the probe resolves errors by early return, never `unwrap()`/panics —
  panic formatting machinery is not the library's cost,
- every enabled feature's encode *and* decode path executes with
  black-boxed inputs so LLVM cannot const-fold a tier away,
- rebuild from scratch when switching feature sets and verify all tier
  binaries differ (`touch examples/cbor-size.rs` before each build).

## Core-tier floor analysis

The 3,148-byte core build decomposes as roughly:

| Component | Bytes |
|---|---|
| compiler builtins (memcpy/memclr/shift helpers) | ~900 |
| codec code (encode, decode, skip, head I/O) | ~1,700 |
| alignment, veneers, unwinding index | ~550 |

Builtins are fixed toolchain cost paid by any Rust binary on this target;
the codec logic itself is well under 2 KB.

Reproduce:

```
cargo build --profile size --target thumbv7m-none-eabi --example cbor-size --features <tier>
size target/thumbv7m-none-eabi/size/examples/cbor-size
```

CI gates the "everything" build at < 26 KB.

## Known Size Levers (spec-compliant only)

Levers considered, with their compliance trade-offs:

1. **Diagnostic float formatting (~14 KB)** — the dominant cost is Rust's
   shortest-roundtrip float Display. A custom Grisu/Ryu-style fixed-buffer
   formatter could cut this substantially; RFC §4.2.2 and §8 conformance
   are preserved either way since output text is unaffected.
2. **`#[cold]`/`#[inline(never)]` on error paths** — already applied to
   the skip stack push; further annotation showed no measurable win.
3. **Panic machinery** — `panic = "abort"` in `profile.size`; downstream
   firmware typically does the same.
4. **Rejected**: removing non-minimal-encoding *acceptance* from decoders
   would shrink nothing measurable and break MUST-level interoperation.
