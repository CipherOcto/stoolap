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

//! DQA (Deterministic Quant Arithmetic) Integration Tests
//!
//! Tests DQA type via Value API: round-trip, format, comparison, serialization

use stoolap::core::Value;
use octo_determin::dqa::Dqa;

/// Test DQA format at scale 0
#[test]
fn test_dqa_format_scale_0() {
    let dqa = Dqa::new(42, 0).unwrap();
    let value = Value::quant(dqa);

    let formatted = value.as_string().expect("DQA should format");
    assert_eq!(formatted, "42", "scale 0 should format as integer");
}

/// Test DQA format at scale 1
#[test]
fn test_dqa_format_scale_1() {
    let dqa = Dqa::new(123, 1).unwrap(); // 12.3
    let value = Value::quant(dqa);

    let formatted = value.as_string().expect("DQA should format");
    assert_eq!(formatted, "12.3", "scale 1 should format with 1 decimal");
}

/// Test DQA format at scale 2
#[test]
fn test_dqa_format_scale_2() {
    let dqa = Dqa::new(12345, 2).unwrap(); // 123.45
    let value = Value::quant(dqa);

    let formatted = value.as_string().expect("DQA should format");
    assert_eq!(formatted, "123.45", "scale 2 should format with 2 decimals");
}

/// Test DQA format at scale 9
#[test]
fn test_dqa_format_scale_9() {
    let dqa = Dqa::new(123456789, 9).unwrap(); // 0.123456789
    let value = Value::quant(dqa);

    let formatted = value.as_string().expect("DQA should format");
    assert_eq!(formatted, "0.123456789", "scale 9 should format correctly");
}

/// Test DQA format at scale 18
#[test]
fn test_dqa_format_scale_18() {
    let dqa = Dqa::new(123456789012345678, 18).unwrap();
    let value = Value::quant(dqa);

    let formatted = value.as_string().expect("DQA should format at scale 18");
    // Should be 0.123456789012345678
    assert!(formatted.starts_with("0."));
}

/// Test DQA as_float64 conversion
#[test]
fn test_dqa_as_float64() {
    let dqa = Dqa::new(12345, 2).unwrap(); // 123.45
    let value = Value::quant(dqa);

    let f64_val = value.as_float64().expect("DQA should convert to f64");
    assert_eq!(f64_val, 123.45, "as_float64 should return numeric value");
}

/// Test DQA to Text coercion (via as_string)
#[test]
fn test_dqa_to_text_coercion() {
    let dqa = Dqa::new(12345, 2).unwrap(); // 123.45
    let value = Value::quant(dqa);

    let text = value.as_string().expect("DQA should convert to text");
    assert_eq!(text, "123.45");
}

/// Test DQA cross-type comparison with Integer
#[test]
fn test_dqa_vs_integer_comparison() {
    let dqa = Value::quant(Dqa::new(100, 0).unwrap()); // 100
    let int_val = Value::integer(100);

    // DQA 100 vs Integer 100 should be equal
    let cmp = dqa.compare(&int_val).expect("comparison should work");
    assert_eq!(cmp, std::cmp::Ordering::Equal);

    let dqa_less = Value::quant(Dqa::new(50, 0).unwrap());
    let cmp2 = dqa_less.compare(&int_val).expect("comparison should work");
    assert_eq!(cmp2, std::cmp::Ordering::Less);

    let dqa_greater = Value::quant(Dqa::new(200, 0).unwrap());
    let cmp3 = dqa_greater.compare(&int_val).expect("comparison should work");
    assert_eq!(cmp3, std::cmp::Ordering::Greater);
}

/// Test DQA cross-type comparison with Float
#[test]
fn test_dqa_vs_float_comparison() {
    let dqa = Value::quant(Dqa::new(12345, 2).unwrap()); // 123.45
    let float_val = Value::float(123.45);

    let cmp = dqa.compare(&float_val).expect("comparison should work");
    assert_eq!(cmp, std::cmp::Ordering::Equal);
}

/// Test DQA serialization round-trip
#[test]
fn test_dqa_serialization_roundtrip() {
    use stoolap::storage::mvcc::persistence::{serialize_value, deserialize_value};

    let dqa = Dqa::new(12345, 2).unwrap(); // 123.45
    let value = Value::quant(dqa);

    let serialized = serialize_value(&value).expect("should serialize");
    let deserialized = deserialize_value(&serialized).expect("should deserialize");

    let deserialized_dqa = deserialized.as_dqa().expect("should be DQA");
    assert_eq!(dqa.value, deserialized_dqa.value);
    assert_eq!(dqa.scale, deserialized_dqa.scale);
}

/// Test DQA negative value round-trip
#[test]
fn test_dqa_negative_roundtrip() {
    use stoolap::storage::mvcc::persistence::{serialize_value, deserialize_value};

    let dqa = Dqa::new(-12345, 2).unwrap(); // -123.45
    let value = Value::quant(dqa);

    let serialized = serialize_value(&value).expect("should serialize");
    let deserialized = deserialize_value(&serialized).expect("should deserialize");

    let deserialized_dqa = deserialized.as_dqa().expect("should be DQA");
    assert_eq!(dqa.value, deserialized_dqa.value);
    assert_eq!(dqa.scale, deserialized_dqa.scale);
}

/// Test DQA zero round-trip
#[test]
fn test_dqa_zero_roundtrip() {
    use stoolap::storage::mvcc::persistence::{serialize_value, deserialize_value};

    let dqa = Dqa::new(0, 0).unwrap();
    let value = Value::quant(dqa);

    let serialized = serialize_value(&value).expect("should serialize");
    let deserialized = deserialize_value(&serialized).expect("should deserialize");

    let deserialized_dqa = deserialized.as_dqa().expect("should be DQA");
    assert_eq!(dqa.value, deserialized_dqa.value);
    assert_eq!(dqa.scale, deserialized_dqa.scale);
}