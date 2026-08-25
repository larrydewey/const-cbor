# Benchmarks

Dependency-free on purpose: the crate keeps third-party crates out of its
own graph, so the bench harness is a plain `harness = false` binary
(`benches/core_throughput.rs`) reporting median nanoseconds per operation
over black-boxed inputs.

```
cargo bench
```

Current baseline (x86_64 host, release):

| bench | median ns/op |
|---|---|
| encode/u64-max | 1.0 |
| decode/u64-max | 2.0 |
| encode/str-44B | 1.0 |
| decode/str-44B | 10.0 |
| skip/nested-8x[int,str] | 44.0 |
| roundtrip/nested-8x[int,str] | 63.0 |

## What to measure when optimizing

- encode/decode throughput of integers, strings, nested arrays
- allocation count (must remain zero — verify with a custom global
  allocator that aborts on alloc)
- linked size via the `cbor-size` probe (CI gate, <26 KB all-features)

Size regression tracking lives in CI (`size` job); throughput regressions
are easiest to spot by diffing `cargo bench` output before/after a change.
