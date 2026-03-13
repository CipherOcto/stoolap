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

//! Deterministic Row type for blockchain SQL database

use crate::common::SmartString;
use crate::core::types::DataType;
use crate::core::value::Value;
use crate::core::Result;
use crate::determ::value::DetermValue;

/// Deterministic Row type
/// A row is a collection of deterministic values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetermRow {
    pub values: Vec<DetermValue>,
}

impl DetermRow {
    /// Create a new empty row
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Create a row from values
    pub fn from_values(values: Vec<DetermValue>) -> Self {
        Self { values }
    }

    /// Get the number of values in the row
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the row is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Compute Merkle hash of the row
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = crate::determ::value::MerkleHasher::new();
        for value in &self.values {
            hasher.input(&value.hash());
        }
        hasher.finalize()
    }

    /// Encode the row to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Write length as u32
        bytes.extend_from_slice(&(self.values.len() as u32).to_le_bytes());
        // Write each value
        for value in &self.values {
            bytes.extend_from_slice(&value.encode());
        }
        bytes
    }

    /// Decode a row from bytes
    pub fn decode(data: &[u8]) -> Result<Self> {
        use crate::core::Error;
        if data.len() < 4 {
            return Err(Error::invalid_argument("invalid row data: too short"));
        }
        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let mut values = Vec::with_capacity(len);
        let mut offset = 4;
        for _ in 0..len {
            if offset >= data.len() {
                return Err(Error::invalid_argument("invalid row data: truncated"));
            }
            // Read value length - for simple encoding we need to parse value by value
            // For now, we'll use a simpler approach where we know the type tag
            let type_tag = data[offset];
            let value_end = match type_tag {
                0 => offset + 1,      // Null
                1 => offset + 9,      // Integer
                2 => offset + 9,      // Float
                3 => offset + 1 + 15, // InlineText
                4 => offset + 5,      // HeapText (1 byte len + 4 bytes length)
                5 => offset + 2,      // Boolean
                6 => offset + 9,      // Timestamp
                7 => offset + 5,      // Extension (1 byte len + 4 bytes length)
                _ => {
                    return Err(Error::invalid_argument(format!(
                        "invalid type tag: {}",
                        type_tag
                    )))
                }
            };
            if value_end > data.len() {
                return Err(Error::invalid_argument("invalid row data: truncated value"));
            }
            let value = DetermValue::decode(&data[offset..value_end])?;
            values.push(value);
            offset = value_end;
        }
        Ok(Self { values })
    }

    /// Convert the row to a Value (for ZK proof generation)
    ///
    /// Returns the first value in the row, or Null if the row is empty.
    /// This is used when converting row data for compressed proof generation.
    pub fn to_value(&self) -> Value {
        if self.values.is_empty() {
            return Value::Null(DataType::Null);
        }

        match &self.values[0] {
            DetermValue::Null => Value::Null(DataType::Null),
            DetermValue::Integer(i) => Value::Integer(*i),
            DetermValue::Float(f) => Value::Float(*f),
            DetermValue::InlineText(bytes, len) => {
                let text = String::from_utf8(bytes[..*len as usize].to_vec()).unwrap_or_default();
                Value::Text(SmartString::from(text))
            }
            DetermValue::HeapText(data) => {
                let text = String::from_utf8(data.as_ref().to_vec()).unwrap_or_default();
                Value::Text(SmartString::from(text))
            }
            DetermValue::Boolean(b) => Value::Boolean(*b),
            DetermValue::Timestamp(ts) => {
                // Convert i64 nanoseconds to DateTime<Utc>
                use chrono::DateTime;
                use chrono::NaiveDateTime;
                use chrono::Utc;

                // Convert nanoseconds to seconds
                let secs = ts / 1_000_000_000;
                let nanos = (ts % 1_000_000_000) as u32;
                if secs >= 0 {
                    if let Some(naive) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
                        return Value::Timestamp(DateTime::<Utc>::from_utc(naive, Utc));
                    }
                }
                Value::Null(DataType::Timestamp)
            }
            DetermValue::Extension(data) => {
                // Extension data is stored as Extension in the Value type
                use crate::common::CompactArc;
                Value::Extension(CompactArc::from(data.as_ref().to_vec()))
            }
        }
    }
}

impl Default for DetermRow {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for DetermRow {
    type Output = DetermValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl std::ops::IndexMut<usize> for DetermRow {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}
