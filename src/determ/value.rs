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

//! Deterministic Value type for blockchain SQL database
//!
//! This module provides deterministic values that:
//! - Use no Arc/pointers for predictable memory layout
//! - Store small text inline ([u8; 15], u8 length)
//! - Store large text on heap with Box<[u8]>
//! - Support deterministic Merkle hashing

use crate::core::{Error, Result};
use sha2::{Digest, Sha256};

/// Deterministic Value enum
///
/// This is a deterministic version of Value that:
/// - Uses no Arc or pointers for predictable memory layout
/// - Stores small text inline ([u8; 15], u8 length)
/// - Stores large text on heap with Box<[u8]>
/// - Supports deterministic Merkle hashing
#[derive(Debug, Clone)]
pub enum DetermValue {
    /// NULL value
    Null,
    /// 64-bit signed integer
    Integer(i64),
    /// 64-bit floating point
    Float(f64),
    /// Inline text (15 bytes or less)
    InlineText([u8; 15], u8),
    /// Heap text (more than 15 bytes)
    HeapText(Box<[u8]>),
    /// Boolean value
    Boolean(bool),
    /// Timestamp (i64 nanoseconds since Unix epoch)
    Timestamp(i64),
    /// Extension type for future use
    Extension(Box<[u8]>),
}

impl PartialEq for DetermValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DetermValue::Null, DetermValue::Null) => true,
            (DetermValue::Integer(a), DetermValue::Integer(b)) => a == b,
            (DetermValue::Float(a), DetermValue::Float(b)) => a.to_bits() == b.to_bits(),
            (DetermValue::InlineText(a_data, a_len), DetermValue::InlineText(b_data, b_len)) => {
                a_len == b_len && &a_data[..*a_len as usize] == &b_data[..*b_len as usize]
            }
            (DetermValue::HeapText(a), DetermValue::HeapText(b)) => a == b,
            (DetermValue::Boolean(a), DetermValue::Boolean(b)) => a == b,
            (DetermValue::Timestamp(a), DetermValue::Timestamp(b)) => a == b,
            (DetermValue::Extension(a), DetermValue::Extension(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for DetermValue {}

impl DetermValue {
    // =========================================================================
    // Data type identifiers
    // =========================================================================

    /// Data type tag for Null
    pub const TYPE_NULL: u8 = 0;
    /// Data type tag for Integer
    pub const TYPE_INTEGER: u8 = 1;
    /// Data type tag for Float
    pub const TYPE_FLOAT: u8 = 2;
    /// Data type tag for InlineText
    pub const TYPE_INLINE_TEXT: u8 = 3;
    /// Data type tag for HeapText
    pub const TYPE_HEAP_TEXT: u8 = 4;
    /// Data type tag for Boolean
    pub const TYPE_BOOLEAN: u8 = 5;
    /// Data type tag for Timestamp
    pub const TYPE_TIMESTAMP: u8 = 6;
    /// Data type tag for Extension
    pub const TYPE_EXTENSION: u8 = 7;

    // =========================================================================
    // Constructors
    // =========================================================================

    /// Create a NULL value
    pub fn null() -> Self {
        DetermValue::Null
    }

    /// Create an integer value
    pub fn integer(value: i64) -> Self {
        DetermValue::Integer(value)
    }

    /// Create a float value
    pub fn float(value: f64) -> Self {
        DetermValue::Float(value)
    }

    /// Create a text value (automatically chooses inline or heap)
    pub fn text(value: &str) -> Self {
        let bytes = value.as_bytes();
        if bytes.len() <= 15 {
            let mut data = [0u8; 15];
            data[..bytes.len()].copy_from_slice(bytes);
            DetermValue::InlineText(data, bytes.len() as u8)
        } else {
            DetermValue::HeapText(bytes.into())
        }
    }

    /// Create a boolean value
    pub fn boolean(value: bool) -> Self {
        DetermValue::Boolean(value)
    }

    /// Create a timestamp value
    pub fn timestamp(value: i64) -> Self {
        DetermValue::Timestamp(value)
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Check if this is a NULL value
    pub fn is_null(&self) -> bool {
        matches!(self, DetermValue::Null)
    }

    /// Get the data type identifier
    pub fn data_type(&self) -> u8 {
        match self {
            DetermValue::Null => Self::TYPE_NULL,
            DetermValue::Integer(_) => Self::TYPE_INTEGER,
            DetermValue::Float(_) => Self::TYPE_FLOAT,
            DetermValue::InlineText(_, _) => Self::TYPE_INLINE_TEXT,
            DetermValue::HeapText(_) => Self::TYPE_HEAP_TEXT,
            DetermValue::Boolean(_) => Self::TYPE_BOOLEAN,
            DetermValue::Timestamp(_) => Self::TYPE_TIMESTAMP,
            DetermValue::Extension(_) => Self::TYPE_EXTENSION,
        }
    }

    // =========================================================================
    // Encoding/Decoding
    // =========================================================================

    /// Encode this value to bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            DetermValue::Null => vec![Self::TYPE_NULL],
            DetermValue::Integer(v) => {
                let mut bytes = vec![Self::TYPE_INTEGER];
                bytes.extend_from_slice(&v.to_le_bytes());
                bytes
            }
            DetermValue::Float(v) => {
                let mut bytes = vec![Self::TYPE_FLOAT];
                bytes.extend_from_slice(&v.to_le_bytes());
                bytes
            }
            DetermValue::InlineText(data, len) => {
                let mut bytes = vec![Self::TYPE_INLINE_TEXT, *len];
                bytes.extend_from_slice(&data[..(*len as usize)]);
                bytes
            }
            DetermValue::HeapText(data) => {
                let mut bytes = vec![Self::TYPE_HEAP_TEXT];
                bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
                bytes.extend_from_slice(data);
                bytes
            }
            DetermValue::Boolean(v) => vec![Self::TYPE_BOOLEAN, *v as u8],
            DetermValue::Timestamp(v) => {
                let mut bytes = vec![Self::TYPE_TIMESTAMP];
                bytes.extend_from_slice(&v.to_le_bytes());
                bytes
            }
            DetermValue::Extension(data) => {
                let mut bytes = vec![Self::TYPE_EXTENSION];
                bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
                bytes.extend_from_slice(data);
                bytes
            }
        }
    }

    /// Return the length of the encoded value without allocating
    pub fn encoded_len(&self) -> usize {
        match self {
            DetermValue::Null => 1,
            DetermValue::Integer(_) => 9,
            DetermValue::Float(_) => 9,
            DetermValue::InlineText(_, len) => 2 + *len as usize,
            DetermValue::HeapText(data) => 5 + data.len(),
            DetermValue::Boolean(_) => 2,
            DetermValue::Timestamp(_) => 9,
            DetermValue::Extension(data) => 5 + data.len(),
        }
    }

    /// Decode a value from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::invalid_argument("cannot decode empty data"));
        }

        match data[0] {
            Self::TYPE_NULL => Ok(DetermValue::Null),
            Self::TYPE_INTEGER => {
                if data.len() < 9 {
                    return Err(Error::invalid_argument("invalid integer data"));
                }
                let bytes = [data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]];
                Ok(DetermValue::Integer(i64::from_le_bytes(bytes)))
            }
            Self::TYPE_FLOAT => {
                if data.len() < 9 {
                    return Err(Error::invalid_argument("invalid float data"));
                }
                let bytes = [data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]];
                Ok(DetermValue::Float(f64::from_le_bytes(bytes)))
            }
            Self::TYPE_INLINE_TEXT => {
                if data.len() < 2 {
                    return Err(Error::invalid_argument("invalid inline text data"));
                }
                let len = data[1] as usize;
                if data.len() < 2 + len {
                    return Err(Error::invalid_argument("invalid inline text data"));
                }
                let mut data_array = [0u8; 15];
                data_array[..len].copy_from_slice(&data[2..2 + len]);
                Ok(DetermValue::InlineText(data_array, len as u8))
            }
            Self::TYPE_HEAP_TEXT => {
                if data.len() < 5 {
                    return Err(Error::invalid_argument("invalid heap text data"));
                }
                let len = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
                if data.len() < 5 + len {
                    return Err(Error::invalid_argument("invalid heap text data"));
                }
                let data = data[5..].to_vec();
                Ok(DetermValue::HeapText(data.into()))
            }
            Self::TYPE_BOOLEAN => {
                if data.len() < 2 {
                    return Err(Error::invalid_argument("invalid boolean data"));
                }
                Ok(DetermValue::Boolean(data[1] != 0))
            }
            Self::TYPE_TIMESTAMP => {
                if data.len() < 9 {
                    return Err(Error::invalid_argument("invalid timestamp data"));
                }
                let bytes = [data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8]];
                Ok(DetermValue::Timestamp(i64::from_le_bytes(bytes)))
            }
            Self::TYPE_EXTENSION => {
                if data.len() < 5 {
                    return Err(Error::invalid_argument("invalid extension data"));
                }
                let len = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
                if data.len() < 5 + len {
                    return Err(Error::invalid_argument("invalid extension data"));
                }
                let data = data[5..].to_vec();
                Ok(DetermValue::Extension(data.into()))
            }
            _ => Err(Error::invalid_argument("unknown data type tag")),
        }
    }

    // =========================================================================
    // Merkle Hashing
    // =========================================================================

    /// Compute the Merkle hash of this value
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = MerkleHasher::new();
        match self {
            DetermValue::Null => {
                hasher.input(&[Self::TYPE_NULL]);
            }
            DetermValue::Integer(v) => {
                hasher.input(&[Self::TYPE_INTEGER]);
                hasher.input(&v.to_le_bytes());
            }
            DetermValue::Float(v) => {
                hasher.input(&[Self::TYPE_FLOAT]);
                hasher.input(&v.to_le_bytes());
            }
            DetermValue::InlineText(data, len) => {
                hasher.input(&[Self::TYPE_INLINE_TEXT]);
                hasher.input(&[*len]);
                hasher.input(&data[..(*len as usize)]);
            }
            DetermValue::HeapText(data) => {
                hasher.input(&[Self::TYPE_HEAP_TEXT]);
                hasher.input(&(data.len() as u32).to_le_bytes());
                hasher.input(data);
            }
            DetermValue::Boolean(v) => {
                hasher.input(&[Self::TYPE_BOOLEAN]);
                hasher.input(&[*v as u8]);
            }
            DetermValue::Timestamp(v) => {
                hasher.input(&[Self::TYPE_TIMESTAMP]);
                hasher.input(&v.to_le_bytes());
            }
            DetermValue::Extension(data) => {
                hasher.input(&[Self::TYPE_EXTENSION]);
                hasher.input(&(data.len() as u32).to_le_bytes());
                hasher.input(data);
            }
        }
        hasher.finalize()
    }

    /// Get the value as an integer if possible
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            DetermValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Get the value as text if possible
    pub fn as_text(&self) -> Option<String> {
        match self {
            DetermValue::InlineText(data, len) => {
                std::str::from_utf8(&data[..*len as usize]).ok().map(|s| s.to_string())
            }
            DetermValue::HeapText(data) => std::str::from_utf8(data).ok().map(|s| s.to_string()),
            _ => None,
        }
    }
}

// ============================================================================
// MerkleHasher
// ============================================================================

/// Cryptographic Merkle hasher using SHA-256
///
/// Provides secure, deterministic hashing for Merkle tree commitments.
#[derive(Debug, Clone)]
pub struct MerkleHasher {
    hasher: Sha256,
}

impl MerkleHasher {
    /// Create a new hasher with initial state
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    /// Input bytes into the hasher
    pub fn input(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Finalize and return the hash
    pub fn finalize(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

impl Default for MerkleHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determ_value_constructors() {
        let null = DetermValue::null();
        assert!(null.is_null());

        let int = DetermValue::integer(42);
        assert_eq!(int.data_type(), DetermValue::TYPE_INTEGER);

        let float = DetermValue::float(3.14);
        assert_eq!(float.data_type(), DetermValue::TYPE_FLOAT);

        let text = DetermValue::text("hello");
        assert_eq!(text.data_type(), DetermValue::TYPE_INLINE_TEXT);

        let long_text = DetermValue::text("this is a very long string that exceeds 15 bytes");
        assert_eq!(long_text.data_type(), DetermValue::TYPE_HEAP_TEXT);

        let bool = DetermValue::boolean(true);
        assert_eq!(bool.data_type(), DetermValue::TYPE_BOOLEAN);

        let ts = DetermValue::timestamp(1234567890);
        assert_eq!(ts.data_type(), DetermValue::TYPE_TIMESTAMP);
    }

    #[test]
    fn test_determ_value_encode_roundtrip() {
        let values = vec![
            DetermValue::null(),
            DetermValue::integer(42),
            DetermValue::float(3.14),
            DetermValue::text("hello"),
            DetermValue::text("this is a very long string that exceeds 15 bytes"),
            DetermValue::boolean(true),
            DetermValue::timestamp(1234567890),
        ];

        for original in values {
            let encoded = original.encode();
            let decoded = DetermValue::decode(&encoded).expect("Failed to decode");
            assert_eq!(original, decoded);
        }
    }

    #[test]
    fn test_merkle_hasher_deterministic() {
        let mut hasher1 = MerkleHasher::new();
        hasher1.input(&[1, 2, 3]);
        let hash1 = hasher1.finalize();

        let mut hasher2 = MerkleHasher::new();
        hasher2.input(&[1, 2, 3]);
        let hash2 = hasher2.finalize();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_merkle_hasher_different_inputs() {
        let mut hasher1 = MerkleHasher::new();
        hasher1.input(&[1, 2, 3]);
        let hash1 = hasher1.finalize();

        let mut hasher2 = MerkleHasher::new();
        hasher2.input(&[4, 5, 6]);
        let hash2 = hasher2.finalize();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_merkle_hasher_appends() {
        let mut hasher = MerkleHasher::new();
        hasher.input(&[1, 2, 3]);
        hasher.input(&[4, 5, 6]);
        let hash1 = hasher.finalize();

        let mut hasher2 = MerkleHasher::new();
        hasher2.input(&[1, 2, 3, 4, 5, 6]);
        let hash2 = hasher2.finalize();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_determ_value_hash() {
        let val1 = DetermValue::integer(42);
        let hash1 = val1.hash();

        let val2 = DetermValue::integer(42);
        let hash2 = val2.hash();

        assert_eq!(hash1, hash2);

        let val3 = DetermValue::integer(43);
        let hash3 = val3.hash();

        assert_ne!(hash1, hash3);
    }
}
