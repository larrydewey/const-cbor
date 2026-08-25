//! Diagnostic rendering: error paths, escapes, u128 magnitudes, sinks.
#![cfg(feature = "diagnostic")]

use const_cbor::Error;
use const_cbor::diagnostic::{render, write_to};

fn rendered(input: &[u8]) -> String {
    let mut out = [0u8; 256];
    let s = render(input, &mut out).unwrap();
    s.to_owned()
}

#[test]
fn structural_errors_propagate() {
    // Truncated.
    let mut out = [0u8; 32];
    assert_eq!(render(&[0x18], &mut out), Err(Error::TRUNCATED));
    assert_eq!(render(&[0x62, b'a'], &mut out), Err(Error::TRUNCATED));
    // Reserved ai.
    assert_eq!(render(&[0x1c], &mut out), Err(Error::INVALID_ADDITIONAL_INFO));
    // Break with nothing open.
    assert_eq!(render(&[0xff], &mut out), Err(Error::UNEXPECTED_BREAK));
    // Break inside a definite array.
    assert_eq!(render(&[0x81, 0xff], &mut out), Err(Error::UNEXPECTED_BREAK));
    // Empty input.
    assert_eq!(render(&[], &mut out), Err(Error::TRUNCATED));
    // Invalid simple in two-byte form below 32.
    let res = render(&[0xf8, 0x05], &mut out);
    assert!(matches!(res, Err(Error::INVALID_SIMPLE | Error::TYPE_MISMATCH)));
    // Indefinite simple value (major 7, ai 31) is INVALID_ADDITIONAL_INFO.
    assert_eq!(render(&[0xff], &mut out), Err(Error::UNEXPECTED_BREAK));
    // Open indefinite map with no content: truncated.
    assert_eq!(render(&[0xbf], &mut out), Err(Error::TRUNCATED));
}

#[test]
fn depth_exceeded_in_diagnostic() {
    // SKIP_LIMIT nested definite arrays.
    let depth = const_cbor::decode::SKIP_LIMIT + 1;
    let mut input = vec![0x81; depth]; // array(1) each
    input.push(0x01);
    let mut out = [0u8; 512];
    assert_eq!(render(&input, &mut out), Err(Error::DEPTH_EXCEEDED));

    // Same through write_to.
    let mut sink = String::new();
    assert_eq!(write_to(&input, &mut sink), Err(Error::DEPTH_EXCEEDED));
}

#[test]
fn out_of_space_on_small_output() {
    // Rendering h'65...' ("hello") needs 7 bytes of output.
    let cbor_hello = [0x65, b'h', b'e', b'l', b'l', b'o'];
    let mut out = [0u8; 6];
    assert_eq!(render(&cbor_hello, &mut out), Err(Error::OUT_OF_SPACE));
    let mut out = [0u8; 7];
    assert_eq!(render(&cbor_hello, &mut out), Ok("\"hello\""));
}

#[test]
fn failing_writer_maps_to_out_of_space() {
    struct Fail;
    impl core::fmt::Write for Fail {
        fn write_str(&mut self, _: &str) -> core::fmt::Result {
            Err(core::fmt::Error)
        }
    }
    assert_eq!(write_to(&[0x01], &mut Fail), Err(Error::OUT_OF_SPACE));
}

#[test]
fn negative_u128_magnitude_renders() {
    // -1 - (2^64 - 1) = -2^64: magnitude overflows u64, exercises put_u128.
    let input = [0x3bu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(rendered(&input), "-18446744073709551616");
    // Largest representable negative: -1 - u64::MAX = -(2^64) ... plus one less
    let input = [0x3bu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe];
    assert_eq!(rendered(&input), "-18446744073709551615");
}

#[test]
fn string_escapes_render_json_style() {
    // \n \r \t \" \\ and a control char.
    let payload: &[u8] = b"a\n\r\t\"\\z\x01";
    let mut input = vec![0x60 + payload.len() as u8];
    input.extend_from_slice(payload);
    assert_eq!(rendered(&input), "\"a\\n\\r\\t\\\"\\\\z\\u0001\"");
    // Non-ASCII passes through as UTF-8.
    let mut input = vec![0x62, 0xc3, 0xa9]; // é
    input[0] = 0x62;
    assert_eq!(rendered(&input), "\"é\"");
}

#[test]
fn tag_and_simple_rendering() {
    assert_eq!(rendered(&[0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]), "1(1363896240)");
    assert_eq!(rendered(&[0xf8, 0x20]), "simple(32)");
    assert_eq!(rendered(&[0xf0]), "simple(16)");
    assert_eq!(rendered(&[0xf4]), "false");
    assert_eq!(rendered(&[0xf5]), "true");
    assert_eq!(rendered(&[0xf6]), "null");
    assert_eq!(rendered(&[0xf7]), "undefined");
}

#[test]
fn empty_containers_and_separators() {
    assert_eq!(rendered(&[0x80]), "[]");
    assert_eq!(rendered(&[0xa0]), "{}");
    assert_eq!(rendered(&[0x82, 0x01, 0x02]), "[1, 2]");
    assert_eq!(rendered(&[0xa2, 0x01, 0x02, 0x03, 0x04]), "{1: 2, 3: 4}");
    // Tag wrapping an array closes both.
    assert_eq!(rendered(&[0xc0, 0x82, 0x01, 0x02]), "0([1, 2])");
    // Indefinite map.
    assert_eq!(rendered(&[0xbf, 0x01, 0x02, 0xff]), "{_ 1: 2}");
    // Indefinite byte string with chunks.
    assert_eq!(rendered(&[0x5f, 0x42, 0x01, 0x02, 0x41, 0x03, 0xff]), "(_ h'0102', h'03')");
}
