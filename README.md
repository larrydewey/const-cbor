# const-cbor

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-2024-blue.svg)](https://www.rust-lang.org/)
[![No Std](https://img.shields.io/badge/no__std-compatible-green.svg)](https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute)

A `no_std` compatible Rust library for Concise Binary Object Representation (CBOR) encoding and decoding that supports compile-time operations via `const fn`.

## Features

- **`no_std` compatible**: Designed for resource-constrained environments like embedded systems
- **Compile-time support**: Uses `const fn` for operations that can be performed at compile time
- **Zero-copy design**: Value references data it doesn't own, minimizing memory allocation
- **Standards compliant**: Implements [RFC 7049](https://tools.ietf.org/html/rfc7049) (CBOR)
- **Lightweight**: Small code size with minimal dependencies
- **Comprehensive**: Supports all CBOR data types
- **Safety focused**: No unsafe code used (`#![deny(unsafe_code)]`)

## What is CBOR?

Concise Binary Object Representation (CBOR) is a binary data format designed for small message size, with extensibility without version negotiation. It's similar to JSON but with a more compact binary representation, making it ideal for constrained environments.

CBOR's design goals include:
- Extremely small code size
- Compact message size
- Deterministic encoding
- Extensibility without version negotiation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
const-cbor = "0.2.0"
```

### Basic Example

```rust
use const_cbor::{Value, encode::encode};

// Create a CBOR value
let value = Value::unsigned(42);
let mut buf = [0u8; 16];

// Encode it to a buffer
let size = encode(&value, &mut buf).unwrap();
assert_eq!(size, 2);
assert_eq!(buf[0], 0x18);
assert_eq!(buf[1], 42);
```

### Complex Structures

```rust
use const_cbor::{Value, encode};

// Create a more complex CBOR structure
let address_fields = [
    (Value::text("street"), Value::text("123 Main St")),
    (Value::text("city"), Value::text("Techville")),
    (Value::text("postal_code"), Value::text("12345")),
];

let person = [
    (Value::text("name"), Value::text("Alex Smith")),
    (Value::text("age"), Value::unsigned(42)),
    (Value::text("active"), Value::bool(true)),
    (Value::text("address"), Value::map(&address_fields)),
];

let document = Value::map(&person);

// Calculate required buffer size
let size_needed = encode::encoded_size(&document);

// Allocate a buffer
let mut buffer = vec![0u8; size_needed];

// Encode the document
let encoded_size = encode::encode(&document, &mut buffer).unwrap();
```

## Supported Data Types

- **Unsigned integers** (0 to 2^64-1)
- **Negative integers** (-1 to -2^64)
- **Byte strings** (arbitrary binary data)
- **Text strings** (UTF-8 encoded text)
- **Arrays** (ordered sequences of data items)
- **Maps** (collections of key-value pairs)
- **Tagged values** (data items with semantic tags)
- **Simple values** (including boolean, null, and undefined)
- **Floating-point numbers** (IEEE 754 double-precision)

## Safety and Constraints

This library follows strict Rust safety practices:

- No unsafe code is used anywhere in the codebase
- All public functions are thoroughly documented
- Comprehensive test coverage ensures reliability
- The codebase follows strict linting rules for quality

## Error Handling

The library uses a simple error handling approach suitable for `no_std` environments:

- `BufferOverflow`: Returned when the output buffer is too small
- `InvalidType`: Returned when the input contains invalid or unsupported CBOR data

## Future Plans

- Support for more specialized CBOR data types
- Additional const-context optimizations
- Stream-based encoding?

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributions

Contributions are welcome! Please feel free to submit a Pull Request.
