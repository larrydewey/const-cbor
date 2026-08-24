//! Size probe for x86_64 (host) and thumbv7m-none-eabi.
//! Inputs are black_boxed so LLVM cannot const-fold the codec away.
#![cfg_attr(not(target_arch = "x86_64"), no_std)]
#![cfg_attr(not(target_arch = "x86_64"), no_main)]
use core::hint::black_box;
use const_cbor::{Decoder, Encoder};

fn body() -> u32 {
    let v: u32 = black_box(1000);
    let s: &str = black_box("hi");
    let b: &[u8] = black_box(&[1u8, 2, 3]);
    let neg: i64 = black_box(-25_000);
    let mut buf = black_box([0u8; 128]);

    let n_written;
    {
        let mut e = Encoder::new(&mut buf);
        e.u32(v).unwrap();
        e.str(s).unwrap();
        e.bytes(b).unwrap();
        e.i64(neg).unwrap();

        #[cfg(feature = "float")]
        {
            let f: f32 = black_box(1.5);
            e.f32(f).unwrap();
        }
        #[cfg(feature = "indefinite")]
        {
            e.str_indef(&["stre", "aming"]).unwrap();
        }
        #[cfg(feature = "canonical")]
        {
            e.map_len(2).unwrap();
            let mut m = const_cbor::canonical::MapBuilder::<2>::new(&mut e);
            m.add(|k| k.str("bb"), |v| v.u8(1)).unwrap();
            m.add(|k| k.str("aa"), |v| v.u8(2)).unwrap();
            m.finish(&mut [0u8; 16]).unwrap();
        }
        n_written = e.written();
    }

    let mut d = Decoder::new(&buf[..n_written]);
    let a = d.u32().unwrap();
    let c = d.str().unwrap();
    let by = d.bytes().unwrap();
    let i = d.i64().unwrap();

    #[cfg(feature = "float")]
    let f = d.f32().unwrap_or_default();
    #[cfg(feature = "indefinite")]
    {
        // Skip the indefinite string wholesale.
        let mut chunks = d.str_chunks().unwrap();
        while let Some(c) = black_box(chunks.next()) {}
        d.sync_chunks(&chunks).unwrap();
    }
    #[cfg(feature = "canonical")]
    {
        let pairs = d.map_len().unwrap();
        for _ in 0..pairs {
            d.str().ok();
            d.u8().ok();
        }
    }

    #[cfg(feature = "diagnostic")]
    {
        let mut text = [0u8; 256];
        if let Ok(pretty) = const_cbor::diagnostic::render(&buf[..n_written], &mut text) {
            black_box(pretty.len());
        }
    }

    black_box((a, c, by.len(), i)).0 as u32
        + u32::from(cfg!(feature = "float"))
}

#[cfg(target_arch = "x86_64")]
fn main() {
    println!("result: {}", body());
}

#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    body();
    loop {}
}

#[cfg(not(target_arch = "x86_64"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
