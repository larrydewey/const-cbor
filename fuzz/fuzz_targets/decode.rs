#![no_main]
// cargo-fuzz target: decoding must never panic on arbitrary input.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut dec = const_cbor::Decoder::new(data);
    // Full-item skip exercises every structural path.
    let _ = dec.skip();
});
