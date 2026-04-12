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

//! Value type for Stoolap - runtime values with type information
//!
//! This module provides a unified Value enum that represents SQL values
//! with full type information and conversion capabilities.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use octo_determin::decimal::{decimal_cmp, decimal_to_string};
use octo_determin::{
    dqa_cmp, BigInt, BigIntError, Decimal, DecimalError, Dfp, DfpClass, DfpEncoding, Dqa,
};

use super::error::{Error, Result};
use super::types::DataType;
use crate::common::{CompactArc, SmartString};

/// Timestamp formats supported for parsing
/// Order matters - more specific formats first
const TIMESTAMP_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f%:z", // RFC3339 with fractional seconds
    "%Y-%m-%dT%H:%M:%S%:z",    // RFC3339
    "%Y-%m-%dT%H:%M:%SZ",      // RFC3339 UTC
    "%Y-%m-%dT%H:%M:%S",       // ISO without timezone
    "%Y-%m-%d %H:%M:%S%.f",    // SQL-style with fractional seconds
    "%Y-%m-%d %H:%M:%S",       // SQL-style
    "%Y-%m-%d",                // Date only
    "%Y/%m/%d %H:%M:%S",       // Alternative with slashes
    "%Y/%m/%d",                // Alternative date only
    "%m/%d/%Y",                // US format
    "%d/%m/%Y",                // European format
];

const TIME_FORMATS: &[&str] = &[
    "%H:%M:%S%.f", // High precision
    "%H:%M:%S",    // Standard
    "%H:%M",       // Hours and minutes only
];

/// A runtime value with type information
///
/// Each variant carries its data directly, avoiding the need for interface
/// indirection or separate value references.
///
/// ## Memory Layout (16 bytes)
///
/// Value is exactly 16 bytes due to niche optimization:
/// - Text(SmartString): 16 bytes with niches in tag byte (values 17-255 unused)
/// - Extension(CompactArc<[u8]>): 8 bytes (thin pointer), leaving niche bytes free
/// - Rust stores Value's discriminant in SmartString's niche values
///
/// ## Extension Variant
///
/// The Extension variant is a catch-all for all complex types (JSON, Vector, Blob, etc.)
/// It stores a single `CompactArc<[u8]>` (8 bytes) where byte[0] is the DataType tag
/// and byte[1..] is the payload. This keeps Value at exactly 7 variants forever —
/// new types are added by extending DataType (a 1-byte `#[repr(u8)]` enum).
///
/// Note: Text uses SmartString for inline storage of strings up to 15 bytes.
/// Longer strings use Arc<String> for O(1) clone and sharing.
#[derive(Debug, Clone)]
pub enum Value {
    /// NULL value with optional type hint
    Null(DataType),

    /// 64-bit signed integer
    Integer(i64),

    /// 64-bit floating point
    Float(f64),

    /// UTF-8 text string (SmartString: inline ≤15 bytes, Arc for larger)
    Text(SmartString),

    /// Boolean value
    Boolean(bool),

    /// Timestamp (UTC)
    Timestamp(DateTime<Utc>),

    /// Extension type: byte[0] = DataType tag, byte[1..] = payload
    /// - Json: byte[0]=6, byte[1..]=UTF-8 bytes (access via `as_json()`)
    /// - Vector: byte[0]=7, byte[1..]=packed LE f32 bytes (access via `as_vector_f32()`)
    /// - Future types add DataType variants, not Value variants
    Extension(CompactArc<[u8]>),

    /// Binary large object: raw byte data for cryptographic hashes, binary keys
    Blob(CompactArc<[u8]>),
}

/// Static NULL value for zero-cost reuse
pub const NULL_VALUE: Value = Value::Null(DataType::Null);

impl Value {
    // =========================================================================
    // Constructors
    // =========================================================================

    /// Create a NULL value with a type hint
    #[inline]
    pub fn null(data_type: DataType) -> Self {
        Value::Null(data_type)
    }

    /// Create a NULL value with unknown type
    #[inline(always)]
    pub fn null_unknown() -> Self {
        Value::Null(DataType::Null)
    }

    /// Create an integer value
    pub fn integer(value: i64) -> Self {
        Value::Integer(value)
    }

    /// Create a float value
    pub fn float(value: f64) -> Self {
        Value::Float(value)
    }

    /// Create a text value
    ///
    /// Uses SmartString::from_string_shared() for heap strings to enable
    /// O(1) clone via Arc<str>. This allows string sharing between
    /// Arena, Index, and VersionStore.
    pub fn text(value: impl Into<String>) -> Self {
        Value::Text(SmartString::from_string_shared(value.into()))
    }

    /// Create a text value from Arc<str> (zero-copy for heap strings)
    ///
    /// Preserves the Arc reference for O(1) clone and sharing.
    pub fn text_arc(value: Arc<str>) -> Self {
        Value::Text(SmartString::from(value))
    }

    /// Create a boolean value
    pub fn boolean(value: bool) -> Self {
        Value::Boolean(value)
    }

    /// Create a timestamp value
    pub fn timestamp(value: DateTime<Utc>) -> Self {
        Value::Timestamp(value)
    }

    /// Create a JSON value (stored as UTF-8 bytes in Extension, tag byte prepended)
    pub fn json(value: impl Into<String>) -> Self {
        let s_bytes = value.into().into_bytes();
        let mut bytes = Vec::with_capacity(1 + s_bytes.len());
        bytes.push(DataType::Json as u8);
        bytes.extend_from_slice(&s_bytes);
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a vector value from f32 data (stored as packed LE f32 bytes in Extension)
    pub fn vector(data: Vec<f32>) -> Self {
        let mut bytes = Vec::with_capacity(1 + data.len() * 4);
        bytes.push(DataType::Vector as u8);
        for f in &data {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a vector value from pre-packed f32 bytes (prepends tag)
    pub fn vector_from_bytes(raw_f32_bytes: CompactArc<[u8]>) -> Self {
        let mut bytes = Vec::with_capacity(1 + raw_f32_bytes.len());
        bytes.push(DataType::Vector as u8);
        bytes.extend_from_slice(&raw_f32_bytes);
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a DFP (Deterministic Floating Point) value from Dfp struct
    pub fn dfp(dfp: Dfp) -> Self {
        let encoding = DfpEncoding::from_dfp(&dfp).to_bytes();
        let mut bytes = Vec::with_capacity(1 + 24);
        bytes.push(DataType::DeterministicFloat as u8);
        bytes.extend_from_slice(&encoding);
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a DFP value from 24-byte encoding
    pub fn dfp_from_encoding(encoding: &[u8; 24]) -> Self {
        let mut bytes = Vec::with_capacity(1 + 24);
        bytes.push(DataType::DeterministicFloat as u8);
        bytes.extend_from_slice(encoding);
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a Quant (DQA) value from Dqa struct
    pub fn quant(dqa: Dqa) -> Self {
        let mut bytes = Vec::with_capacity(1 + 16);
        bytes.push(DataType::Quant as u8);
        // DqaEncoding is 16 bytes: value(i64) + scale(u8) + reserved[7]
        bytes.extend_from_slice(&dqa.value.to_be_bytes());
        bytes.push(dqa.scale);
        bytes.extend_from_slice(&[0u8; 7]); // reserved bytes
        Value::Extension(CompactArc::from(bytes))
    }

    /// Create a BIGINT value from BigInt struct
    /// Uses BigIntEncoding wire format: [version:1][sign:1][reserved:2][num_limbs:1][reserved:3][limb0:8]...[limbN:8]
    pub fn bigint(bi: BigInt) -> Self {
        let encoding = bi.serialize();
        let bytes = encoding.to_bytes();
        let mut buf = Vec::with_capacity(1 + bytes.len());
        buf.push(DataType::Bigint as u8);
        buf.extend_from_slice(&bytes);
        Value::Extension(CompactArc::from(buf))
    }

    /// Create a DECIMAL value from Decimal struct
    /// Uses 24-byte fixed format: [version:1][reserved:3][scale:1][mantissa:16] per decimal_to_bytes
    pub fn decimal(d: Decimal) -> Self {
        let bytes = decimal_to_bytes(&d);
        let mut buf = Vec::with_capacity(1 + 24);
        buf.push(DataType::Decimal as u8);
        buf.extend_from_slice(&bytes);
        Value::Extension(CompactArc::from(buf))
    }

    /// Create a Blob value from raw byte data
    pub fn blob(data: Vec<u8>) -> Self {
        Value::Blob(CompactArc::from(data))
    }

    // =========================================================================
    // Type accessors
    // =========================================================================

    /// Returns the data type of this value
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Null(dt) => *dt,
            Value::Integer(_) => DataType::Integer,
            Value::Float(_) => DataType::Float,
            Value::Text(_) => DataType::Text,
            Value::Boolean(_) => DataType::Boolean,
            Value::Timestamp(_) => DataType::Timestamp,
            Value::Extension(data) => data
                .first()
                .and_then(|&b| DataType::from_u8(b))
                .unwrap_or(DataType::Null),
            Value::Blob(_) => DataType::Blob,
        }
    }

    /// Returns true if this value is NULL
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null(_))
    }

    // =========================================================================
    // Value extractors
    // =========================================================================

    /// Extract as i64, with type coercion
    ///
    /// Returns None if:
    /// - Value is NULL
    /// - Conversion is not possible
    pub fn as_int64(&self) -> Option<i64> {
        match self {
            Value::Null(_) => None,
            Value::Integer(v) => Some(*v),
            Value::Float(v) => Some(*v as i64),
            Value::Text(s) => s
                .parse::<i64>()
                .ok()
                .or_else(|| s.parse::<f64>().ok().map(|f| f as i64)),
            Value::Boolean(b) => Some(if *b { 1 } else { 0 }),
            Value::Timestamp(t) => Some(t.timestamp_nanos_opt().unwrap_or(0)),
            Value::Extension(data) if data.first().copied() == Some(DataType::Bigint as u8) => {
                // BIGINT → i64: return None if value exceeds i64 range
                self.as_bigint().and_then(|bi| {
                    // Only single-limb values fit in i64
                    if bi.limbs().len() == 1 {
                        let limb = bi.limbs()[0];
                        let val = limb as i64;
                        // Check for overflow: if sign bit is set and value doesn't match
                        if bi.sign() {
                            if val == -(limb as i64) && val <= 0 {
                                Some(val)
                            } else {
                                None
                            }
                        } else {
                            if val == limb as i64 && val >= 0 {
                                Some(val)
                            } else {
                                None
                            }
                        }
                    } else {
                        None
                    }
                })
            }
            Value::Extension(data) if data.first().copied() == Some(DataType::Decimal as u8) => {
                // DECIMAL → i64: truncate fractional part
                self.as_decimal().and_then(|d| {
                    let scale = d.scale() as u32;
                    let truncated = d.mantissa() / 10i128.pow(scale);
                    i64::try_from(truncated).ok()
                })
            }
            Value::Extension(_) | Value::Blob(_) => None,
        }
    }

    /// Extract as f64, with type coercion
    pub fn as_float64(&self) -> Option<f64> {
        match self {
            Value::Null(_) => None,
            Value::Integer(v) => Some(*v as f64),
            Value::Float(v) => Some(*v),
            Value::Text(s) => s.parse::<f64>().ok(),
            Value::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Extension(data)
                if data.first() == Some(&(DataType::DeterministicFloat as u8)) =>
            {
                self.as_dfp().map(|d| d.to_f64())
            }
            Value::Extension(data) if data.first() == Some(&(DataType::Quant as u8)) => self
                .as_dqa()
                .map(|q| (q.value as f64) / 10f64.powi(q.scale as i32)),
            Value::Extension(data) if data.first() == Some(&(DataType::Decimal as u8)) => self
                .as_decimal()
                .map(|d| (d.mantissa() as f64) / 10f64.powi(d.scale() as i32)),
            Value::Extension(data) if data.first().copied() == Some(DataType::Bigint as u8) => {
                // BIGINT → f64: single-limb values only
                self.as_bigint().and_then(|bi| {
                    if bi.limbs().len() == 1 {
                        let limb = bi.limbs()[0];
                        Some(if bi.sign() { -(limb as f64) } else { limb as f64 })
                    } else {
                        None
                    }
                })
            }
            Value::Timestamp(_) | Value::Blob(_) => None,
            Value::Extension(_) => None,
        }
    }

    /// Extract as boolean, with type coercion
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Null(_) => None,
            Value::Integer(v) => Some(*v != 0),
            Value::Float(v) => Some(*v != 0.0),
            Value::Text(s) => {
                // OPTIMIZATION: Use eq_ignore_ascii_case to avoid allocation
                let s_ref: &str = s.as_ref();
                if s_ref.eq_ignore_ascii_case("true")
                    || s_ref.eq_ignore_ascii_case("t")
                    || s_ref.eq_ignore_ascii_case("yes")
                    || s_ref.eq_ignore_ascii_case("y")
                    || s_ref == "1"
                {
                    Some(true)
                } else if s_ref.eq_ignore_ascii_case("false")
                    || s_ref.eq_ignore_ascii_case("f")
                    || s_ref.eq_ignore_ascii_case("no")
                    || s_ref.eq_ignore_ascii_case("n")
                    || s_ref == "0"
                    || s_ref.is_empty()
                {
                    Some(false)
                } else {
                    s_ref.parse::<f64>().ok().map(|f| f != 0.0)
                }
            }
            Value::Boolean(b) => Some(*b),
            Value::Timestamp(_) | Value::Extension(_) | Value::Blob(_) => None,
        }
    }

    /// Extract as String, with type coercion
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::Null(_) => None,
            Value::Integer(v) => Some(v.to_string()),
            Value::Float(v) => Some(format_float(*v)),
            Value::Text(s) => Some(s.to_string()),
            Value::Boolean(b) => Some(if *b { "true" } else { "false" }.to_string()),
            Value::Timestamp(t) => Some(t.to_rfc3339()),
            Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                // SAFETY: Json data is always stored as valid UTF-8
                Some(std::str::from_utf8(&data[1..]).unwrap_or("").to_string())
            }
            Value::Extension(data) if data.first() == Some(&(DataType::Vector as u8)) => {
                Some(format_vector_bytes(&data[1..]))
            }
            Value::Extension(data)
                if data.first() == Some(&(DataType::DeterministicFloat as u8)) =>
            {
                self.as_dfp().map(|d| d.to_string())
            }
            Value::Extension(data) if data.first() == Some(&(DataType::Quant as u8)) => {
                self.as_dqa().map(format_dqa)
            }
            Value::Extension(data) if data.first() == Some(&(DataType::Bigint as u8)) => {
                self.as_bigint().map(|bi| bi.to_string())
            }
            Value::Extension(data) if data.first() == Some(&(DataType::Decimal as u8)) => {
                self.as_decimal().and_then(|d| decimal_to_string(&d).ok())
            }
            Value::Extension(data) => {
                // Generic fallback: try payload as UTF-8
                if data.len() > 1 {
                    std::str::from_utf8(&data[1..]).ok().map(|s| s.to_string())
                } else {
                    None
                }
            }
            Value::Blob(_) => None,
        }
    }

    /// Extract as string reference (avoids clone for Text/Json)
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s.as_str()),
            // SAFETY: Json data is always stored as valid UTF-8 (tag at [0], payload at [1..])
            Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                Some(std::str::from_utf8(&data[1..]).unwrap_or(""))
            }
            _ => None,
        }
    }

    /// Extract as DateTime<Utc>
    pub fn as_timestamp(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::Null(_) => None,
            Value::Timestamp(t) => Some(*t),
            Value::Text(s) => parse_timestamp(s).ok(),
            Value::Integer(nanos) => {
                // Interpret as nanoseconds since Unix epoch
                DateTime::from_timestamp(*nanos / 1_000_000_000, (*nanos % 1_000_000_000) as u32)
            }
            _ => None,
        }
    }

    /// Extract as JSON string
    pub fn as_json(&self) -> Option<&str> {
        match self {
            Value::Null(_) => Some("{}"),
            // SAFETY: Json data is always stored as valid UTF-8 (tag at [0], payload at [1..])
            Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                Some(std::str::from_utf8(&data[1..]).unwrap_or(""))
            }
            _ => None,
        }
    }

    /// Extract vector as Vec<f32> (reads packed LE f32 bytes from Extension payload)
    pub fn as_vector_f32(&self) -> Option<Vec<f32>> {
        match self {
            Value::Extension(data) if data.first() == Some(&(DataType::Vector as u8)) => {
                let payload = &data[1..];
                let len = payload.len() / 4;
                let mut result = Vec::with_capacity(len);
                for i in 0..len {
                    let bytes = [
                        payload[i * 4],
                        payload[i * 4 + 1],
                        payload[i * 4 + 2],
                        payload[i * 4 + 3],
                    ];
                    result.push(f32::from_le_bytes(bytes));
                }
                Some(result)
            }
            _ => None,
        }
    }

    /// Extract DFP as Dfp struct (decodes 24-byte DfpEncoding from Extension payload)
    pub fn as_dfp(&self) -> Option<Dfp> {
        match self {
            Value::Extension(data)
                if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
            {
                extract_dfp_from_extension(data)
            }
            _ => None,
        }
    }

    /// Extract DQA from Extension payload
    pub fn as_dqa(&self) -> Option<Dqa> {
        match self {
            Value::Extension(data) if data.first().copied() == Some(DataType::Quant as u8) => {
                if data.len() < 10 {
                    return None;
                }
                let value = i64::from_be_bytes(data[1..9].try_into().ok()?);
                let scale = data[9];
                Dqa::new(value, scale).ok()
            }
            _ => None,
        }
    }

    /// Extract BIGINT from Extension payload (variable-length limb array)
    pub fn as_bigint(&self) -> Option<BigInt> {
        match self {
            Value::Extension(data) if data.first().copied() == Some(DataType::Bigint as u8) => {
                // Skip tag byte, pass variable-length limb data
                BigInt::deserialize(&data[1..]).ok()
            }
            _ => None,
        }
    }

    /// Extract DECIMAL from Extension payload (fixed 24-byte format)
    /// Format: [mantissa:16][reserved:7][scale:1] (24 bytes total) per decimal_to_bytes
    /// We parse directly to bypass decimal_from_bytes validation which has a bug
    /// rejecting some canonical values (e.g., mantissa=1, scale=0).
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Extension(data) if data.first().copied() == Some(DataType::Decimal as u8) => {
                // Need at least 1 (tag) + 24 (decimal encoding) = 25 bytes
                if data.len() < 25 {
                    return None;
                }
                // Parse 24-byte decimal encoding directly:
                // Extension layout: [tag:0x0e][version:1][reserved:3][scale:1][mantissa:16]
                // 24-byte encoding starts at data[1]:
                //   version = data[1], reserved = data[2-4], scale = data[5], mantissa = data[9..25]
                let scale = data[5];
                let mantissa_bytes: [u8; 16] = data[9..25].try_into().ok()?;
                let mantissa = i128::from_be_bytes(mantissa_bytes);
                Decimal::new(mantissa, scale).ok()
            }
            _ => None,
        }
    }

    /// Extract blob as byte slice
    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Value::Blob(data) => Some(data),
            _ => None,
        }
    }

    /// Extract blob as 32-byte array (for SHA256 key_hash values)
    ///
    /// Returns `None` if the value is not a Blob or is not exactly 32 bytes.
    pub fn as_blob_32(&self) -> Option<[u8; 32]> {
        match self {
            Value::Blob(data) if data.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(data);
                Some(arr)
            }
            _ => None,
        }
    }

    /// Convert value to DFP (deterministic floating-point)
    pub fn to_dfp(&self) -> Option<Dfp> {
        match self {
            Value::Integer(i) => Some(Dfp::from_i64(*i)),
            Value::Float(f) => Some(Dfp::from_f64(*f)),
            Value::Extension(data)
                if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
            {
                self.as_dfp()
            }
            _ => None,
        }
    }

    /// Convert value to DFP or return default
    pub fn coerce_to_dfp(&self) -> Dfp {
        self.to_dfp().unwrap_or(Dfp::nan())
    }

    // =========================================================================
    // Comparison
    // =========================================================================

    /// Compare two values for ordering
    ///
    /// Returns:
    /// - Ok(Ordering::Less) if self < other
    /// - Ok(Ordering::Equal) if self == other
    /// - Ok(Ordering::Greater) if self > other
    /// - Err if comparison is not possible
    pub fn compare(&self, other: &Value) -> Result<Ordering> {
        // Handle NULL comparisons
        if self.is_null() || other.is_null() {
            if self.is_null() && other.is_null() {
                return Ok(Ordering::Equal);
            }
            return Err(Error::NullComparison);
        }

        // Same type comparison (most efficient path)
        if self.data_type() == other.data_type() {
            return self.compare_same_type(other);
        }

        // Cross-type numeric comparison (integer vs float vs DFP vs DQA)
        if self.data_type().is_numeric() && other.data_type().is_numeric() {
            // Convert to f64 for comparison
            let v1 = self.as_float64().unwrap();
            let v2 = other.as_float64().unwrap();
            return Ok(compare_floats(v1, v2));
        }

        // Timestamp ↔ Text: try parsing the text side as a timestamp
        match (self, other) {
            (Value::Timestamp(ts), Value::Text(s)) => {
                if let Ok(parsed) = parse_timestamp(s) {
                    return Ok(ts.cmp(&parsed));
                }
            }
            (Value::Timestamp(ts), Value::Extension(data))
                if data.first() == Some(&(DataType::Json as u8)) =>
            {
                // SAFETY: Json data is always valid UTF-8
                let s = std::str::from_utf8(&data[1..]).unwrap_or("");
                if let Ok(parsed) = parse_timestamp(s) {
                    return Ok(ts.cmp(&parsed));
                }
            }
            (Value::Text(s), Value::Timestamp(ts)) => {
                if let Ok(parsed) = parse_timestamp(s) {
                    return Ok(parsed.cmp(ts));
                }
            }
            (Value::Extension(data), Value::Timestamp(ts))
                if data.first() == Some(&(DataType::Json as u8)) =>
            {
                // SAFETY: Json data is always valid UTF-8
                let s = std::str::from_utf8(&data[1..]).unwrap_or("");
                if let Ok(parsed) = parse_timestamp(s) {
                    return Ok(parsed.cmp(ts));
                }
            }
            _ => {}
        }

        // Fall back to string comparison for mixed types
        let s1 = self.as_string().unwrap_or_default();
        let s2 = other.as_string().unwrap_or_default();
        Ok(s1.cmp(&s2))
    }

    /// Compare values of the same type
    fn compare_same_type(&self, other: &Value) -> Result<Ordering> {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => Ok(compare_floats(*a, *b)),
            (Value::Text(a), Value::Text(b)) => Ok(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Ok(a.cmp(b)),
            (Value::Timestamp(a), Value::Timestamp(b)) => Ok(a.cmp(b)),
            (Value::Blob(a), Value::Blob(b)) => {
                // Blob comparison: byte-by-byte until difference, then by length
                match a.as_ref().cmp(b.as_ref()) {
                    Ordering::Equal => Ok(a.len().cmp(&b.len())),
                    ord => Ok(ord),
                }
            }
            (Value::Extension(a), Value::Extension(b)) => {
                if a.first() != b.first() {
                    return Err(Error::IncomparableTypes);
                }
                let tag = a.first().copied().unwrap_or(0);
                match tag {
                    t if t == DataType::DeterministicFloat as u8 => {
                        let da = extract_dfp_from_extension(a).ok_or(Error::Internal {
                            message: "invalid dfp data".into(),
                        })?;
                        let db = extract_dfp_from_extension(b).ok_or(Error::Internal {
                            message: "invalid dfp data".into(),
                        })?;
                        Ok(compare_dfp(&da, &db))
                    }
                    t if t == DataType::Quant as u8 => {
                        let da = self.as_dqa().ok_or(Error::Internal {
                            message: "invalid dqa data".into(),
                        })?;
                        let db = other.as_dqa().ok_or(Error::Internal {
                            message: "invalid dqa data".into(),
                        })?;
                        Ok(match dqa_cmp(da, db) {
                            -1 => Ordering::Less,
                            0 => Ordering::Equal,
                            1 => Ordering::Greater,
                            n => {
                                debug_assert!(false, "invalid dqa comparison result: {}", n);
                                Ordering::Greater
                            }
                        })
                    }
                    t if t == DataType::Bigint as u8 => {
                        let ba = self.as_bigint().ok_or(Error::Internal {
                            message: "invalid bigint data".into(),
                        })?;
                        let bb = other.as_bigint().ok_or(Error::Internal {
                            message: "invalid bigint data".into(),
                        })?;
                        Ok(match ba.compare(&bb) {
                            -1 => Ordering::Less,
                            0 => Ordering::Equal,
                            1 => Ordering::Greater,
                            n => {
                                debug_assert!(false, "invalid bigint comparison result: {}", n);
                                Ordering::Greater
                            }
                        })
                    }
                    t if t == DataType::Decimal as u8 => {
                        let da = self.as_decimal().ok_or(Error::Internal {
                            message: "invalid decimal data".into(),
                        })?;
                        let db = other.as_decimal().ok_or(Error::Internal {
                            message: "invalid decimal data".into(),
                        })?;
                        Ok(match decimal_cmp(&da, &db) {
                            -1 => Ordering::Less,
                            0 => Ordering::Equal,
                            1 => Ordering::Greater,
                            n => {
                                debug_assert!(false, "invalid decimal comparison result: {}", n);
                                Ordering::Greater
                            }
                        })
                    }
                    _ => {
                        // Other extension types: equality only
                        if a == b {
                            Ok(Ordering::Equal)
                        } else {
                            Err(Error::IncomparableTypes)
                        }
                    }
                }
            }
            _ => Err(Error::IncomparableTypes),
        }
    }

    // =========================================================================
    // Construction from typed values
    // =========================================================================

    /// Create a Value from a typed value with explicit data type
    pub fn from_typed(value: Option<&dyn std::any::Any>, data_type: DataType) -> Self {
        match value {
            None => Value::Null(data_type),
            Some(v) => {
                // Try to downcast based on expected type
                match data_type {
                    DataType::Integer => {
                        if let Some(&i) = v.downcast_ref::<i64>() {
                            Value::Integer(i)
                        } else if let Some(&i) = v.downcast_ref::<i32>() {
                            Value::Integer(i as i64)
                        } else if let Some(s) = v.downcast_ref::<String>() {
                            s.parse::<i64>()
                                .map(Value::Integer)
                                .unwrap_or(Value::Null(data_type))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Float => {
                        if let Some(&f) = v.downcast_ref::<f64>() {
                            Value::Float(f)
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Value::Float(i as f64)
                        } else if let Some(s) = v.downcast_ref::<String>() {
                            s.parse::<f64>()
                                .map(Value::Float)
                                .unwrap_or(Value::Null(data_type))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Text => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            Value::Text(SmartString::new(s))
                        } else if let Some(&s) = v.downcast_ref::<&str>() {
                            Value::Text(SmartString::from(s))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Boolean => {
                        if let Some(&b) = v.downcast_ref::<bool>() {
                            Value::Boolean(b)
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Value::Boolean(i != 0)
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Timestamp => {
                        if let Some(&t) = v.downcast_ref::<DateTime<Utc>>() {
                            Value::Timestamp(t)
                        } else if let Some(s) = v.downcast_ref::<String>() {
                            parse_timestamp(s)
                                .map(Value::Timestamp)
                                .unwrap_or(Value::Null(data_type))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Json => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            // Validate JSON
                            if serde_json::from_str::<serde_json::Value>(s).is_ok() {
                                Value::json(s)
                            } else {
                                Value::Null(data_type)
                            }
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Vector => {
                        if let Some(vec) = v.downcast_ref::<Vec<f32>>() {
                            Value::vector(vec.clone())
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::DeterministicFloat => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            s.parse::<f64>()
                                .map(|f| Value::dfp(Dfp::from_f64(f)))
                                .unwrap_or(Value::Null(data_type))
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Value::dfp(Dfp::from_i64(i))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Quant => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            parse_string_to_dqa(s)
                                .map(Value::quant)
                                .unwrap_or(Value::Null(data_type))
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Dqa::new(i, 0)
                                .map(Value::quant)
                                .unwrap_or(Value::Null(data_type))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Blob => {
                        // Blob support - downcast from Vec<u8>
                        if let Some(vec) = v.downcast_ref::<Vec<u8>>() {
                            Value::blob(vec.clone())
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Bigint => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            // String → BigInt parsing; returns NULL on invalid input
                            BigInt::from_str(s)
                                .map(|bi| Value::Extension(bi.serialize().to_bytes().into()))
                                .unwrap_or(Value::Null(data_type))
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Value::Extension(BigInt::from(i).serialize().to_bytes().into())
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Decimal => {
                        if let Some(s) = v.downcast_ref::<String>() {
                            // String → Decimal parsing; try i128 first (no fractional part)
                            s.parse::<i128>()
                                .ok()
                                .and_then(|i| Decimal::new(i, 0).ok())
                                .map(|d| Value::Extension(decimal_to_bytes(&d).to_vec().into()))
                                .unwrap_or(Value::Null(data_type))
                        } else if let Some(&i) = v.downcast_ref::<i64>() {
                            Decimal::new(i.into(), 0)
                                .map(|d| Value::Extension(decimal_to_bytes(&d).to_vec().into()))
                                .unwrap_or(Value::Null(data_type))
                        } else {
                            Value::Null(data_type)
                        }
                    }
                    DataType::Null => Value::Null(DataType::Null),
                }
            }
        }
    }

    // =========================================================================
    // Type coercion
    // =========================================================================

    /// Coerce this value to the target data type
    ///
    /// Type coercion rules:
    /// - Integer column receiving Float → converts to Integer
    /// - Float column receiving Integer → converts to Float
    /// - Text column receiving any type → converts to Text
    /// - Timestamp column receiving String → parses timestamp
    /// - JSON column receiving valid JSON string → stores as JSON
    /// - Boolean column receiving Integer/String → converts to Boolean
    ///
    /// Returns the coerced value, or NULL if coercion fails.
    pub fn coerce_to_type(&self, target_type: DataType) -> Value {
        self.cast_to_type(target_type)
    }

    /// Explicit cast - allows all conversions including FLOAT→DFP
    pub fn cast_to_type(&self, target_type: DataType) -> Value {
        // NULL stays NULL (with target type hint)
        if self.is_null() {
            return Value::Null(target_type);
        }

        // Same type - no conversion needed
        if self.data_type() == target_type {
            return self.clone();
        }

        match target_type {
            DataType::Integer => {
                // Convert to INTEGER
                match self {
                    Value::Integer(v) => Value::Integer(*v),
                    Value::Float(v) => Value::Integer(*v as i64),
                    Value::Text(s) => s
                        .parse::<i64>()
                        .map(Value::Integer)
                        .unwrap_or(Value::Null(target_type)),
                    Value::Boolean(b) => Value::Integer(if *b { 1 } else { 0 }),
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                    {
                        // DFP -> Integer (truncate)
                        if let Some(dfp) = self.as_dfp() {
                            Value::Integer(dfp.to_f64() as i64)
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    _ => {
                        // Try as_int64 for BIGINT, DECIMAL, etc.
                        self.as_int64()
                            .map(Value::Integer)
                            .unwrap_or(Value::Null(target_type))
                    }
                }
            }
            DataType::Float => {
                // Convert to FLOAT
                match self {
                    Value::Float(v) => Value::Float(*v),
                    Value::Integer(v) => Value::Float(*v as f64),
                    Value::Text(s) => s
                        .parse::<f64>()
                        .map(Value::Float)
                        .unwrap_or(Value::Null(target_type)),
                    Value::Boolean(b) => Value::Float(if *b { 1.0 } else { 0.0 }),
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                    {
                        // DFP -> Float
                        if let Some(dfp) = self.as_dfp() {
                            Value::Float(dfp.to_f64())
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    _ => {
                        // Try as_float64 for BIGINT, DECIMAL, etc.
                        self.as_float64()
                            .map(Value::Float)
                            .unwrap_or(Value::Null(target_type))
                    }
                }
            }
            DataType::DeterministicFloat => {
                // Convert to DFP (Deterministic Floating Point)
                match self {
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                    {
                        // Already DFP
                        self.clone()
                    }
                    Value::Integer(v) => Value::dfp(Dfp::from_i64(*v)),
                    Value::Float(v) => Value::dfp(Dfp::from_f64(*v)),
                    Value::Text(s) => {
                        // Try to parse as f64 first, then convert to DFP
                        s.parse::<f64>()
                            .map(|f| Value::dfp(Dfp::from_f64(f)))
                            .unwrap_or(Value::Null(target_type))
                    }
                    Value::Boolean(b) => Value::dfp(Dfp::from_f64(if *b { 1.0 } else { 0.0 })),
                    _ => Value::Null(target_type),
                }
            }
            DataType::Text => {
                // Convert to TEXT - everything can become text
                match self {
                    Value::Text(s) => Value::Text(s.clone()),
                    Value::Integer(v) => Value::Text(SmartString::from_string(v.to_string())),
                    Value::Float(v) => Value::Text(SmartString::from_string(format_float(*v))),
                    Value::Boolean(b) => {
                        Value::Text(SmartString::new(if *b { "true" } else { "false" }))
                    }
                    Value::Timestamp(t) => Value::Text(SmartString::from_string(t.to_rfc3339())),
                    Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                        Value::Text(SmartString::new(
                            std::str::from_utf8(&data[1..]).unwrap_or(""),
                        ))
                    }
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                    {
                        // DFP -> Text
                        if let Some(dfp) = self.as_dfp() {
                            Value::Text(SmartString::from_string(dfp.to_string()))
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    Value::Extension(data) if data.first() == Some(&(DataType::Vector as u8)) => {
                        Value::Text(SmartString::from_string(format_vector_bytes(&data[1..])))
                    }
                    Value::Extension(_) | Value::Null(_) | Value::Blob(_) => {
                        Value::Null(target_type)
                    }
                }
            }
            DataType::Boolean => {
                // Convert to BOOLEAN
                match self {
                    Value::Boolean(b) => Value::Boolean(*b),
                    Value::Integer(v) => Value::Boolean(*v != 0),
                    Value::Float(v) => Value::Boolean(*v != 0.0),
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                    {
                        // DFP -> Boolean (true if not zero)
                        if let Some(dfp) = self.as_dfp() {
                            Value::Boolean(dfp.to_f64() != 0.0)
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    Value::Text(s) => {
                        // OPTIMIZATION: Use eq_ignore_ascii_case to avoid allocation
                        let s_ref: &str = s.as_ref();
                        if s_ref.eq_ignore_ascii_case("true")
                            || s_ref.eq_ignore_ascii_case("t")
                            || s_ref.eq_ignore_ascii_case("yes")
                            || s_ref.eq_ignore_ascii_case("y")
                            || s_ref == "1"
                        {
                            Value::Boolean(true)
                        } else if s_ref.eq_ignore_ascii_case("false")
                            || s_ref.eq_ignore_ascii_case("f")
                            || s_ref.eq_ignore_ascii_case("no")
                            || s_ref.eq_ignore_ascii_case("n")
                            || s_ref == "0"
                        {
                            Value::Boolean(false)
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Timestamp => {
                // Convert to TIMESTAMP
                match self {
                    Value::Timestamp(t) => Value::Timestamp(*t),
                    Value::Text(s) => parse_timestamp(s)
                        .map(Value::Timestamp)
                        .unwrap_or(Value::Null(target_type)),
                    Value::Integer(nanos) => {
                        // Interpret as nanoseconds since Unix epoch
                        DateTime::from_timestamp(
                            *nanos / 1_000_000_000,
                            (*nanos % 1_000_000_000) as u32,
                        )
                        .map(Value::Timestamp)
                        .unwrap_or(Value::Null(target_type))
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Quant => {
                // Convert to DQA (Deterministic Quant Arithmetic)
                match self {
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::Quant as u8) =>
                    {
                        // Already Quant
                        self.clone()
                    }
                    Value::Integer(v) => Dqa::new(*v, 0)
                        .map(Value::quant)
                        .unwrap_or(Value::Null(target_type)),
                    Value::Float(v) => {
                        // Convert via string to preserve decimal precision
                        let s = format!("{}", v);
                        parse_string_to_dqa(&s)
                            .map(Value::quant)
                            .unwrap_or(Value::Null(target_type))
                    }
                    Value::Text(s) => parse_string_to_dqa(s.as_ref())
                        .map(Value::quant)
                        .unwrap_or(Value::Null(target_type)),
                    Value::Boolean(b) => Dqa::new(if *b { 1 } else { 0 }, 0)
                        .map(Value::quant)
                        .unwrap_or(Value::Null(target_type)),
                    _ => Value::Null(target_type),
                }
            }
            DataType::Json => {
                // Convert to JSON
                match self {
                    Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                        self.clone()
                    }
                    Value::Text(s) => {
                        // Validate JSON
                        if serde_json::from_str::<serde_json::Value>(s.as_str()).is_ok() {
                            Value::json(s.as_str())
                        } else {
                            Value::Null(target_type)
                        }
                    }
                    // Convert other types to JSON representation
                    Value::Integer(v) => Value::json(v.to_string()),
                    Value::Float(v) => Value::json(format_float(*v)),
                    Value::Boolean(b) => Value::json(if *b { "true" } else { "false" }),
                    _ => Value::Null(target_type),
                }
            }
            DataType::Vector => match self {
                Value::Extension(data) if data.first() == Some(&(DataType::Vector as u8)) => {
                    self.clone()
                }
                Value::Text(s) => {
                    if let Some(floats) = parse_vector_str(s.as_str()) {
                        Value::vector(floats)
                    } else {
                        Value::Null(target_type)
                    }
                }
                _ => Value::Null(target_type),
            },
            DataType::Blob => match self {
                Value::Extension(data) if data.first().copied() == Some(DataType::Blob as u8) => {
                    self.clone()
                }
                _ => Value::Null(target_type),
            },
            DataType::Bigint => {
                // BIGINT target type
                match self {
                    Value::Integer(i) => Value::bigint(BigInt::from(*i)),
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::Bigint as u8) =>
                    {
                        // Already BIGINT
                        self.clone()
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Decimal => {
                // DECIMAL target type
                match self {
                    Value::Integer(i) => {
                        // INTEGER→DECIMAL shortcut: Decimal::new(i128, 0)
                        Decimal::new((*i).into(), 0)
                            .map(Value::decimal)
                            .unwrap_or(Value::Null(target_type))
                    }
                    Value::Extension(data)
                        if data.first().copied() == Some(DataType::Decimal as u8) =>
                    {
                        // Already DECIMAL
                        self.clone()
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Null => Value::Null(DataType::Null),
        }
    }

    /// Coerce value to target type, consuming self
    /// OPTIMIZATION: Avoids clone when types already match
    #[inline]
    pub fn into_coerce_to_type(self, target_type: DataType) -> Value {
        // NULL stays NULL (with target type hint)
        if self.is_null() {
            return Value::Null(target_type);
        }

        // Same type - no conversion needed, return self directly
        if self.data_type() == target_type {
            return self;
        }

        match target_type {
            DataType::Integer => match &self {
                Value::Integer(v) => Value::Integer(*v),
                Value::Float(v) => Value::Integer(*v as i64),
                Value::Text(s) => s
                    .parse::<i64>()
                    .map(Value::Integer)
                    .unwrap_or(Value::Null(target_type)),
                Value::Boolean(b) => Value::Integer(if *b { 1 } else { 0 }),
                _ => Value::Null(target_type),
            },
            DataType::Float => match &self {
                Value::Float(v) => Value::Float(*v),
                Value::Integer(v) => Value::Float(*v as f64),
                Value::Text(s) => s
                    .parse::<f64>()
                    .map(Value::Float)
                    .unwrap_or(Value::Null(target_type)),
                Value::Boolean(b) => Value::Float(if *b { 1.0 } else { 0.0 }),
                _ => Value::Null(target_type),
            },
            DataType::Text => match self {
                Value::Text(s) => Value::Text(s),
                Value::Integer(v) => Value::Text(SmartString::from_string(v.to_string())),
                Value::Float(v) => Value::Text(SmartString::from_string(format_float(v))),
                Value::Boolean(b) => {
                    Value::Text(SmartString::new(if b { "true" } else { "false" }))
                }
                Value::Timestamp(t) => Value::Text(SmartString::from_string(t.to_rfc3339())),
                Value::Extension(data) if data.first() == Some(&(DataType::Json as u8)) => {
                    Value::Text(SmartString::new(
                        std::str::from_utf8(&data[1..]).unwrap_or(""),
                    ))
                }
                Value::Extension(data) if data.first() == Some(&(DataType::Vector as u8)) => {
                    Value::Text(SmartString::from_string(format_vector_bytes(&data[1..])))
                }
                Value::Extension(_) | Value::Null(_) | Value::Blob(_) => Value::Null(target_type),
            },
            DataType::Boolean => match &self {
                Value::Boolean(b) => Value::Boolean(*b),
                Value::Integer(v) => Value::Boolean(*v != 0),
                Value::Float(v) => Value::Boolean(*v != 0.0),
                Value::Text(s) => {
                    // OPTIMIZATION: Use eq_ignore_ascii_case to avoid allocation
                    let s_ref: &str = s.as_ref();
                    if s_ref.eq_ignore_ascii_case("true")
                        || s_ref.eq_ignore_ascii_case("t")
                        || s_ref.eq_ignore_ascii_case("yes")
                        || s_ref.eq_ignore_ascii_case("y")
                        || s_ref == "1"
                    {
                        Value::Boolean(true)
                    } else if s_ref.eq_ignore_ascii_case("false")
                        || s_ref.eq_ignore_ascii_case("f")
                        || s_ref.eq_ignore_ascii_case("no")
                        || s_ref.eq_ignore_ascii_case("n")
                        || s_ref == "0"
                    {
                        Value::Boolean(false)
                    } else {
                        Value::Null(target_type)
                    }
                }
                _ => Value::Null(target_type),
            },
            DataType::Timestamp => match self {
                Value::Timestamp(t) => Value::Timestamp(t),
                Value::Text(s) => parse_timestamp(&s)
                    .map(Value::Timestamp)
                    .unwrap_or(Value::Null(target_type)),
                Value::Integer(nanos) => {
                    DateTime::from_timestamp(nanos / 1_000_000_000, (nanos % 1_000_000_000) as u32)
                        .map(Value::Timestamp)
                        .unwrap_or(Value::Null(target_type))
                }
                _ => Value::Null(target_type),
            },
            DataType::Json => match self {
                Value::Extension(ref data) if data.first() == Some(&(DataType::Json as u8)) => self,
                Value::Text(s) => {
                    if serde_json::from_str::<serde_json::Value>(s.as_str()).is_ok() {
                        Value::json(s.as_str())
                    } else {
                        Value::Null(target_type)
                    }
                }
                Value::Integer(v) => Value::json(v.to_string()),
                Value::Float(v) => Value::json(format_float(v)),
                Value::Boolean(b) => Value::json(if b { "true" } else { "false" }),
                _ => Value::Null(target_type),
            },
            DataType::Vector => match self {
                Value::Extension(ref data) if data.first() == Some(&(DataType::Vector as u8)) => {
                    self
                }
                Value::Text(s) => {
                    if let Some(floats) = parse_vector_str(s.as_str()) {
                        Value::vector(floats)
                    } else {
                        Value::Null(target_type)
                    }
                }
                _ => Value::Null(target_type),
            },
            DataType::Quant => match &self {
                Value::Extension(data) if data.first().copied() == Some(DataType::Quant as u8) => {
                    self
                }
                Value::Integer(v) => Dqa::new(*v, 0)
                    .map(Value::quant)
                    .unwrap_or(Value::Null(target_type)),
                Value::Float(v) => {
                    let s = format!("{}", v);
                    parse_string_to_dqa(&s)
                        .map(Value::quant)
                        .unwrap_or(Value::Null(target_type))
                }
                Value::Text(s) => parse_string_to_dqa(s.as_ref())
                    .map(Value::quant)
                    .unwrap_or(Value::Null(target_type)),
                _ => Value::Null(target_type),
            },
            DataType::DeterministicFloat => match &self {
                Value::Extension(data)
                    if data.first().copied() == Some(DataType::DeterministicFloat as u8) =>
                {
                    self
                }
                Value::Integer(v) => Value::dfp(Dfp::from_i64(*v)),
                Value::Float(v) => Value::dfp(Dfp::from_f64(*v)),
                Value::Text(s) => s
                    .parse::<f64>()
                    .map(|f| Value::dfp(Dfp::from_f64(f)))
                    .unwrap_or(Value::Null(target_type)),
                _ => Value::Null(target_type),
            },
            DataType::Blob => match self {
                Value::Extension(ref data) if data.first() == Some(&(DataType::Blob as u8)) => self,
                _ => Value::Null(target_type),
            },
            DataType::Bigint => {
                // BIGINT target type
                match self {
                    Value::Integer(i) => Value::bigint(BigInt::from(i)),
                    Value::Extension(ref data)
                        if data.first().copied() == Some(DataType::Bigint as u8) =>
                    {
                        // Already BIGINT
                        self
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Decimal => {
                // DECIMAL target type
                match self {
                    Value::Integer(i) => {
                        // INTEGER→DECIMAL shortcut: Decimal::new(i128, 0)
                        Decimal::new(i.into(), 0)
                            .map(Value::decimal)
                            .unwrap_or(Value::Null(target_type))
                    }
                    Value::Extension(ref data)
                        if data.first().copied() == Some(DataType::Decimal as u8) =>
                    {
                        // Already DECIMAL
                        self
                    }
                    _ => Value::Null(target_type),
                }
            }
            DataType::Null => Value::Null(DataType::Null),
        }
    }
}

// =========================================================================
// Trait implementations
// =========================================================================

impl Default for Value {
    fn default() -> Self {
        Value::Null(DataType::Null)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null(_) => write!(f, "NULL"),
            Value::Integer(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", format_float(*v)),
            Value::Text(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Timestamp(t) => write!(f, "{}", t.to_rfc3339()),
            Value::Extension(data) => {
                let tag = data.first().copied().unwrap_or(0);
                if tag == DataType::Json as u8 {
                    write!(f, "{}", std::str::from_utf8(&data[1..]).unwrap_or(""))
                } else if tag == DataType::Vector as u8 {
                    write!(f, "{}", format_vector_bytes(&data[1..]))
                } else if tag == DataType::DeterministicFloat as u8 {
                    if let Some(dfp) = self.as_dfp() {
                        write!(f, "{}", dfp.to_string())
                    } else {
                        write!(f, "<invalid DFP>")
                    }
                } else if tag == DataType::Quant as u8 {
                    if let Some(dqa) = self.as_dqa() {
                        write!(f, "{}", format_dqa(dqa))
                    } else {
                        write!(f, "<invalid DQA>")
                    }
                } else if tag == DataType::Bigint as u8 {
                    if let Some(bi) = self.as_bigint() {
                        write!(f, "{}", bi)
                    } else {
                        write!(f, "<invalid BIGINT>")
                    }
                } else if tag == DataType::Decimal as u8 {
                    if let Some(d) = self.as_decimal() {
                        decimal_to_string(&d)
                            .map(|s| write!(f, "{}", s))
                            .unwrap_or_else(|_| write!(f, "<invalid DECIMAL>"))
                    } else {
                        write!(f, "<invalid DECIMAL>")
                    }
                } else {
                    write!(f, "<extension:{}>", tag)
                }
            }
            Value::Blob(data) => {
                // Show hex preview of first 8 bytes
                let preview = &data[..data.len().min(8)];
                write!(f, "Blob({:02x?}", preview)
                    .and_then(|_| {
                        if data.len() > 8 {
                            write!(f, "...")
                        } else {
                            Ok(())
                        }
                    })
                    .and_then(|_| write!(f, ")"))
            }
        }
    }
}

impl PartialEq for Value {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Single match handles NULL and all type comparisons without redundant is_null() calls
        match (self, other) {
            // NULL handling: NULL == NULL (SQL equality semantics for grouping)
            (Value::Null(_), Value::Null(_)) => true,
            // NULL != any non-NULL value
            (Value::Null(_), _) | (_, Value::Null(_)) => false,
            // Same type comparisons
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                // Handle NaN: NaN != NaN in IEEE 754, but we consider them equal
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            // Cross-type numeric comparison: Integer vs Float
            // This is critical for queries like WHERE id = 5.0 or WHERE price = 100
            (Value::Integer(i), Value::Float(f)) | (Value::Float(f), Value::Integer(i)) => {
                *f == (*i as f64)
            }
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Timestamp(a), Value::Timestamp(b)) => a == b,
            (Value::Extension(a), Value::Extension(b)) => a == b,
            (Value::Blob(a), Value::Blob(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

/// Maximum i64 value that can be safely hashed as i64 without f64 conversion.
/// Integers in the range [I64_SAFE_MIN, I64_SAFE_MAX] have unique f64 representations,
/// while integers outside this range may round to the same f64 value.
/// This is 2^53 - 1 = 9007199254740991.
const I64_SAFE_MAX: i64 = (1_i64 << 53) - 1;
const I64_SAFE_MIN: i64 = -I64_SAFE_MAX;

/// WyHash-style 128-bit multiply mixing function.
/// Provides excellent avalanche properties - small input changes produce
/// completely different outputs. This pre-mixes values before the hasher
/// sees them, fixing collision problems with simple hashers like FxHash.
#[inline(always)]
fn wymix(a: u64, b: u64) -> u64 {
    let r = (a as u128).wrapping_mul(b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

// WyHash prime constants for mixing
const WY_P1: u64 = 0xa0761d6478bd642f;
const WY_P2: u64 = 0xe7037ed1a0b428db;

impl Hash for Value {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Pre-mix strategy: Instead of writing raw values that may have poor
        // distribution (causing collisions in simple hashers like FxHash),
        // we pre-mix everything using WyHash-style 128-bit multiply mixing.
        // This gives ANY hasher well-distributed inputs.
        //
        // Constraint: Integer(5) == Float(5.0) must have equal hashes.
        // We handle this by using the same mixing for whole-number floats.
        match self {
            Value::Null(_) => {
                // All NULLs hash the same
                state.write_u64(0);
            }
            Value::Integer(v) => {
                // Pre-mix integer with discriminant
                match *v {
                    I64_SAFE_MIN..=I64_SAFE_MAX => {
                        state.write_u64(wymix(1 ^ (*v as u64), WY_P1));
                    }
                    _ => {
                        // Large integer: use f64 bits for consistency with Float
                        state.write_u64(wymix(1 ^ (*v as f64).to_bits(), WY_P1));
                    }
                }
            }
            Value::Float(v) => {
                if v.is_nan() {
                    // All NaNs are equal, so they must hash the same
                    state.write_u64(wymix(6 ^ f64::NAN.to_bits(), WY_P1));
                } else if v.fract() == 0.0 {
                    // Whole number float - must hash same as equivalent Integer
                    match *v as i64 {
                        i @ I64_SAFE_MIN..=I64_SAFE_MAX => {
                            state.write_u64(wymix(1 ^ (i as u64), WY_P1));
                        }
                        _ => {
                            state.write_u64(wymix(1 ^ v.to_bits(), WY_P1));
                        }
                    }
                } else {
                    // Fractional float - use discriminant 6 (can't equal any Integer)
                    state.write_u64(wymix(6 ^ v.to_bits(), WY_P1));
                }
            }
            Value::Text(s) => {
                // Pre-hash string with WyHash-style mixing, write single u64
                let bytes = s.as_bytes();
                let len = bytes.len();
                let mut h = wymix(2 ^ (len as u64), WY_P1);

                // Process 8 bytes at a time
                let chunks = len / 8;
                let ptr = bytes.as_ptr();
                for i in 0..chunks {
                    // SAFETY: We iterate i from 0..chunks where chunks = len/8.
                    // So i*8 is always < len, and we read 8 bytes which is valid
                    // since (i+1)*8 <= chunks*8 <= len. read_unaligned handles alignment.
                    let chunk = unsafe { (ptr.add(i * 8) as *const u64).read_unaligned() };
                    h = wymix(h ^ chunk, WY_P2);
                }

                // Handle tail bytes (0-7)
                let tail_start = chunks * 8;
                if tail_start < len {
                    let mut tail = 0u64;
                    for (j, &b) in bytes[tail_start..].iter().enumerate() {
                        tail |= (b as u64) << (j * 8);
                    }
                    h = wymix(h ^ tail, WY_P1);
                }

                state.write_u64(h);
            }
            Value::Boolean(b) => {
                // Pre-mixed boolean
                state.write_u64(wymix(if *b { 5 } else { 4 }, WY_P1));
            }
            Value::Timestamp(t) => {
                // Pre-mix timestamp nanos
                let nanos = t.timestamp_nanos_opt().unwrap_or(i64::MAX);
                state.write_u64(wymix(3 ^ (nanos as u64), WY_P1));
            }
            Value::Extension(data) => {
                // Pre-hash extension data with WyHash-style mixing
                // Tag byte is included in data, so discriminant is embedded
                let bytes: &[u8] = data;
                let len = bytes.len();
                let mut h = wymix(10 ^ (len as u64), WY_P1);

                let chunks = len / 8;
                let ptr = bytes.as_ptr();
                for i in 0..chunks {
                    // SAFETY: We iterate i from 0..chunks where chunks = len/8.
                    // So i*8 is always < len, and we read 8 bytes which is valid
                    // since (i+1)*8 <= chunks*8 <= len. read_unaligned handles alignment.
                    let chunk = unsafe { (ptr.add(i * 8) as *const u64).read_unaligned() };
                    h = wymix(h ^ chunk, WY_P2);
                }

                let tail_start = chunks * 8;
                if tail_start < len {
                    let mut tail = 0u64;
                    for (j, &b) in bytes[tail_start..].iter().enumerate() {
                        tail |= (b as u64) << (j * 8);
                    }
                    h = wymix(h ^ tail, WY_P1);
                }

                state.write_u64(h);
            }
            Value::Blob(data) => {
                // Pre-hash blob data with WyHash-style mixing
                // Blob data is just bytes (no tag byte since it's its own variant)
                let bytes: &[u8] = data;
                let len = bytes.len();
                let mut h = wymix(11 ^ (len as u64), WY_P1); // discriminant 11 for Blob

                let chunks = len / 8;
                let ptr = bytes.as_ptr();
                for i in 0..chunks {
                    // SAFETY: We iterate i from 0..chunks where chunks = len/8.
                    // So i*8 is always < len, and we read 8 bytes which is valid
                    // since (i+1)*8 <= chunks*8 <= len. read_unaligned handles alignment.
                    let chunk = unsafe { (ptr.add(i * 8) as *const u64).read_unaligned() };
                    h = wymix(h ^ chunk, WY_P2);
                }

                let tail_start = chunks * 8;
                if tail_start < len {
                    let mut tail = 0u64;
                    for (j, &b) in bytes[tail_start..].iter().enumerate() {
                        tail |= (b as u64) << (j * 8);
                    }
                    h = wymix(h ^ tail, WY_P1);
                }

                state.write_u64(h);
            }
        }
    }
}

// Note: PartialOrd intentionally differs from Ord for SQL semantics
// - PartialOrd: SQL comparison (NULL returns None, cross-type numeric comparison)
// - Ord: BTreeMap ordering (NULLs first, type discriminant ordering)
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Use the original compare method for semantic correctness in SQL operations
        // This preserves NULL comparison semantics (returning None for NULL comparisons)
        // and proper cross-type numeric comparison (Integer vs Float)
        self.compare(other).ok()
    }
}

/// Total ordering implementation for Value
///
/// This is required for using Value as a key in BTreeMap/BTreeSet.
/// The ordering is defined as follows:
/// 1. NULLs are always ordered first (smallest)
/// 2. Numeric types (Integer, Float) are compared by numeric value (consistent with PartialEq)
/// 3. Other different data types are ordered by their type discriminant
/// 4. Same data types use their natural ordering
///
/// IMPORTANT: This ordering MUST be consistent with PartialEq. Since Integer(5) == Float(5.0)
/// per PartialEq, we must ensure Integer(5).cmp(&Float(5.0)) == Ordering::Equal.
/// Violating this contract causes BTreeMap corruption.
///
/// Note: This differs from SQL NULL semantics where NULL comparisons
/// return UNKNOWN. This ordering is only for internal index structure.
impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        // Handle NULL comparisons - NULLs are ordered first
        match (self.is_null(), other.is_null()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {} // Continue to value comparison
        }

        // Cross-type numeric comparison: Integer vs Float
        // This MUST be consistent with PartialEq where Integer(5) == Float(5.0)
        match (self, other) {
            (Value::Integer(i), Value::Float(f)) => {
                let i_as_f64 = *i as f64;
                // Handle NaN: NaN is ordered last
                if f.is_nan() {
                    return Ordering::Less; // Any number < NaN
                }
                return i_as_f64.partial_cmp(f).unwrap_or(Ordering::Equal);
            }
            (Value::Float(f), Value::Integer(i)) => {
                let i_as_f64 = *i as f64;
                // Handle NaN: NaN is ordered last
                if f.is_nan() {
                    return Ordering::Greater; // NaN > any number
                }
                return f.partial_cmp(&i_as_f64).unwrap_or(Ordering::Equal);
            }
            _ => {} // Continue to same-type comparison
        }

        // Helper function to get type discriminant for ordering
        fn type_discriminant(v: &Value) -> u8 {
            match v {
                Value::Null(_) => 0,
                Value::Boolean(_) => 1,
                // Integer and Float share the same discriminant for ordering purposes
                // This ensures they sort together by numeric value
                Value::Integer(_) | Value::Float(_) => 2,
                Value::Text(_) => 3,
                Value::Timestamp(_) => 4,
                Value::Extension(_) => 5,
                Value::Blob(_) => 6,
            }
        }

        let self_disc = type_discriminant(self);
        let other_disc = type_discriminant(other);

        // Different types: order by type discriminant
        if self_disc != other_disc {
            return self_disc.cmp(&other_disc);
        }

        // Same type comparison
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                // Handle NaN: NaN is ordered last
                match (a.is_nan(), b.is_nan()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
                }
            }
            (Value::Text(a), Value::Text(b)) => a.cmp(b),
            (Value::Boolean(a), Value::Boolean(b)) => a.cmp(b),
            (Value::Timestamp(a), Value::Timestamp(b)) => a.cmp(b),
            (Value::Extension(a), Value::Extension(b)) => {
                if a.first() != b.first() {
                    return a.cmp(b); // different extension types: byte order
                }
                let tag = a.first().copied().unwrap_or(0);
                match tag {
                    t if t == DataType::DeterministicFloat as u8 => {
                        match (extract_dfp_from_extension(a), extract_dfp_from_extension(b)) {
                            (Some(da), Some(db)) => compare_dfp(&da, &db),
                            _ => a.cmp(b), // fallback on deserialization failure
                        }
                    }
                    t if t == DataType::Quant as u8 => {
                        match (Value::as_dqa(self), Value::as_dqa(other)) {
                            (Some(da), Some(db)) => match dqa_cmp(da, db) {
                                -1 => Ordering::Less,
                                0 => Ordering::Equal,
                                1 => Ordering::Greater,
                                _ => Ordering::Equal,
                            },
                            _ => a.cmp(b),
                        }
                    }
                    t if t == DataType::Bigint as u8 => {
                        match (Value::as_bigint(self), Value::as_bigint(other)) {
                            (Some(ba), Some(bb)) => match ba.compare(&bb) {
                                -1 => Ordering::Less,
                                0 => Ordering::Equal,
                                1 => Ordering::Greater,
                                _ => {
                                    debug_assert!(false, "invalid bigint comparison result");
                                    a.cmp(b) // fallback on unexpected result
                                }
                            },
                            _ => a.cmp(b), // fallback on deserialization failure
                        }
                    }
                    t if t == DataType::Decimal as u8 => {
                        match (Value::as_decimal(self), Value::as_decimal(other)) {
                            (Some(da), Some(db)) => match decimal_cmp(&da, &db) {
                                -1 => Ordering::Less,
                                0 => Ordering::Equal,
                                1 => Ordering::Greater,
                                _ => {
                                    debug_assert!(false, "invalid decimal comparison result");
                                    a.cmp(b) // fallback on unexpected result
                                }
                            },
                            _ => a.cmp(b), // fallback on deserialization failure
                        }
                    }
                    _ => a.cmp(b), // other extensions: byte order
                }
            }
            (Value::Blob(a), Value::Blob(b)) => {
                // Compare by byte content first, then by length
                match a.cmp(b) {
                    Ordering::Equal => a.len().cmp(&b.len()),
                    other => other,
                }
            }
            _ => Ordering::Equal, // Should not reach here
        }
    }
}

// =========================================================================
// From implementations for convenient construction
// =========================================================================

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Integer(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<u16> for Value {
    fn from(v: u16) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<u8> for Value {
    fn from(v: u8) -> Self {
        Value::Integer(v as i64)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v as f64)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(SmartString::from_string(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(SmartString::from(v))
    }
}

impl From<Arc<str>> for Value {
    fn from(v: Arc<str>) -> Self {
        Value::Text(SmartString::from(v.as_ref()))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Boolean(v)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(v: DateTime<Utc>) -> Self {
        Value::Timestamp(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => Value::Null(DataType::Null),
        }
    }
}

// =========================================================================
// Helper functions
// =========================================================================

/// Parse a timestamp string with multiple format support
pub fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    let s = s.trim();

    // Try each timestamp format
    for format in TIMESTAMP_FORMATS {
        if let Ok(dt) = DateTime::parse_from_str(s, format) {
            return Ok(dt.with_timezone(&Utc));
        }
        // Try parsing as naive datetime and assume UTC
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, format) {
            return Ok(Utc.from_utc_datetime(&ndt));
        }
    }

    // Try date-only formats
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(Utc.from_utc_datetime(&datetime));
    }

    // Try time-only formats (use today's date)
    for format in TIME_FORMATS {
        if let Ok(time) = NaiveTime::parse_from_str(s, format) {
            let today = Utc::now().date_naive();
            let datetime = today.and_time(time);
            return Ok(Utc.from_utc_datetime(&datetime));
        }
    }

    Err(Error::parse(format!("invalid timestamp format: {}", s)))
}

/// Format a float value consistently
fn format_float(v: f64) -> String {
    // Handle special cases
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v.is_sign_positive() {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_string();
    }

    let abs_v = v.abs();

    // Use scientific notation for very large or very small numbers
    if abs_v != 0.0 && !(1e-4..1e15).contains(&abs_v) {
        // Use scientific notation with up to 15 significant digits
        let s = format!("{:e}", v);
        // Clean up trailing zeros in mantissa
        if let Some(e_pos) = s.find('e') {
            let (mantissa, exp) = s.split_at(e_pos);
            let clean_mantissa = if mantissa.contains('.') {
                mantissa
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                mantissa.to_string()
            };
            return format!("{}{}", clean_mantissa, exp);
        }
        return s;
    }

    if v.fract() == 0.0 {
        // Integer-like float, format without decimal
        format!("{:.0}", v)
    } else {
        // Use standard representation for normal range
        let s = format!("{:?}", v);
        // Remove trailing zeros after decimal point
        if s.contains('.') && !s.contains('e') && !s.contains('E') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    }
}

/// Format a DQA value as a decimal string
/// e.g., Dqa { value: 123, scale: 2 } → "1.23"
fn format_dqa(dqa: Dqa) -> String {
    if dqa.scale == 0 {
        return dqa.value.to_string();
    }
    let abs_val = dqa.value.unsigned_abs();
    let divisor = 10u64.pow(dqa.scale as u32);
    let whole = abs_val / divisor;
    let frac = abs_val % divisor;
    let frac_str = format!("{:0>width$}", frac, width = dqa.scale as usize);
    let frac_trimmed = frac_str.trim_end_matches('0');
    let sign = if dqa.value < 0 { "-" } else { "" };
    if frac_trimmed.is_empty() {
        format!("{}{}", sign, whole)
    } else {
        format!("{}{}.{}", sign, whole, frac_trimmed)
    }
}

/// Parse a decimal string into a DQA value
/// e.g., "1.23" → Dqa { value: 123, scale: 2 }
fn parse_string_to_dqa(s: &str) -> Option<Dqa> {
    let s = s.trim();
    if let Some(dot_pos) = s.find('.') {
        let int_part: i64 = s[..dot_pos].parse().ok()?;
        let frac_str = &s[dot_pos + 1..];
        let scale = frac_str.len() as u8;
        if scale == 0 || scale > 18 {
            return None;
        }
        let frac_val: u64 = frac_str.parse().ok()?;
        let magnitude = 10i64.pow(scale as u32);
        let combined = int_part
            .abs()
            .saturating_mul(magnitude)
            .saturating_add(frac_val as i64);
        let value = if int_part < 0 { -combined } else { combined };
        Dqa::new(value, scale).ok()
    } else {
        let value: i64 = s.parse().ok()?;
        Dqa::new(value, 0).ok()
    }
}

/// Re-export decimal_to_bytes so it can be used via crate::core::decimal_to_bytes
/// (imported privately for internal use, then re-exported publicly)
pub use octo_determin::decimal::decimal_to_bytes;

/// Parse a bigint string into a BigInt value per RFC-0110 §10.
/// Input format: `^[+-]?[0-9]+$` (decimal) or `^0x[0-9a-fA-F]+$` (hex).
/// Returns BigIntError on malformed input (including empty string, invalid hex).
pub fn stoolap_parse_bigint(s: &str) -> std::result::Result<BigInt, BigIntError> {
    BigInt::from_str(s)
}

/// Parse a decimal string into a Decimal value per RFC-0202-A §6.8a.
/// Input format: `^[+-]?[0-9]+(\.[0-9]+)?$` (rejects scientific notation, bare dots, whitespace-only)
/// Returns DecimalError::InvalidScale if fractional digits > 36
/// Returns DecimalError::Overflow if mantissa exceeds i128 range (>38 digits)
/// Returns DecimalError::NonCanonical for malformed input
pub fn stoolap_parse_decimal(s: &str) -> std::result::Result<Decimal, DecimalError> {
    let s = s.trim();

    // Check for empty string
    if s.is_empty() {
        return Err(DecimalError::NonCanonical);
    }

    let (sign, rest) = match s.starts_with('-') {
        true => (-1, &s[1..]),
        false => (1, s),
    };

    // Must start with digit (after optional sign)
    if rest.is_empty() || !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(DecimalError::NonCanonical);
    }

    // Split on decimal point if present
    let (int_part, frac_part) = match rest.find('.') {
        Some(pos) => (&rest[..pos], Some(&rest[pos + 1..])),
        None => (rest, None),
    };

    // Integer part must not be empty and must be all digits
    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(DecimalError::NonCanonical);
    }

    // Fractional part must be all digits if present
    if let Some(frac) = frac_part {
        if frac.is_empty() || !frac.chars().all(|c| c.is_ascii_digit()) {
            return Err(DecimalError::NonCanonical);
        }
    }

    // Total digits check for overflow (>38 digits in i128)
    // i128 max is about 1.7e38, which is 39 digits, but 10^38 is the boundary
    let total_digits = int_part.len() + frac_part.map(|f| f.len()).unwrap_or(0);
    if total_digits > 38 {
        return Err(DecimalError::Overflow);
    }

    // Build mantissa string and parse as i128
    let mantissa_str = if let Some(frac) = frac_part {
        format!("{}{}", int_part, frac)
    } else {
        int_part.to_string()
    };

    let mantissa: i128 = mantissa_str.parse().map_err(|_| DecimalError::Overflow)?;

    // Scale is the number of fractional digits
    let mut scale = frac_part.map(|f| f.len()).unwrap_or(0) as u8;

    // Scale must be <= 36
    if scale > 36 {
        return Err(DecimalError::InvalidScale);
    }

    // Apply sign
    let mut mantissa = mantissa * (sign as i128);

    // Validate bounds before canonicalization
    if mantissa.abs() > octo_determin::decimal::MAX_DECIMAL_MANTISSA {
        return Err(DecimalError::Overflow);
    }

    // Canonicalize: remove trailing zeros (e.g., "1.0" with mantissa=10, scale=1 → mantissa=1, scale=0)
    // This ensures we produce canonical Decimal wire format per RFC-0111 §Canonical Byte Format
    if mantissa != 0 {
        while mantissa % 10 == 0 && scale > 0 {
            mantissa /= 10;
            scale -= 1;
        }
    }

    Decimal::new(mantissa, scale)
}

/// Format packed LE f32 bytes as "[1.0, 2.0, 3.0]" string
pub fn format_vector_bytes(data: &[u8]) -> String {
    let len = data.len() / 4;
    let mut s = String::with_capacity(len * 8 + 2);
    s.push('[');
    for i in 0..len {
        if i > 0 {
            s.push_str(", ");
        }
        let f = f32::from_le_bytes([
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ]);
        use std::fmt::Write;
        if f.fract() == 0.0 && f.is_finite() {
            let _ = write!(s, "{:.1}", f);
        } else {
            let _ = write!(s, "{}", f);
        }
    }
    s.push(']');
    s
}

/// Convert Extension byte data to str (for JSON and other UTF-8 extension types)
///
/// Parse a vector string in [f32, f32, ...] format
pub fn parse_vector_str(s: &str) -> Option<Vec<f32>> {
    let s = s.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for part in inner.split(',') {
        let val: f32 = part.trim().parse().ok()?;
        result.push(val);
    }
    Some(result)
}

/// Extract DFP from raw Extension bytes
fn extract_dfp_from_extension(data: &CompactArc<[u8]>) -> Option<Dfp> {
    if data.len() < 25 {
        return None;
    }
    let encoding_bytes: [u8; 24] = data[1..25].try_into().ok()?;
    Some(DfpEncoding::from_bytes(encoding_bytes).to_dfp())
}

/// Compare two floats with proper NaN handling
fn compare_floats(a: f64, b: f64) -> Ordering {
    // Handle NaN: treat as greater than all other values for consistency
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
    }
}

/// Compare two DFP values with proper special value handling
fn compare_dfp(a: &Dfp, b: &Dfp) -> Ordering {
    // Handle NaN: treat as greater than all other values for consistency
    use DfpClass::*;
    match (a.class, b.class) {
        (NaN, NaN) => Ordering::Equal,
        (NaN, _) => Ordering::Greater,
        (_, NaN) => Ordering::Less,
        (Zero, Zero)
        | (Zero, _)
        | (_, Zero)
        | (Infinity, Infinity)
        | (Infinity, _)
        | (_, Infinity)
        | (Normal, Normal)
        | (Normal, _)
        | (_, Normal) => {
            // Compare signs first
            if a.sign != b.sign {
                return if a.sign {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }

            // Both same sign - compare magnitudes
            let cmp = compare_dfp_magnitude(a, b);

            // If both negative, flip the result
            if a.sign {
                cmp.reverse()
            } else {
                cmp
            }
        }
    }
}

/// Compare magnitude of two DFP values (absolute value comparison)
fn compare_dfp_magnitude(a: &Dfp, b: &Dfp) -> Ordering {
    use DfpClass::*;

    match (a.class, b.class) {
        // Zero vs anything
        (Zero, Zero) => Ordering::Equal,
        (Zero, _) => Ordering::Less,
        (_, Zero) => Ordering::Greater,
        // Infinity vs finite
        (Infinity, Infinity) => Ordering::Equal,
        (Infinity, _) => Ordering::Greater,
        (_, Infinity) => Ordering::Less,
        // NaN should have been handled earlier, but include for exhaustiveness
        (NaN, _) | (_, NaN) => Ordering::Equal,
        // Normal vs any - compare actual values via to_f64()
        // This handles the fact that DFP mantissa/exponent don't compare directly
        _ => {
            let a_val = a.to_f64();
            let b_val = b.to_f64();
            compare_floats(a_val, b_val)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    // =========================================================================
    // Size verification tests
    // =========================================================================

    #[test]
    fn test_value_size() {
        use std::mem::size_of;

        // Value must be exactly 16 bytes for memory efficiency
        assert_eq!(
            size_of::<Value>(),
            16,
            "Value should be 16 bytes, got {}",
            size_of::<Value>()
        );

        // Option<Value> should also be 16 bytes due to niche optimization
        assert_eq!(
            size_of::<Option<Value>>(),
            16,
            "Option<Value> should be 16 bytes (niche optimization), got {}",
            size_of::<Option<Value>>()
        );
    }

    // =========================================================================
    // Constructor tests
    // =========================================================================

    #[test]
    fn test_constructors() {
        assert!(Value::null(DataType::Integer).is_null());
        assert_eq!(Value::integer(42).as_int64(), Some(42));
        assert_eq!(Value::float(3.5).as_float64(), Some(3.5));
        assert_eq!(Value::text("hello").as_str(), Some("hello"));
        assert_eq!(Value::boolean(true).as_boolean(), Some(true));
        assert!(Value::json(r#"{"key": "value"}"#).as_json().is_some());
    }

    #[test]
    fn test_from_implementations() {
        let v: Value = 42i64.into();
        assert_eq!(v.as_int64(), Some(42));

        let v: Value = 3.5f64.into();
        assert_eq!(v.as_float64(), Some(3.5));

        let v: Value = "hello".into();
        assert_eq!(v.as_str(), Some("hello"));

        let v: Value = true.into();
        assert_eq!(v.as_boolean(), Some(true));

        let v: Value = Option::<i64>::None.into();
        assert!(v.is_null());

        let v: Value = Some(42i64).into();
        assert_eq!(v.as_int64(), Some(42));
    }

    // =========================================================================
    // Type accessor tests
    // =========================================================================

    #[test]
    fn test_data_type() {
        assert_eq!(
            Value::null(DataType::Integer).data_type(),
            DataType::Integer
        );
        assert_eq!(Value::integer(42).data_type(), DataType::Integer);
        assert_eq!(Value::float(3.5).data_type(), DataType::Float);
        assert_eq!(Value::text("hello").data_type(), DataType::Text);
        assert_eq!(Value::boolean(true).data_type(), DataType::Boolean);
        assert_eq!(
            Value::Timestamp(Utc::now()).data_type(),
            DataType::Timestamp
        );
        assert_eq!(Value::json("{}").data_type(), DataType::Json);
    }

    // =========================================================================
    // AsXxx conversion tests
    // =========================================================================

    #[test]
    fn test_as_int64() {
        // Direct integer
        assert_eq!(Value::integer(42).as_int64(), Some(42));

        // Float to integer (truncates)
        assert_eq!(Value::float(3.7).as_int64(), Some(3));
        assert_eq!(Value::float(-3.7).as_int64(), Some(-3));

        // String to integer
        assert_eq!(Value::text("42").as_int64(), Some(42));
        assert_eq!(Value::text("-42").as_int64(), Some(-42));
        assert_eq!(Value::text("3.7").as_int64(), Some(3)); // Parse as float, convert

        // Boolean to integer
        assert_eq!(Value::boolean(true).as_int64(), Some(1));
        assert_eq!(Value::boolean(false).as_int64(), Some(0));

        // NULL returns None
        assert_eq!(Value::null(DataType::Integer).as_int64(), None);

        // Invalid string
        assert_eq!(Value::text("not a number").as_int64(), None);
    }

    #[test]
    fn test_as_float64() {
        // Direct float
        assert_eq!(Value::float(3.5).as_float64(), Some(3.5));

        // Integer to float
        assert_eq!(Value::integer(42).as_float64(), Some(42.0));

        // String to float
        assert_eq!(Value::text("3.5").as_float64(), Some(3.5));

        // Boolean to float
        assert_eq!(Value::boolean(true).as_float64(), Some(1.0));
        assert_eq!(Value::boolean(false).as_float64(), Some(0.0));

        // NULL returns None
        assert_eq!(Value::null(DataType::Float).as_float64(), None);
    }

    #[test]
    fn test_as_boolean() {
        // Direct boolean
        assert_eq!(Value::boolean(true).as_boolean(), Some(true));
        assert_eq!(Value::boolean(false).as_boolean(), Some(false));

        // Integer to boolean
        assert_eq!(Value::integer(1).as_boolean(), Some(true));
        assert_eq!(Value::integer(0).as_boolean(), Some(false));
        assert_eq!(Value::integer(-1).as_boolean(), Some(true));

        // Float to boolean
        assert_eq!(Value::float(1.0).as_boolean(), Some(true));
        assert_eq!(Value::float(0.0).as_boolean(), Some(false));

        // String to boolean (various string values)
        assert_eq!(Value::text("true").as_boolean(), Some(true));
        assert_eq!(Value::text("TRUE").as_boolean(), Some(true));
        assert_eq!(Value::text("t").as_boolean(), Some(true));
        assert_eq!(Value::text("yes").as_boolean(), Some(true));
        assert_eq!(Value::text("y").as_boolean(), Some(true));
        assert_eq!(Value::text("1").as_boolean(), Some(true));
        assert_eq!(Value::text("false").as_boolean(), Some(false));
        assert_eq!(Value::text("FALSE").as_boolean(), Some(false));
        assert_eq!(Value::text("f").as_boolean(), Some(false));
        assert_eq!(Value::text("no").as_boolean(), Some(false));
        assert_eq!(Value::text("n").as_boolean(), Some(false));
        assert_eq!(Value::text("0").as_boolean(), Some(false));
        assert_eq!(Value::text("").as_boolean(), Some(false));

        // Numeric strings
        assert_eq!(Value::text("42").as_boolean(), Some(true));
        assert_eq!(Value::text("0.0").as_boolean(), Some(false));
    }

    #[test]
    fn test_as_string() {
        // Direct string
        assert_eq!(Value::text("hello").as_string(), Some("hello".to_string()));

        // Integer to string
        assert_eq!(Value::integer(42).as_string(), Some("42".to_string()));

        // Float to string
        assert_eq!(Value::float(3.5).as_string(), Some("3.5".to_string()));

        // Boolean to string
        assert_eq!(Value::boolean(true).as_string(), Some("true".to_string()));
        assert_eq!(Value::boolean(false).as_string(), Some("false".to_string()));

        // NULL returns None
        assert_eq!(Value::null(DataType::Text).as_string(), None);
    }

    // =========================================================================
    // Equality tests
    // =========================================================================

    #[test]
    fn test_equality() {
        // Same type equality
        assert_eq!(Value::integer(42), Value::integer(42));
        assert_ne!(Value::integer(42), Value::integer(43));

        assert_eq!(Value::float(3.5), Value::float(3.5));
        assert_ne!(Value::float(3.5), Value::float(3.15));

        assert_eq!(Value::text("hello"), Value::text("hello"));
        assert_ne!(Value::text("hello"), Value::text("world"));

        assert_eq!(Value::boolean(true), Value::boolean(true));
        assert_ne!(Value::boolean(true), Value::boolean(false));

        // NULL equality
        assert_eq!(Value::null(DataType::Integer), Value::null(DataType::Float));
        assert_ne!(Value::null(DataType::Integer), Value::integer(0));

        // Cross-type numeric comparison: Integer and Float with same value ARE equal
        // This is important for queries like WHERE id = 5.0 or WHERE price = 100
        assert_eq!(Value::integer(1), Value::float(1.0));
        assert_eq!(Value::integer(5), Value::float(5.0));
        assert_ne!(Value::integer(1), Value::float(1.5)); // Different values are not equal

        // Different non-numeric types are not equal
        assert_ne!(Value::text("1"), Value::integer(1));
    }

    #[test]
    fn test_float_nan_equality() {
        // NaN handling: NaN == NaN in our implementation (for consistency)
        let nan = Value::float(f64::NAN);
        assert_eq!(nan, nan.clone());
    }

    // =========================================================================
    // Comparison tests
    // =========================================================================

    #[test]
    fn test_compare_integers() {
        assert_eq!(
            Value::integer(1).compare(&Value::integer(2)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            Value::integer(2).compare(&Value::integer(2)).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            Value::integer(3).compare(&Value::integer(2)).unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_floats() {
        assert_eq!(
            Value::float(1.0).compare(&Value::float(2.0)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            Value::float(2.0).compare(&Value::float(2.0)).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            Value::float(3.0).compare(&Value::float(2.0)).unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_cross_type_numeric() {
        // Integer vs Float comparison
        assert_eq!(
            Value::integer(1).compare(&Value::float(2.0)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            Value::integer(2).compare(&Value::float(2.0)).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            Value::float(3.0).compare(&Value::integer(2)).unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_strings() {
        assert_eq!(
            Value::text("a").compare(&Value::text("b")).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            Value::text("b").compare(&Value::text("b")).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            Value::text("c").compare(&Value::text("b")).unwrap(),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_null() {
        // NULL comparisons
        assert_eq!(
            Value::null(DataType::Integer)
                .compare(&Value::null(DataType::Float))
                .unwrap(),
            Ordering::Equal
        );

        // NULL vs non-NULL should error
        assert!(Value::null(DataType::Integer)
            .compare(&Value::integer(0))
            .is_err());
        assert!(Value::integer(0)
            .compare(&Value::null(DataType::Integer))
            .is_err());
    }

    #[test]
    fn test_compare_json_error() {
        // JSON comparison only allows equality
        let j1 = Value::json(r#"{"a": 1}"#);
        let j2 = Value::json(r#"{"b": 2}"#);
        assert!(j1.compare(&j2).is_err());

        // Same JSON values are equal
        let j3 = Value::json(r#"{"a": 1}"#);
        assert_eq!(j1.compare(&j3).unwrap(), Ordering::Equal);
    }

    // =========================================================================
    // Timestamp parsing tests
    // =========================================================================

    #[test]
    fn test_parse_timestamp() {
        // RFC3339
        let ts = parse_timestamp("2024-01-15T10:30:00Z").unwrap();
        assert_eq!(ts.year(), 2024);
        assert_eq!(ts.month(), 1);
        assert_eq!(ts.day(), 15);
        assert_eq!(ts.hour(), 10);
        assert_eq!(ts.minute(), 30);

        // SQL format
        let ts = parse_timestamp("2024-01-15 10:30:00").unwrap();
        assert_eq!(ts.year(), 2024);

        // Date only
        let ts = parse_timestamp("2024-01-15").unwrap();
        assert_eq!(ts.year(), 2024);
        assert_eq!(ts.hour(), 0);

        // Invalid format
        assert!(parse_timestamp("not a date").is_err());
    }

    // =========================================================================
    // Display tests
    // =========================================================================

    #[test]
    fn test_display() {
        assert_eq!(Value::null(DataType::Integer).to_string(), "NULL");
        assert_eq!(Value::integer(42).to_string(), "42");
        assert_eq!(Value::float(3.5).to_string(), "3.5");
        assert_eq!(Value::text("hello").to_string(), "hello");
        assert_eq!(Value::boolean(true).to_string(), "true");
        assert_eq!(Value::boolean(false).to_string(), "false");
    }

    // =========================================================================
    // Hash tests
    // =========================================================================

    #[test]
    fn test_hash() {
        use rustc_hash::FxHashSet;

        let mut set = FxHashSet::default();
        set.insert(Value::integer(42));
        set.insert(Value::integer(42)); // Duplicate
        set.insert(Value::integer(43));

        assert_eq!(set.len(), 2);
        assert!(set.contains(&Value::integer(42)));
        assert!(set.contains(&Value::integer(43)));
    }

    #[test]
    fn test_hash_integer_float_consistency() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_value(v: &Value) -> u64 {
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            hasher.finish()
        }

        // Basic case: Integer(5) and Float(5.0) must hash the same
        assert_eq!(
            hash_value(&Value::integer(5)),
            hash_value(&Value::float(5.0))
        );
        assert_eq!(
            hash_value(&Value::integer(-100)),
            hash_value(&Value::float(-100.0))
        );
        assert_eq!(
            hash_value(&Value::integer(0)),
            hash_value(&Value::float(0.0))
        );

        // Fractional floats should NOT hash the same as any integer
        assert_ne!(
            hash_value(&Value::float(5.5)),
            hash_value(&Value::integer(5))
        );
        assert_ne!(
            hash_value(&Value::float(5.5)),
            hash_value(&Value::integer(6))
        );

        // Large integers within safe range
        let safe_max = (1_i64 << 53) - 1; // 9007199254740991
        assert_eq!(
            hash_value(&Value::integer(safe_max)),
            hash_value(&Value::float(safe_max as f64))
        );
        assert_eq!(
            hash_value(&Value::integer(-safe_max)),
            hash_value(&Value::float(-safe_max as f64))
        );

        // Boundary case: 2^53 (outside safe range, uses f64.to_bits())
        let boundary = 1_i64 << 53; // 9007199254740992
        assert_eq!(
            hash_value(&Value::integer(boundary)),
            hash_value(&Value::float(boundary as f64))
        );

        // Large integers that round to same f64 should hash same as that f64
        // 2^53 + 1 rounds to 2^53 in f64
        let large = boundary + 1; // 9007199254740993
        let large_as_f64 = large as f64; // rounds to 9007199254740992.0
        assert_eq!(
            hash_value(&Value::integer(large)),
            hash_value(&Value::float(large_as_f64))
        );
    }

    #[test]
    fn test_hash_in_hashmap() {
        use rustc_hash::FxHashMap;

        // Test that Integer and Float can be used as equivalent keys
        let mut map = FxHashMap::default();
        map.insert(Value::integer(42), "int");

        // Looking up with Float(42.0) should find the Integer(42) entry
        assert_eq!(map.get(&Value::float(42.0)), Some(&"int"));

        // Inserting Float(42.0) should overwrite Integer(42)
        map.insert(Value::float(42.0), "float");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&Value::integer(42)), Some(&"float"));
    }

    #[test]
    fn test_hash_nan_consistency() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_value(v: &Value) -> u64 {
            let mut hasher = DefaultHasher::new();
            v.hash(&mut hasher);
            hasher.finish()
        }

        // All NaN values must hash the same (they're equal in PartialEq)
        let nan1 = Value::float(f64::NAN);
        // Use a different NaN representation (quiet vs signaling doesn't matter for hash equality)
        let nan2 = Value::float(f64::from_bits(0x7ff8000000000001)); // Another NaN bit pattern
        let nan3 = Value::float(f64::INFINITY - f64::INFINITY);

        assert_eq!(hash_value(&nan1), hash_value(&nan2));
        assert_eq!(hash_value(&nan2), hash_value(&nan3));

        // Verify they're equal in PartialEq
        assert_eq!(nan1, nan2);
        assert_eq!(nan2, nan3);
    }

    #[test]
    fn test_blob_compare() {
        use std::cmp::Ordering;

        let blob1 = Value::blob(vec![0x01u8; 32]);
        let blob2 = Value::blob(vec![0x01u8; 32]);
        let blob3 = Value::blob(vec![0x02u8; 32]);

        // Test compare() - should return Ok(Ordering::Equal) for equal blobs
        assert_eq!(blob1.compare(&blob2).unwrap(), Ordering::Equal);
        assert_eq!(blob1.compare(&blob3).unwrap(), Ordering::Less);
        assert_eq!(blob3.compare(&blob1).unwrap(), Ordering::Greater);

        // Test PartialEq - should return true for equal blobs
        assert!(blob1 == blob2);
        assert!(blob1 != blob3);
    }

    // =========================================================================
    // Bug fix tests: DQA round-trip, DFP/DQA same-type compare, DFP/DQA Ord
    // =========================================================================

    #[test]
    fn test_dqa_quant_round_trip() {
        let dqa = Dqa::new(12345, 2).unwrap();
        let v = Value::quant(dqa);
        // as_dqa() should extract the same value
        let extracted = v.as_dqa().expect("as_dqa should succeed");
        assert_eq!(extracted.value, 12345);
        assert_eq!(extracted.scale, 2);
    }

    #[test]
    fn test_dfp_same_type_compare() {
        let v1 = Value::dfp(Dfp::from_f64(1.0));
        let v2 = Value::dfp(Dfp::from_f64(2.0));
        let v3 = Value::dfp(Dfp::from_f64(1.0));
        // Should return ordering, not IncomparableTypes
        assert_eq!(v1.compare(&v2).unwrap(), Ordering::Less);
        assert_eq!(v2.compare(&v1).unwrap(), Ordering::Greater);
        assert_eq!(v1.compare(&v3).unwrap(), Ordering::Equal);
    }

    #[test]
    fn test_dqa_same_type_compare() {
        let v1 = Value::quant(Dqa::new(1, 0).unwrap());
        let v2 = Value::quant(Dqa::new(2, 0).unwrap());
        let v3 = Value::quant(Dqa::new(1, 0).unwrap());
        assert_eq!(v1.compare(&v2).unwrap(), Ordering::Less);
        assert_eq!(v2.compare(&v1).unwrap(), Ordering::Greater);
        assert_eq!(v1.compare(&v3).unwrap(), Ordering::Equal);
    }

    #[test]
    fn test_dfp_ord() {
        let v1 = Value::dfp(Dfp::from_f64(1.0));
        let v2 = Value::dfp(Dfp::from_f64(2.0));
        let v3 = Value::dfp(Dfp::from_f64(3.0));
        assert!(v1 < v2, "dfp(1.0) should be less than dfp(2.0)");
        assert!(v2 > v1, "dfp(2.0) should be greater than dfp(1.0)");
        assert!(v2 < v3, "dfp(2.0) should be less than dfp(3.0)");
        assert!(v3 > v2, "dfp(3.0) should be greater than dfp(2.0)");
        // Also verify compare() is consistent with PartialOrd
        assert_eq!(v2.compare(&v3).unwrap(), Ordering::Less);
        assert_eq!(v3.compare(&v2).unwrap(), Ordering::Greater);
    }

    #[test]
    fn test_dqa_ord() {
        let v1 = Value::quant(Dqa::new(1, 0).unwrap());
        let v2 = Value::quant(Dqa::new(2, 0).unwrap());
        assert!(v1 < v2, "dqa(1,0) should be less than dqa(2,0)");
        assert!(v2 > v1, "dqa(2,0) should be greater than dqa(1,0)");
    }

    #[test]
    fn test_dqa_ord_negative() {
        let vn = Value::quant(Dqa::new(-5, 0).unwrap());
        let vp = Value::quant(Dqa::new(5, 0).unwrap());
        assert!(vn < vp, "dqa(-5) should be less than dqa(5)");
    }

    // =========================================================================
    // Integration tests: DFP/DQA fixes from code reviews
    // =========================================================================

    #[test]
    fn test_as_float64_dfp() {
        // DFP values should convert to f64 for cross-type comparison
        let v = Value::dfp(Dfp::from_f64(2.75));
        assert_eq!(v.as_float64(), Some(2.75));

        let v_zero = Value::dfp(Dfp::from_f64(0.0));
        assert_eq!(v_zero.as_float64(), Some(0.0));

        let v_neg = Value::dfp(Dfp::from_f64(-42.5));
        assert_eq!(v_neg.as_float64(), Some(-42.5));
    }

    #[test]
    fn test_as_float64_dqa() {
        // DQA values should convert to f64 for cross-type comparison
        let v = Value::quant(Dqa::new(315, 2).unwrap()); // 3.15
        let f = v.as_float64().unwrap();
        assert!(
            (f - 3.15).abs() < 1e-10,
            "DQA(315,2) should be ~3.15, got {}",
            f
        );

        let v_int = Value::quant(Dqa::new(42, 0).unwrap());
        assert_eq!(v_int.as_float64(), Some(42.0));

        let v_neg = Value::quant(Dqa::new(-150, 2).unwrap()); // -1.50
        assert_eq!(v_neg.as_float64(), Some(-1.50));
    }

    #[test]
    fn test_as_string_dfp() {
        let v = Value::dfp(Dfp::from_f64(2.75));
        let s = v.as_string().expect("DFP as_string should work");
        assert!(
            !s.contains("extension"),
            "should not contain <extension:...>, got: {}",
            s
        );
    }

    #[test]
    fn test_as_string_dqa() {
        let v = Value::quant(Dqa::new(123, 2).unwrap()); // 1.23
        let s = v.as_string().expect("DQA as_string should work");
        assert_eq!(s, "1.23");

        let v_int = Value::quant(Dqa::new(42, 0).unwrap());
        assert_eq!(v_int.as_string(), Some("42".to_string()));
    }

    #[test]
    fn test_display_dfp() {
        let v = Value::dfp(Dfp::from_f64(2.5));
        let display = format!("{}", v);
        assert!(
            !display.contains("extension"),
            "DFP display should show numeric value, got: {}",
            display
        );
    }

    #[test]
    fn test_display_dqa() {
        let v = Value::quant(Dqa::new(123, 2).unwrap()); // 1.23
        let display = format!("{}", v);
        assert_eq!(display, "1.23");

        let v_int = Value::quant(Dqa::new(42, 0).unwrap());
        assert_eq!(format!("{}", v_int), "42");
    }

    #[test]
    fn test_cast_to_dqa() {
        // Integer → DQA
        let v = Value::integer(42);
        let cast = v.cast_to_type(DataType::Quant);
        let dqa = cast.as_dqa().expect("should be DQA");
        assert_eq!(dqa.value, 42);
        assert_eq!(dqa.scale, 0);

        // Float → DQA
        let v = Value::float(1.5);
        let cast = v.cast_to_type(DataType::Quant);
        let dqa = cast.as_dqa().expect("should be DQA");
        assert_eq!(dqa.value, 15);
        assert_eq!(dqa.scale, 1);

        // Text → DQA
        let v = Value::text("2.50");
        let cast = v.cast_to_type(DataType::Quant);
        let dqa = cast.as_dqa().expect("should be DQA from text");
        assert_eq!(dqa.value, 250);
        assert_eq!(dqa.scale, 2);

        // Same type → identity
        let v = Value::quant(Dqa::new(99, 1).unwrap());
        let cast = v.cast_to_type(DataType::Quant);
        assert_eq!(cast.as_dqa().unwrap().value, 99);
    }

    #[test]
    fn test_into_coerce_dfp() {
        // Integer → DFP
        let v = Value::integer(42);
        let coerced = v.into_coerce_to_type(DataType::DeterministicFloat);
        assert!(coerced.as_dfp().is_some());

        // Float → DFP
        let v = Value::float(2.75);
        let coerced = v.into_coerce_to_type(DataType::DeterministicFloat);
        assert!(coerced.as_dfp().is_some());

        // Same type → identity
        let v = Value::dfp(Dfp::from_f64(1.0));
        let coerced = v.into_coerce_to_type(DataType::DeterministicFloat);
        assert!(coerced.as_dfp().is_some());
    }

    #[test]
    fn test_into_coerce_dqa() {
        // Integer → DQA
        let v = Value::integer(42);
        let coerced = v.into_coerce_to_type(DataType::Quant);
        let dqa = coerced.as_dqa().expect("should be DQA");
        assert_eq!(dqa.value, 42);
        assert_eq!(dqa.scale, 0);

        // Float → DQA
        let v = Value::float(1.5);
        let coerced = v.into_coerce_to_type(DataType::Quant);
        let dqa = coerced.as_dqa().expect("should be DQA");
        assert_eq!(dqa.value, 15);
        assert_eq!(dqa.scale, 1);

        // Same type → identity
        let v = Value::quant(Dqa::new(99, 1).unwrap());
        let coerced = v.into_coerce_to_type(DataType::Quant);
        assert_eq!(coerced.as_dqa().unwrap().value, 99);
    }

    #[test]
    fn test_from_typed_dfp() {
        // String → DFP
        let boxed = "2.75".to_string();
        let v = Value::from_typed(
            Some(&boxed as &dyn std::any::Any),
            DataType::DeterministicFloat,
        );
        assert!(v.as_dfp().is_some());

        // Integer → DFP
        let boxed = 42i64;
        let v = Value::from_typed(
            Some(&boxed as &dyn std::any::Any),
            DataType::DeterministicFloat,
        );
        assert!(v.as_dfp().is_some());
    }

    #[test]
    fn test_from_typed_dqa() {
        // String → DQA
        let boxed = "1.50".to_string();
        let v = Value::from_typed(Some(&boxed as &dyn std::any::Any), DataType::Quant);
        let dqa = v.as_dqa().expect("should be DQA");
        assert_eq!(dqa.value, 150);
        assert_eq!(dqa.scale, 2);

        // Integer → DQA
        let boxed = 42i64;
        let v = Value::from_typed(Some(&boxed as &dyn std::any::Any), DataType::Quant);
        let dqa = v.as_dqa().expect("should be DQA from int");
        assert_eq!(dqa.value, 42);
        assert_eq!(dqa.scale, 0);
    }

    #[test]
    fn test_cross_type_compare_dfp_int() {
        // DFP vs Integer: should not panic, should compare by numeric value
        let dfp = Value::dfp(Dfp::from_f64(5.0));
        let int = Value::integer(5);
        assert_eq!(dfp.compare(&int).unwrap(), Ordering::Equal);

        let dfp = Value::dfp(Dfp::from_f64(3.0));
        let int = Value::integer(5);
        assert_eq!(dfp.compare(&int).unwrap(), Ordering::Less);

        let dfp = Value::dfp(Dfp::from_f64(10.0));
        let int = Value::integer(5);
        assert_eq!(dfp.compare(&int).unwrap(), Ordering::Greater);
    }

    #[test]
    fn test_cross_type_compare_dqa_float() {
        // DQA vs Float: should not panic, should compare by numeric value
        let dqa = Value::quant(Dqa::new(15, 1).unwrap()); // 1.5
        let float = Value::float(1.5);
        assert_eq!(dqa.compare(&float).unwrap(), Ordering::Equal);

        let dqa = Value::quant(Dqa::new(10, 1).unwrap()); // 1.0
        let float = Value::float(1.5);
        assert_eq!(dqa.compare(&float).unwrap(), Ordering::Less);
    }

    #[test]
    fn test_cross_type_compare_dqa_int() {
        // DQA vs Integer: should not panic
        let dqa = Value::quant(Dqa::new(50, 1).unwrap()); // 5.0
        let int = Value::integer(5);
        assert_eq!(dqa.compare(&int).unwrap(), Ordering::Equal);
    }

    #[test]
    fn test_format_dqa_values() {
        // Scale 0: no decimal point
        assert_eq!(format_dqa(Dqa::new(42, 0).unwrap()), "42");
        assert_eq!(format_dqa(Dqa::new(-5, 0).unwrap()), "-5");

        // Scale 2: decimal formatting
        assert_eq!(format_dqa(Dqa::new(123, 2).unwrap()), "1.23");
        assert_eq!(format_dqa(Dqa::new(-123, 2).unwrap()), "-1.23");
        assert_eq!(format_dqa(Dqa::new(100, 2).unwrap()), "1");
        assert_eq!(format_dqa(Dqa::new(1, 2).unwrap()), "0.01");
    }
}
