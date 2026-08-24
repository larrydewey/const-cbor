//! IEEE 754 binary16 conversions per RFC 8949 Appendix D.
//!
//! Encoding picks half precision only when the widened result equals the
//! original value (round-then-verify), making every accepted narrowing
//! provably lossless. Decoding widens losslessly. All functions are pure,
//! allocation-free, and panic-free.

/// Converts a 32-bit float to binary16 bits when the conversion is exact.
///
/// Returns `None` whenever the value would lose information: insufficient
/// mantissa precision, subnormal underflow, or overflow to infinity.
#[must_use]
pub fn f16_bits_of(v: f32) -> Option<u16> {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp8 = ((bits >> 23) & 0xff) as i32;

    if v.is_nan() {
        return Some(sign | 0x7e00);
    }
    if v.is_infinite() {
        return Some(sign | 0x7c00);
    }

    let unbiased = exp8 - 127;
    let frac23 = bits & 0x007f_ffff;

    let half = match unbiased {
        // Overflow. Widening infinity can never equal a finite input, so
        // verification below rejects it.
        16.. => sign | 0x7c00,
        // Normal binary16 range: re-bias the exponent, round the mantissa.
        -14..=15 => {
            let m = rne(frac23, 13);
            match m >> 10 {
                // Mantissa carry propagates into the exponent field.
                0 => sign | (((unbiased + 15) as u16) << 10) | m as u16,
                _ => sign | (((unbiased + 16) as u16) << 10),
            }
        }
        // Subnormal binary16 range: scale the full significand down to the
        // subnormal grid, rounding to nearest even.
        _ => {
            let significand = match exp8 {
                // f32 itself is subnormal here; no implicit bit exists.
                0 => frac23,
                _ => frac23 | 0x0080_0000,
            };
            let drop = (13 + (-14 - unbiased)).min(63) as u32;
            match rne(significand, drop) {
                // Rounding carried up into the minimum normal.
                1024 => sign | 0x0400,
                m @ ..=1023 => sign | m as u16,
                _ => return None,
            }
        }
    };

    // Accept only proven-lossless narrowings.
    match f32_of_f16_bits(half) == v {
        true => Some(half),
        false => None,
    }
}

/// Rounds `x` right by `drop` bits using round-to-nearest, ties-to-even.
///
/// Uses 64-bit intermediates because subnormal scaling can demand shifts
/// of up to 63 bits.
const fn rne(x: u32, drop: u32) -> u32 {
    match drop {
        0 => x,
        _ => {
            let x = x as u64;
            let q = (x >> drop) as u32;
            let r = x & ((1u64 << drop) - 1);
            let half = 1u64 << (drop - 1);
            match r > half || (r == half && q & 1 == 1) {
                true => q + 1,
                false => q,
            }
        }
    }
}

/// Converts binary16 bits to a 32-bit float (lossless widening).
#[must_use]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::match_bool)]
pub fn f32_of_f16_bits(h: u16) -> f32 {
    let sign = u32::from(h & 0x8000) << 16;
    let exp5 = u32::from((h >> 10) & 0x1f);
    let frac10 = u32::from(h & 0x03ff);

    let bits = match (exp5, frac10) {
        // Zero.
        (0, 0) => sign,
        // Subnormal h = m * 2^-24. Normalize onto f32's scale: with k the
        // index of m's highest set bit, the value is (m/2^k) * 2^(k-24).
        (0, _) => {
            let k = frac10.ilog2();
            let biased = 103 + k; // (k - 24) + 127
            let mantissa = (frac10 << (23 - k)) & 0x007f_ffff;
            sign | (biased << 23) | mantissa
        }
        // Inf / NaN.
        (31, _) => sign | (0xff << 23) | (frac10 << 13),
        // Normal: re-bias from excess-15 to excess-127.
        (_, _) => sign | ((exp5 + 112) << 23) | (frac10 << 13),
    };
    f32::from_bits(bits)
}

#[cfg(test)]
mod tests {
    use super::{f16_bits_of, f32_of_f16_bits};

    #[test]
    fn rfc_appendix_d_vectors() {
        assert_eq!(f32_of_f16_bits(0x0000), 0.0);
        assert_eq!(f32_of_f16_bits(0x8000), -0.0);
        assert_eq!(f32_of_f16_bits(0x3c00), 1.0);
        assert_eq!(f32_of_f16_bits(0xbc00), -1.0);
        assert_eq!(f32_of_f16_bits(0x3555), 0.25 * (1.0 + 341.0 / 1024.0));
        assert_eq!(f32_of_f16_bits(0x7bff), 65504.0);
        assert_eq!(f32_of_f16_bits(0x0400), (2.0f32).powi(-14));
        assert!(f32_of_f16_bits(0x7c00).is_infinite());
        assert!(f32_of_f16_bits(0xfc00).is_infinite());
        assert!(f32_of_f16_bits(0x7e00).is_nan());

        assert_eq!(f16_bits_of(0.0), Some(0x0000));
        assert_eq!(f16_bits_of(-0.0), Some(0x8000));
        assert_eq!(f16_bits_of(1.0), Some(0x3c00));
        assert_eq!(f16_bits_of(65504.0), Some(0x7bff));
        assert_eq!(f16_bits_of(f32::INFINITY), Some(0x7c00));
        assert_eq!(f16_bits_of(f32::NEG_INFINITY), Some(0xfc00));
        assert_eq!(f16_bits_of(f32::NAN).unwrap() & 0x7e00, 0x7e00);

        // Inexact narrowings must be refused.
        assert_eq!(f16_bits_of(65505.0), None);
        assert_eq!(f16_bits_of(0.1), None);
    }

    #[test]
    fn subnormals_and_boundaries() {
        assert_eq!(f32_of_f16_bits(0x0001), (2.0f32).powi(-24)); // 2^-24
        assert_eq!(f16_bits_of((2.0f32).powi(-24)), Some(0x0001));
        assert_eq!(f16_bits_of((2.0f32).powi(-14)), Some(0x0400)); // 2^-14
        // 2^-25 is exactly halfway between 0 and the smallest subnormal;
        // ties-to-even rounds to zero, which loses the value.
        assert_eq!(f16_bits_of((2.0f32).powi(-25)), None);
        assert_eq!(f16_bits_of(f32_of_f16_bits(0x03ff)), Some(0x03ff));
    }

    #[test]
    fn roundtrip_exact_values() {
        for h in [0x0001_u16, 0x0002, 0x3555, 0x3c00, 0x4000, 0x4248, 0xc000, 0x7bff, 0x0400] {
            let v = f32_of_f16_bits(h);
            assert_eq!(f16_bits_of(v), Some(h));
        }
    }
}
