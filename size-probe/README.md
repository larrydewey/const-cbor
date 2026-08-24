# Size Probe

Measures real linked flash footprint on `thumbv7m-none-eabi` and runs
natively on x86_64 for symbol inspection.

All inputs are `black_box`ed so LLVM cannot const-fold the codec away,
and every enabled feature's code path is actually exercised.

## Usage

```sh
# host (x86_64): runs and prints a result; inspect symbols:
cargo run --release --features all
nm target/release/cbor-size | grep const_cbor

# embedded: measure linked sections
cargo build --release --target thumbv7m-none-eabi --features all
llvm-size -A target/thumbv7m-none-eabi/release/cbor-size
```

Compare `.text + .rodata + .ARM.exidx` totals between feature sets.
