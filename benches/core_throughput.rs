//! Throughput benchmarks for the core codec.
//!
//! Deliberately dependency-free (`harness = false`): the crate keeps
//! third-party crates out of its graph, and `criterion` would pull
//! hundreds of them for marginal value at this size. Median wall-clock
//! over black-boxed inputs is a solid CI-friendly regression signal.
//!
//! Run with:
//! ```text
//! cargo bench
//! ```

use const_cbor::{Decoder, Encoder};
use core::hint::black_box;
use std::time::Instant;

/// Median per-call nanoseconds of `f` over `iters` repetitions.
fn median_ns(iters: u64, f: &mut dyn FnMut()) -> u64 {
    let warmup = (iters / 10).max(1);
    for _ in 0..warmup {
        f();
    }
    let mut samples = [0u64; 31];
    for s in &mut samples {
        let start = Instant::now();
        for _ in 0..iters {
            f();
        }
        *s = start.elapsed().as_nanos() as u64 / iters;
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

macro_rules! ok {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            // Bench inputs are always valid; a failure means the bench is
            // broken, so stop measuring rather than report garbage.
            Err(e) => {
                eprintln!("bench input rejected: {e:?}");
                std::process::exit(1);
            }
        }
    };
}

struct Case<'a> {
    name: &'static str,
    iters: u64,
    f: Box<dyn FnMut() + 'a>,
}

fn main() {
    let payload = "the quick brown fox jumps over the lazy dog";

    // A representative nested structure: array(8) of [int, text(44)].
    let mut struct_item = Vec::with_capacity(512);
    // Text item built with the encoder so its length prefix is well-formed.
    let mut str_scratch = [0u8; 64];
    let str_item: Vec<u8> = {
        let mut e = Encoder::new(&mut str_scratch);
        ok!(e.str(payload));
        let (buf, n) = e.finish();
        buf[..n].to_vec()
    };
    let mut scratch = [0u8; 1024];
    {
        let mut e = Encoder::new(&mut scratch);
        ok!(e.array_len(8));
        for i in 0..8u64 {
            ok!(e.array_len(2));
            ok!(e.uint(i));
            ok!(e.str(payload));
        }
        let (buf, n) = e.finish();
        struct_item.extend_from_slice(&buf[..n]);
    }

    let mut cases: Vec<Case> = vec![
        Case {
            name: "encode/u64-max",
            iters: 200_000,
            f: Box::new(|| {
                let mut buf = [0u8; 16];
                let mut e = Encoder::new(&mut buf);
                ok!(e.uint(black_box(u64::MAX)));
                black_box(e.written());
            }),
        },
        Case {
            name: "decode/u64-max",
            iters: 200_000,
            f: Box::new(|| {
                let input = [0x1b_u8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
                let mut d = Decoder::new(black_box(&input));
                black_box(ok!(d.u64()));
            }),
        },
        Case {
            name: "encode/str-44B",
            iters: 200_000,
            f: Box::new(|| {
                let mut buf = [0u8; 64];
                let mut e = Encoder::new(&mut buf);
                ok!(e.str(black_box(payload)));
                black_box(e.written());
            }),
        },
        Case {
            name: "decode/str-44B",
            iters: 200_000,
            f: Box::new(|| {
                let mut d = Decoder::new(black_box(str_item.as_slice()));
                black_box(ok!(d.str()));
            }),
        },
        Case {
            name: "skip/nested-8x[int,str]",
            iters: 50_000,
            f: Box::new(|| {
                let mut d = Decoder::new(black_box(&struct_item));
                ok!(d.skip());
                black_box(d.position());
            }),
        },
        Case {
            name: "roundtrip/nested-8x[int,str]",
            iters: 50_000,
            f: Box::new(|| {
                let mut d = Decoder::new(black_box(&struct_item));
                let n = ok!(d.array_len());
                for _ in 0..n {
                    ok!(d.array_len());
                    black_box(ok!(d.u64()));
                    black_box(ok!(d.str()));
                }
                black_box(d.position());
            }),
        },
    ];

    println!("{:<28} {:>12}", "bench", "median ns/op");
    println!("{:-<42}", "");
    for c in &mut cases {
        let ns = median_ns(c.iters, &mut *c.f);
        println!("{:<28} {:>12.1}", c.name, ns as f64);
    }
}
