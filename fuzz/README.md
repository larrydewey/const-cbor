# Fuzzing

```
cargo install cargo-fuzz
cargo fuzz run decode
```

`libfuzzer-sys` is a dev-only toolchain and never enters the library's
dependency graph.
