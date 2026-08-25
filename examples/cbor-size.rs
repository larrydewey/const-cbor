//! Binary-size probe: exercises one representative encoding + decoding
//! path per enabled feature tier with black-boxed inputs so nothing is
//! const-folded away, then reports the bytes written.
//!
//! Build & run on the host:
//! ```text
//! cargo run --release --example cbor-size --features <tier>
//! ```
//! Cross-build for Cortex-M3 and inspect linked sections:
//! ```text
//! cargo build --release --target thumbv7m-none-eabi \
//!     --example cbor-size --features <tier>
//! size target/thumbv7m-none-eabi/release/examples/cbor-size
//! ```

#![no_std]
#![cfg_attr(target_os = "none", no_main)]

use const_cbor::{Decoder, Encoder};
use core::hint::black_box;

#[cfg(not(target_os = "none"))]
#[macro_use]
extern crate std;

#[cfg(target_os = "none")]
#[panic_handler]
fn ph(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

/// Fallible calls resolve errors by early exit rather than panicking:
/// the probe must measure the *library*, not the panic machinery.
#[cfg(not(target_os = "none"))]
macro_rules! bail {
    () => {
        return core::hint::black_box(Default::default())
    };
}
#[cfg(target_os = "none")]
macro_rules! bail {
    () => {
        loop {
            core::hint::black_box(());
        }
    };
}
#[cfg(not(target_os = "none"))]
macro_rules! ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => return core::hint::black_box(Default::default()),
        }
    };
}
#[cfg(target_os = "none")]
macro_rules! ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(_) => loop {
                core::hint::black_box(());
            },
        }
    };
}

#[cfg(not(target_os = "none"))]
fn main() {
    let n = run_once();
    println!("{n}");
}

#[cfg(target_os = "none")]
// Bare-metal entry point: no runtime exists, so we provide the symbol the
// linker looks for. `no_mangle` is an FFI attribute, not memory-unsafe code.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    loop {
        black_box(run_once());
    }
}

/// Exercises every enabled feature tier once.
///
/// Fallible calls resolve errors by early return rather than panicking:
/// the probe must measure the *library*, not the panic machinery.
fn run_once() -> usize {
    let mut buf = [0u8; 512];

    // ---- always available: definite-length core --------------------
    let mut e = Encoder::new(&mut buf);
    let input: u64 = black_box(0xa1_2b_3c_4d_5e_6f_70_81);
    let neg: i64 = black_box(-25_000);
    let text: &str = black_box("heartbeat");
    ok!(e.uint(input));
    ok!(e.i64(neg));
    ok!(e.str(text));
    ok!(ok!(e.array_len(2)).null().and_then(|e| e.bool(true)));
    let n = e.written();

    let mut d = Decoder::new(black_box(&buf[..n]));
    let _ = black_box(ok!(d.u64()));
    let _ = black_box(ok!(d.i64()));
    let _ = black_box(ok!(d.str()));
    ok!(d.skip());

    #[cfg(feature = "float")]
    float_tier(&mut buf);

    #[cfg(feature = "indefinite")]
    indefinite_tier(&mut buf);

    #[cfg(feature = "bignum")]
    bignum_tier(&mut buf);

    #[cfg(feature = "diagnostic")]
    diagnostic_tier();

    #[cfg(feature = "canonical")]
    canonical_tier(&mut buf);

    #[cfg(all(feature = "raw", feature = "indefinite"))]
    raw_tier(&mut buf);

    n
}

#[cfg(feature = "float")]
fn float_tier(buf: &mut [u8]) {
    let mut e = Encoder::new(buf);
    ok!(e.f64(black_box(1.0e300)));
    ok!(e.f32(black_box(-273.15)));
    let n = e.written();
    let mut d = Decoder::new(&buf[..n]);
    let _ = black_box(ok!(d.f64()));
    let _ = black_box(ok!(d.f32()));
}

#[cfg(feature = "indefinite")]
fn indefinite_tier(buf: &mut [u8]) {
    let mut e = Encoder::new(buf);
    ok!(e.begin_bytes());
    ok!(e.bytes_chunk(black_box(b"chunked")));
    ok!(e.end_bytes());
    ok!(e.begin_array());
    ok!(e.u8(1));
    ok!(e.end_indef());
    let n = e.written();
    let mut d = Decoder::new(black_box(&buf[..n]));
    if let Ok(mut chunks) = d.bytes_chunks() {
        while chunks.next().is_some() {}
        let _ = d.sync_chunks(&chunks);
    }
    let _ = black_box(d.skip());
    black_box(n);
}

#[cfg(feature = "bignum")]
fn bignum_tier(buf: &mut [u8]) {
    let mut e = Encoder::new(buf);
    let mag: [u8; 16] = black_box([0x01; 16]);
    ok!(e.bignum_uint(&mag));
    let n = e.written();
    let mut d = Decoder::new(&buf[..n]);
    let _ = black_box(d.bignum_i128());
}

#[cfg(feature = "diagnostic")]
fn diagnostic_tier() {
    let item = &[0x82u8, 0x01, 0x02];
    let mut out = [0u8; 128];
    let rendered = const_cbor::diagnostic::render(black_box(item), &mut out).unwrap_or("");
    black_box(rendered.len());
}

#[cfg(feature = "canonical")]
fn canonical_tier(buf: &mut [u8]) {
    use const_cbor::canonical::MapBuilder;
    let mut e = Encoder::new(buf);
    ok!(e.map_len(2));
    let mut m = MapBuilder::<2>::new(&mut e);
    ok!(m.add(|k| k.str("bb"), |v| v.u8(1)));
    ok!(m.add(|k| k.str("aa"), |v| v.u8(2)));
    ok!(m.finish(&mut [0u8; 32]));
    black_box(e.written());
}

#[cfg(feature = "raw")]
fn raw_tier(buf: &mut [u8]) {
    let item = [0x82u8, 0x01, 0x02];
    let mut d = Decoder::new(black_box(&item));
    let Ok(raw) = d.raw_value() else { bail!() };
    let mut e = Encoder::new(buf);
    ok!(e.raw(&raw));
    black_box(e.written());
}
