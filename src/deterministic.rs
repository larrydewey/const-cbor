//! Deterministic-encoding validation (RFC 8949 §4.2), feature
//! `deterministic`.
//!
//! Ordinary decoding accepts every well-formed item, including
//! non-preferred ones (§4.2.1 makes preferred serialization a SHOULD).
//! This module is the strict counterpart: [`check`] walks one item and
//! rejects anything a deterministic encoder would not have produced:
//!
//! * indefinite lengths (§4.2.1: definite lengths only),
//! * non-minimal argument widths for major types 0–6,
//! * two-byte simple encodings below 32,
//! * floats wider than the value requires (§4.2.2; `float` feature),
//! * map keys not strictly increasing bytewise — which also rules out
//!   duplicate keys,
//! * bignums (tags 2/3) with empty or leading-zero magnitudes
//!   (§3.4.3 + §4.2.1: representable integers use basic encoding).
//!
//! The walk is iterative over a fixed [`SKIP_LIMIT`]-sized stack; deeper
//! nesting yields [`Error::DEPTH_EXCEEDED`] like [`Decoder::skip`].

use crate::decode::{Decoder, SKIP_LIMIT};
use crate::error::Error as E;
use crate::head::{self, Major};

/// One open container on the validation stack.
#[derive(Clone, Copy)]
struct Frame {
    /// Items (or pair halves) still expected.
    left: usize,
    /// Map scope: even slots are keys and participate in order checking.
    map: bool,
    /// Span of the previous key in this map, if any.
    prev_key: Option<(usize, usize)>,
}

/// Validates that `input` contains exactly one complete item encoded per
/// RFC 8949 §4.2 deterministic rules.
///
/// # Errors
///
/// [`E::NOT_CANONICAL`] for any preferred-serialization violation;
/// structural errors ([`E::TRUNCATED`], ...) for malformed input;
/// [`E::DEPTH_EXCEEDED`] beyond [`SKIP_LIMIT`] nesting levels.
pub fn check(input: &[u8]) -> Result<(), E> {
    let mut d = Decoder::new(input);
    walk_item(&mut d)?;
    match d.is_empty() {
        true => Ok(()),
        false => Err(E::TRUNCATED),
    }
}

/// Validates every complete item in `input`, requiring the input to be
/// fully consumed by well-formed canonical items.
///
/// # Errors
///
/// As [`check`].
pub fn check_all(input: &[u8]) -> Result<(), E> {
    let mut d = Decoder::new(input);
    while !d.is_empty() {
        walk_item(&mut d)?;
    }
    Ok(())
}

/// Walks exactly one item starting at the decoder's cursor.
fn walk_item(d: &mut Decoder<'_>) -> Result<(), E> {
    let mut stack = [Frame { left: 0, map: false, prev_key: None }; SKIP_LIMIT];
    // Virtual root frame expecting one item.
    stack[0] = Frame { left: 1, map: false, prev_key: None };
    let mut sp = 1usize;
    // Set while a bignum tag (2/3) awaits its byte-string payload.
    let mut bignum_tag: Option<u64> = None;

    loop {
        let start = d.position();
        let Some(&initial) = d.input.get(d.pos) else {
            return Err(E::TRUNCATED);
        };
        if head::is_break(initial) {
            return Err(E::UNEXPECTED_BREAK);
        }
        let ai = initial & 0x1f;
        let (major, arg, indef) = d.next_head_ext()?;
        let was_bignum = bignum_tag.take();

        // Determinism rule 1: definite lengths only (§4.2.1).
        if indef {
            return Err(E::NOT_CANONICAL);
        }
        // Determinism rule 2: minimal argument width. Major 7 is exempt:
        // its low bits select a float width or encode the simple value
        // itself rather than choosing an argument width.
        if !matches!(major, Major::Simple) && ai != head::ai_of(arg) {
            return Err(E::NOT_CANONICAL);
        }

        match major {
            // A tag wraps exactly one more item: defer the counter
            // decrement so the tagged payload completes this element.
            Major::Tag => {
                bignum_tag = matches!(arg, 2 | 3).then_some(arg);
                continue;
            }
            Major::Unsigned | Major::Negative => {}
            Major::Bytes | Major::Text => {
                if was_bignum.is_some() && !bignum_preferred_content(d, arg) {
                    return Err(E::NOT_CANONICAL);
                }
                let len = d.content_len(arg)?;
                d.pos += len;
            }
            Major::Simple => check_simple(ai, arg, d)?,
            Major::Array | Major::Map => {
                let items = match major {
                    Major::Map => usize::try_from(arg.saturating_mul(2))
                        .map_err(|_| E::INTEGER_OUT_OF_RANGE)?,
                    _ => usize::try_from(arg).map_err(|_| E::INTEGER_OUT_OF_RANGE)?,
                };
                if items > 0 {
                    sp = push(
                        &mut stack,
                        sp,
                        Frame { left: items, map: major == Major::Map, prev_key: None },
                    )?;
                    continue;
                }
            }
        }

        // Completed element spans `[start, d.pos)`; unwind finished scopes,
        // checking map key ordering on key slots.
        if unwind(&mut stack, &mut sp, d.input, start, d.position())? {
            return Ok(());
        }
    }
}

/// Preferred serialization of bignum payloads (§3.4.3): the magnitude has
/// no leading zeroes and is never empty — a value expressible as a basic
/// integer must be encoded as one instead.
fn bignum_preferred_content(d: &Decoder<'_>, len: u64) -> bool {
    // Called right after the byte-string head: the content lies ahead.
    let start = d.position();
    match d.input.get(start..start + len as usize) {
        Some(content) => !content.is_empty() && content[0] != 0,
        None => false, // truncated; the walk reports TRUNCATED later
    }
}

/// Two-byte simples below 32 are invalid everywhere (§3.3). Float widths
/// are checked for shortest-form minimality when floats are supported.
fn check_simple(ai: u8, arg: u64, d: &Decoder<'_>) -> Result<(), E> {
    #[cfg(not(feature = "float"))]
    let _ = d;
    match ai {
        0..=23 => Ok(()),
        24 if arg >= 32 => Ok(()),
        #[cfg(feature = "float")]
        25..=27 => check_float_width(ai, d),
        // Without float support the payload cannot be interpreted, so
        // width minimality of floats stays unverifiable and is accepted.
        #[cfg(not(feature = "float"))]
        25..=27 => Ok(()),
        _ => Err(E::NOT_CANONICAL),
    }
}

/// A float item is canonical only when encoded at its shortest exact width
/// (§4.2.2), mirroring `Encoder::f32`/`f64` selection.
#[cfg(feature = "float")]
fn check_float_width(ai: u8, d: &Decoder<'_>) -> Result<(), E> {
    use crate::float::{f16_bits_of, f32_of_f16_bits};
    let ext = head::ext_len(ai);
    let bits = &d.input[d.pos - ext..d.pos];
    match ai {
        // Half is always the narrowest possible form.
        25 => Ok(()),
        26 => {
            let v = f32::from_bits(u32::from_be_bytes([bits[0], bits[1], bits[2], bits[3]]));
            match f16_bits_of(v).is_some() {
                true => Err(E::NOT_CANONICAL), // could have been a half
                false => Ok(()),
            }
        }
        _ => {
            let v = f64::from_bits(u64::from_be_bytes([
                bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7],
            ]));
            let fits_half =
                f16_bits_of(v as f32).is_some_and(|h| f64::from(f32_of_f16_bits(h)) == v);
            let fits_single = f64::from(v as f32) == v;
            // Any NaN has a canonical half form (0x7e00 / 0xfe00).
            match fits_half || fits_single || v.is_nan() {
                true => Err(E::NOT_CANONICAL),
                false => Ok(()),
            }
        }
    }
}

fn push(stack: &mut [Frame; SKIP_LIMIT], sp: usize, frame: Frame) -> Result<usize, E> {
    match sp < SKIP_LIMIT {
        true => {
            stack[sp] = frame;
            Ok(sp + 1)
        }
        false => Err(E::DEPTH_EXCEEDED),
    }
}

/// Decrements counters after a completed element spanning `[start, end)`;
/// returns `true` when the virtual root completed. Key slots inside maps
/// must be strictly increasing bytewise (RFC §4.2.1 "sorting", which also
/// rules out duplicate keys).
fn unwind(
    stack: &mut [Frame; SKIP_LIMIT],
    sp: &mut usize,
    input: &[u8],
    start: usize,
    end: usize,
) -> Result<bool, E> {
    loop {
        let top = &mut stack[*sp - 1];
        if top.map && top.left.is_multiple_of(2) {
            if let Some((ps, pe)) = top.prev_key
                && input[ps..pe] >= input[start..end]
            {
                return Err(E::NOT_CANONICAL); // duplicate or out of order
            }
            top.prev_key = Some((start, end));
        }
        top.left -= 1;
        if top.left != 0 {
            return Ok(false);
        }
        if *sp == 1 {
            return Ok(true);
        }
        *sp -= 1;
    }
}
