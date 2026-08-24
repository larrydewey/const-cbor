# Benchmarks

Kept dependency-free on purpose: the crate forbids third-party deps in its
own graph, and `criterion` would pull in hundreds of crates for marginal
value at this size. Use the standard harness instead:

```
RUSTC_BOOTSTRAP=1 cargo bench
```

(`#[bench]` requires nightly; CI tracks release rlib size and example
throughput in `examples/` as regression signals.)

## What to measure when optimizing

- encode/decode throughput of integers, strings, nested arrays
- allocation count (must remain zero — verify with a custom global
  allocator that aborts on alloc)
- release rlib size (CI gate)
