// SPDX-License-Identifier: MIT

use const_cbor::{Value, encode};

// Integration test that creates a complex nested structure and encodes it
#[test]
fn test_complex_document() {
    // Create a hierarchical structure representing a person record
    let empty_array = &[];
    let tags = [
        Value::text("rust"),
        Value::text("cbor"),
        Value::text("const"),
    ];

    let address_fields = [
        (Value::text("street"), Value::text("123 Main St")),
        (Value::text("city"), Value::text("Techville")),
        (Value::text("postal_code"), Value::text("12345")),
    ];

    // Store the timestamp in a variable to extend its lifetime
    let timestamp = Value::text("2024-05-20T10:30:00Z");

    let person_record = [
        (Value::text("name"), Value::text("Alex Smith")),
        (Value::text("age"), Value::unsigned(42)),
        (Value::text("active"), Value::bool(true)),
        (Value::text("address"), Value::map(&address_fields)),
        (Value::text("tags"), Value::array(&tags)),
        (Value::text("notes"), Value::null()),
        (Value::text("scores"), Value::array(empty_array)),
        (Value::text("balance"), Value::float(123.45)),
        (Value::text("created_at"), Value::tag(0, &timestamp)),
    ];

    let document = Value::map(&person_record);

    // Calculate the size
    let size_needed = encode::encoded_size(&document);

    // Allocate a buffer
    let mut buffer = vec![0u8; size_needed];

    // Encode the document
    let result = encode::encode(&document, &mut buffer);
    assert!(result.is_ok());
    let encoded_size = result.unwrap();

    // Size prediction should match actual size
    assert_eq!(encoded_size, size_needed);

    // We don't check the exact bytes here since that would be very verbose,
    // but in a real application you might decode it back and verify the content
}

// Test that encoding and size calculation handle empty/edge cases correctly
#[test]
fn test_edge_cases() {
    // Empty array
    let empty_array = Value::array(&[]);
    let mut buf = [0u8; 10];
    let size = encode::encode(&empty_array, &mut buf).unwrap();
    assert_eq!(size, 1);
    assert_eq!(buf[0], 0x80); // Array of length 0
    assert_eq!(encode::encoded_size(&empty_array), 1);

    // Empty map
    let empty_map = Value::map(&[]);
    let size = encode::encode(&empty_map, &mut buf).unwrap();
    assert_eq!(size, 1);
    assert_eq!(buf[0], 0xA0); // Map of length 0
    assert_eq!(encode::encoded_size(&empty_map), 1);

    // Integer boundary cases
    // Max u8
    let max_u8 = Value::unsigned(u8::MAX as u64);
    let size = encode::encode(&max_u8, &mut buf).unwrap();
    assert_eq!(size, 2);
    assert_eq!(buf[0], 0x18);
    assert_eq!(buf[1], 0xFF);

    // Min u16
    let min_u16 = Value::unsigned(u8::MAX as u64 + 1);
    let size = encode::encode(&min_u16, &mut buf).unwrap();
    assert_eq!(size, 3);
    assert_eq!(buf[0], 0x19);
    assert_eq!(buf[1], 0x01);
    assert_eq!(buf[2], 0x00);
}

// Test maximum values that can be encoded at each size
#[test]
fn test_boundary_values() {
    let mut buf = [0u8; 16];

    // Smallest value that needs 2 bytes
    let val24 = Value::unsigned(24);
    let size = encode::encode(&val24, &mut buf).unwrap();
    assert_eq!(size, 2);
    assert_eq!(buf[0], 0x18);
    assert_eq!(buf[1], 24);

    // Largest value that fits in 1 byte of additional info
    let val_u8_max = Value::unsigned(u8::MAX as u64);
    let size = encode::encode(&val_u8_max, &mut buf).unwrap();
    assert_eq!(size, 2);
    assert_eq!(buf[0], 0x18);
    assert_eq!(buf[1], 0xFF);

    // Smallest value that needs 2 bytes of additional info
    let val_u8_max_plus_1 = Value::unsigned(u8::MAX as u64 + 1);
    let size = encode::encode(&val_u8_max_plus_1, &mut buf).unwrap();
    assert_eq!(size, 3);
    assert_eq!(buf[0], 0x19);
    assert_eq!(buf[1], 0x01);
    assert_eq!(buf[2], 0x00);

    // Largest value that fits in 2 bytes of additional info
    let val_u16_max = Value::unsigned(u16::MAX as u64);
    let size = encode::encode(&val_u16_max, &mut buf).unwrap();
    assert_eq!(size, 3);
    assert_eq!(buf[0], 0x19);
    assert_eq!(buf[1], 0xFF);
    assert_eq!(buf[2], 0xFF);

    // Smallest value that needs 4 bytes of additional info
    let val_u16_max_plus_1 = Value::unsigned(u16::MAX as u64 + 1);
    let size = encode::encode(&val_u16_max_plus_1, &mut buf).unwrap();
    assert_eq!(size, 5);
    assert_eq!(buf[0], 0x1A);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x01);
    assert_eq!(buf[3], 0x00);
    assert_eq!(buf[4], 0x00);
}
