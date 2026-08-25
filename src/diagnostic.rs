//! Human-readable diagnostic output (RFC 8949 §8, Appendix A), feature
//! `diagnostic`.
//!
//! Renders CBOR into the RFC's diagnostic notation — e.g.
//! `{ "a": 1, "b": [_ 2, 3] }` — without allocation: output goes into a
//! caller-provided byte buffer or any [`core::fmt::Write`] sink. Nesting is
//! traversed iteratively over a fixed stack; input nested deeper than
//! [`SKIP_LIMIT`] yields [`Error::DEPTH_EXCEEDED`] rather than overflow.
//!
//! Floats render as shortest-roundtrip decimals (`1.5`, `NaN`, `Infinity`)
//! when the `float` feature is on, and as `float(0x…)` bit patterns
//! otherwise.

use crate::decode::SKIP_LIMIT;
use crate::error::Error;
use crate::head::{self, Major};

/// Sentinel meaning "open-ended scope, terminated by a break byte".
const OPEN: u64 = u64::MAX;

/// Byte-accepting output target that never allocates.
trait Sink {
    /// # Errors
    ///
    /// [`Error::OUT_OF_SPACE`] when the underlying target cannot fit `s`.
    fn put(&mut self, s: &str) -> Result<(), Error>;
}

impl Sink for TextBuf<'_> {
    fn put(&mut self, s: &str) -> Result<(), Error> {
        let Some(end) = self.pos.checked_add(s.len()) else {
            return Err(Error::OUT_OF_SPACE);
        };
        match end <= self.buf.len() {
            true => {
                self.buf[self.pos..end].copy_from_slice(s.as_bytes());
                self.pos = end;
                Ok(())
            }
            false => Err(Error::OUT_OF_SPACE),
        }
    }
}

/// Fixed-buffer output wrapper tracking the written length.
struct TextBuf<'b> {
    buf: &'b mut [u8],
    pos: usize,
}

/// Writes `input`'s diagnostic notation into `out`, returning the rendered
/// slice.
///
/// Style follows RFC 8949 Appendix A: single line, `, ` between siblings,
/// `: ` between map keys and values, `[_ `/`{_ `/`(_ ` markers for
/// indefinite lengths, `tag(item)` for tags.
///
/// # Errors
///
/// Structural decode errors propagate unchanged; [`Error::OUT_OF_SPACE`]
/// when `out` cannot hold the rendering; [`Error::DEPTH_EXCEEDED`] beyond
/// [`SKIP_LIMIT`] nesting levels.
pub fn render<'o>(input: &[u8], out: &'o mut [u8]) -> Result<&'o str, Error> {
    let mut buf = TextBuf { buf: out, pos: 0 };
    write_to_sink(input, &mut buf)?;
    // Only UTF-8 literals ever reach the buffer.
    Ok(core::str::from_utf8(&buf.buf[..buf.pos]).unwrap_or(""))
}

/// Writes `input`'s diagnostic notation to any format writer, forwarding
/// without intermediate buffering.
///
/// # Errors
///
/// As [`render`].
pub fn write_to(input: &[u8], w: &mut dyn core::fmt::Write) -> Result<(), Error> {
    struct W<'a>(&'a mut (dyn core::fmt::Write + 'a));
    impl Sink for W<'_> {
        fn put(&mut self, s: &str) -> Result<(), Error> {
            core::fmt::Write::write_str(self.0, s).map_err(|_| Error::OUT_OF_SPACE)
        }
    }
    let mut sink = W(w);
    write_to_sink(input, &mut sink)
}

fn write_to_sink(input: &[u8], out: &mut dyn Sink) -> Result<(), Error> {
    let mut dec = crate::Decoder::new(input);
    // A virtual root frame expecting exactly one item makes termination
    // uniform.
    let mut stack = [Frame { major: Major::Array, indef: false, left: 1, done: 0 }; SKIP_LIMIT];
    let mut sp = 1usize;

    loop {
        // A break byte closes the innermost open-ended scope.
        if matches!(dec.input.get(dec.pos), Some(&b) if head::is_break(b)) {
            dec.pos += 1;
            match sp > 1 && stack[sp - 1].indef {
                true => {}
                false => return Err(Error::UNEXPECTED_BREAK),
            }
            // Pop the open-ended scope; the break itself is its terminator.
            let closed = stack[sp - 1].major;
            sp -= 1;
            out.put(match closed {
                Major::Array => "]",
                Major::Map => "}",
                _ => ")",
            })?;
            if complete(out, &mut stack, &mut sp)? {
                return Ok(());
            }
            continue;
        }

        separator(out, &stack[sp - 1])?;

        let head_start = dec.pos;
        let (major, arg, indef) = dec.next_head_ext()?;
        // Additional information straight from the initial byte: needed to
        // recognize float widths among major type 7 items.
        let ai = dec.input[head_start] & 0x1f;

        match major {
            Major::Unsigned => put_uint(out, arg)?,
            Major::Negative => {
                // Value is -1 - n over the full u64 magnitude range.
                out.put("-")?;
                let mag = u128::from(arg) + 1;
                match u64::try_from(mag) {
                    Ok(m) => put_uint(out, m),
                    Err(_) => put_u128(out, mag),
                }?;
            }
            Major::Bytes | Major::Text if indef => {
                // Header only: definite chunks follow as children of the
                // new frame, terminated by a break byte.
                out.put("(_ ")?;
                sp = push(&mut stack, sp, Frame { major, indef: true, left: OPEN, done: 0 })?;
                continue;
            }
            Major::Bytes | Major::Text => {
                let len = dec.content_len(arg)?;
                let payload = &dec.input[dec.pos..dec.pos + len];
                dec.pos += len;
                match major {
                    Major::Bytes => put_hex(out, payload)?,
                    _ => put_escaped(out, payload)?,
                }
            }
            Major::Array | Major::Map => {
                let (bracket, count) = match (major, indef) {
                    (Major::Array, false) => ("[", arg),
                    (Major::Map, false) => ("{", arg.saturating_mul(2)),
                    (_, true) if major == Major::Array => ("[_ ", OPEN),
                    (_, true) => ("{_ ", OPEN),
                    _ => unreachable!("all arms covered"),
                };
                out.put(bracket)?;
                match count {
                    OPEN | 1.. => {
                        sp = push(&mut stack, sp, Frame { major, indef, left: count, done: 0 })?;
                        continue;
                    }
                    _ => {
                        // Empty container: closer immediately, then treat
                        // as a completed item in the parent.
                        out.put(match major {
                            Major::Array => "]",
                            _ => "}",
                        })?;
                    }
                }
            }
            Major::Tag => {
                put_uint(out, arg)?;
                out.put("(")?;
                sp = push(
                    &mut stack,
                    sp,
                    Frame { major: Major::Tag, indef: false, left: 1, done: 0 },
                )?;
                continue; // tagged item follows without a sibling separator
            }
            Major::Simple => render_simple(&mut dec, out, arg, ai, indef)?,
        }

        if complete(out, &mut stack, &mut sp)? {
            return Ok(());
        }
    }
}

/// One open container or tag wrapper on the rendering stack.
#[derive(Clone, Copy)]
struct Frame {
    major: Major,
    /// Open-ended (break-terminated) scope.
    indef: bool,
    /// Items still expected (pairs×2 for maps); meaningless when `indef`.
    left: u64,
    /// Items rendered so far in this scope.
    done: u32,
}

#[inline]
fn push(stack: &mut [Frame; SKIP_LIMIT], sp: usize, frame: Frame) -> Result<usize, Error> {
    match sp < SKIP_LIMIT {
        true => {
            stack[sp] = frame;
            Ok(sp + 1)
        }
        false => Err(Error::DEPTH_EXCEEDED),
    }
}

/// Emits `, ` / `: ` / nothing based on how many items the frame already
/// rendered (odd counts inside maps are values).
fn separator(out: &mut dyn Sink, frame: &Frame) -> Result<(), Error> {
    match frame.done {
        0 => Ok(()),
        n if frame.major == Major::Map && n % 2 == 1 => out.put(": "),
        _ => out.put(", "),
    }
}

/// Marks the innermost frame as having rendered one more item, popping and
/// closing finished scopes outward. Returns `true` when the virtual root
/// completed (whole input rendered).
fn complete(
    out: &mut dyn Sink,
    stack: &mut [Frame; SKIP_LIMIT],
    sp: &mut usize,
) -> Result<bool, Error> {
    loop {
        let frame = stack[*sp - 1];
        if frame.indef {
            // Open-ended scopes wait for their break byte; still count the
            // finished item so sibling separators come out right.
            stack[*sp - 1].done += 1;
            return Ok(false);
        }
        let done = frame.done + 1;
        if done < frame.left as u32 || frame.left == OPEN {
            stack[*sp - 1].done = done;
            return Ok(false);
        }
        // The virtual root has no textual closer.
        if *sp == 1 {
            return Ok(true);
        }
        *sp -= 1;
        out.put(match frame.major {
            Major::Array => "]",
            Major::Map => "}",
            _ => ")",
        })?;
    }
}

#[inline]
fn put_uint(out: &mut dyn Sink, v: u64) -> Result<(), Error> {
    const DIGITS: &[u8; 10] = b"0123456789";
    let mut tmp = [0u8; 20];
    let mut i = tmp.len();
    let mut v = v;
    loop {
        i -= 1;
        tmp[i] = DIGITS[(v % 10) as usize];
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.put(core::str::from_utf8(&tmp[i..]).unwrap_or(""))
}

fn put_u128(out: &mut dyn Sink, v: u128) -> Result<(), Error> {
    const DIGITS: &[u8; 10] = b"0123456789";
    let mut tmp = [0u8; 40];
    let mut i = tmp.len();
    let mut v = v;
    loop {
        i -= 1;
        tmp[i] = DIGITS[(v % 10) as usize];
        v /= 10;
        if v == 0 {
            break;
        }
    }
    out.put(core::str::from_utf8(&tmp[i..]).unwrap_or(""))
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Lowercase hex per RFC diagnostic convention: h'0a1b'.
fn put_hex(out: &mut dyn Sink, bytes: &[u8]) -> Result<(), Error> {
    out.put("h'")?;
    let mut tmp = [0u8; 2];
    for &b in bytes {
        tmp[0] = HEX_DIGITS[usize::from(b >> 4)];
        tmp[1] = HEX_DIGITS[usize::from(b & 0x0f)];
        out.put(core::str::from_utf8(&tmp).unwrap_or(""))?;
    }
    out.put("'")
}

/// JSON-style escaping for text payloads; non-ASCII passes through as UTF-8.
fn put_escaped(out: &mut dyn Sink, s: &[u8]) -> Result<(), Error> {
    out.put("\"")?;
    let mut start = 0;
    for (i, &b) in s.iter().enumerate() {
        let esc: &str = match b {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f => {
                out.put(core::str::from_utf8(&s[start..i]).unwrap_or(""))?;
                put_u_escape(out, b)?;
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        out.put(core::str::from_utf8(&s[start..i]).unwrap_or(""))?;
        out.put(esc)?;
        start = i + 1;
    }
    out.put(core::str::from_utf8(&s[start..]).unwrap_or(""))?;
    out.put("\"")
}

fn put_u_escape(out: &mut dyn Sink, b: u8) -> Result<(), Error> {
    let tmp: [u8; 6] = [
        b'\\',
        b'u',
        b'0',
        b'0',
        HEX_DIGITS[usize::from(b >> 4)],
        HEX_DIGITS[usize::from(b & 0x0f)],
    ];
    out.put(core::str::from_utf8(&tmp).unwrap_or(""))
}

/// Renders a major type 7 leaf: booleans, null, undefined, simple values,
/// and floats (when present).
fn render_simple(
    dec: &mut crate::Decoder<'_>,
    out: &mut dyn Sink,
    arg: u64,
    ai: u8,
    indef: bool,
) -> Result<(), Error> {
    if indef {
        return Err(Error::INVALID_ADDITIONAL_INFO);
    }
    // Float widths are selected by additional information, not by the
    // argument (which carries the raw bit pattern).
    if matches!(ai, 25..=27) {
        let width = head::ext_len(ai);
        // next_head_ext already consumed (and bounds-checked) the argument
        // bytes, so the payload sits immediately behind the cursor.
        if dec.pos < width {
            return Err(Error::TRUNCATED);
        }
        let bits = &dec.input[dec.pos - width..dec.pos];
        #[cfg(feature = "float")]
        return put_float(out, bits);
        #[cfg(not(feature = "float"))]
        {
            out.put("float(0x")?;
            let mut tmp = [0u8; 2];
            for &b in bits {
                tmp[0] = HEX_DIGITS[usize::from(b >> 4)];
                tmp[1] = HEX_DIGITS[usize::from(b & 0x0f)];
                out.put(core::str::from_utf8(&tmp).unwrap_or(""))?;
            }
            return out.put(")");
        }
    }
    match arg {
        20 => out.put("false"),
        21 => out.put("true"),
        22 => out.put("null"),
        23 => out.put("undefined"),
        // All other simple values render as simple(n). Legality depends on
        // the encoding width: the two-byte form (ai 24) is forbidden below
        // 32 (RFC 8949 section 3.3); the one-byte form carries 0..=23.
        _ => {
            let legal = match ai {
                24 => arg >= 32,
                _ => arg < 24,
            };
            match legal {
                true => {
                    out.put("simple(")?;
                    put_uint(out, arg)?;
                    out.put(")")
                }
                false => Err(Error::INVALID_SIMPLE),
            }
        }
    }
}

/// Allocation-free adapter for `core::write!` into a fixed buffer.
#[cfg(feature = "float")]
struct Fmt<'a> {
    buf: &'a mut [u8],
    pos: usize,
}
#[cfg(feature = "float")]
impl core::fmt::Write for Fmt<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let Some(end) = self.pos.checked_add(s.len()) else {
            return Err(core::fmt::Error);
        };
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.pos..end].copy_from_slice(s.as_bytes());
        self.pos = end;
        Ok(())
    }
}

/// Formats a float value as shortest-roundtrip decimal text.
#[cfg(feature = "float")]
fn put_float(out: &mut dyn Sink, bits: &[u8]) -> Result<(), Error> {
    use crate::float::f32_of_f16_bits;

    let value: f64 = match bits.len() {
        2 => f64::from(f32_of_f16_bits(u16::from_be_bytes([bits[0], bits[1]]))),
        4 => f64::from(f32::from_bits(u32::from_be_bytes([bits[0], bits[1], bits[2], bits[3]]))),
        _ => f64::from_bits(u64::from_be_bytes([
            bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7],
        ])),
    };

    // RFC naming differs from Rust's.
    if value.is_nan() {
        return out.put("NaN");
    }
    if value.is_infinite() {
        return out.put(match value.is_sign_negative() {
            true => "-Infinity",
            false => "Infinity",
        });
    }

    let mut tmp = [0u8; 48];
    #[allow(clippy::items_after_statements)]
    let written = {
        use core::fmt::Write as _;
        let mut fmt = Fmt { buf: &mut tmp, pos: 0 };
        match core::write!(fmt, "{value}") {
            Ok(()) => fmt.pos,
            Err(_) => return Err(Error::OUT_OF_SPACE),
        }
    };
    out.put(core::str::from_utf8(&tmp[..written]).unwrap_or(""))
}
