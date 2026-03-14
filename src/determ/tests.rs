// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests for deterministic value types
#![allow(clippy::approx_constant)]

use crate::determ::DetermValue;

#[test]
fn test_determ_value_integer_hash() {
    // Test that integer values hash deterministically
    let v1 = DetermValue::Integer(42);
    let v2 = DetermValue::Integer(42);
    let v3 = DetermValue::Integer(43);

    // Same values should have same hash
    assert_eq!(v1.hash(), v2.hash());
    // Different values should have different hashes (with high probability)
    assert_ne!(v1.hash(), v3.hash());

    // Hash should be deterministic
    let expected_hash = v1.hash();
    assert_eq!(v1.hash(), expected_hash);
    assert_eq!(v2.hash(), expected_hash);
}

#[test]
fn test_determ_value_inline_text() {
    // Test inline text (15 bytes or less)
    let short_text = DetermValue::InlineText(
        [b'h', b'e', b'l', b'l', b'o', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        5,
    );

    // Hash should be deterministic
    let hash1 = short_text.hash();
    let hash2 = short_text.hash();
    assert_eq!(hash1, hash2);

    // Same content should have same hash
    let short_text2 = DetermValue::InlineText(
        [b'h', b'e', b'l', b'l', b'o', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        5,
    );
    assert_eq!(short_text.hash(), short_text2.hash());
}

#[test]
fn test_determ_value_encode_roundtrip() {
    // Test encode/decode for various value types
    let test_cases = vec![
        DetermValue::Null,
        DetermValue::Integer(42),
        DetermValue::Integer(-12345),
        DetermValue::Float(3.14159),
        DetermValue::Boolean(true),
        DetermValue::Boolean(false),
        DetermValue::Timestamp(1234567890),
    ];

    for original in test_cases {
        let encoded = original.encode();
        let decoded = DetermValue::decode(&encoded).unwrap();

        // Roundtrip should preserve value
        match (&original, &decoded) {
            (DetermValue::Float(a), DetermValue::Float(b)) => {
                // Float comparison with NaN handling
                if a.is_nan() && b.is_nan() {
                    // Both NaN, that's fine
                } else {
                    assert_eq!(a, b);
                }
            }
            _ => assert_eq!(original, decoded, "Roundtrip failed for {:?}", original),
        }
    }
}

#[test]
fn test_determ_value_data_type() {
    assert_eq!(DetermValue::Null.data_type(), 0);
    assert_eq!(DetermValue::Integer(42).data_type(), 1);
    assert_eq!(DetermValue::Float(3.14).data_type(), 2);
    assert_eq!(DetermValue::InlineText([0u8; 15], 0).data_type(), 3);
    assert_eq!(DetermValue::HeapText(Box::new([])).data_type(), 4);
    assert_eq!(DetermValue::Boolean(true).data_type(), 5);
    assert_eq!(DetermValue::Timestamp(123).data_type(), 6);
    assert_eq!(DetermValue::Extension(Box::new([])).data_type(), 7);
}

#[test]
fn test_determ_value_is_null() {
    assert!(DetermValue::Null.is_null());
    assert!(!DetermValue::Integer(0).is_null());
    assert!(!DetermValue::Boolean(false).is_null());
}

#[test]
fn test_determ_value_encoded_len() {
    // Null: 1 byte for type tag
    assert_eq!(DetermValue::Null.encoded_len(), 1);

    // Integer: 1 byte type + 8 bytes value
    assert_eq!(DetermValue::Integer(42).encoded_len(), 9);

    // Float: 1 byte type + 8 bytes value
    assert_eq!(DetermValue::Float(3.14).encoded_len(), 9);

    // Boolean: 1 byte type + 1 byte value
    assert_eq!(DetermValue::Boolean(true).encoded_len(), 2);

    // Timestamp: 1 byte type + 8 bytes value
    assert_eq!(DetermValue::Timestamp(123).encoded_len(), 9);
}
