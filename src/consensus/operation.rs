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

//! Operation types for blockchain consensus
//!
//! This module defines the operation types that are recorded in the blockchain's
//! operation log. Each operation represents a database change that needs to be
//! replicated across nodes.

use std::fmt;

/// Operation-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpError {
    /// Invalid operation data
    InvalidOperation(String),
    /// Failed to decode operation
    DecodeError(String),
    /// Invalid operation type byte
    InvalidOperationType(u8),
    /// Invalid index type byte
    InvalidIndexType(u8),
    /// Invalid data type byte
    InvalidDataType(u8),
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            OpError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
            OpError::InvalidOperationType(b) => write!(f, "Invalid operation type: {}", b),
            OpError::InvalidIndexType(b) => write!(f, "Invalid index type: {}", b),
            OpError::InvalidDataType(b) => write!(f, "Invalid data type: {}", b),
        }
    }
}

impl std::error::Error for OpError {}

/// Result type for Operation operations
pub type OpResult<T> = Result<T, OpError>;

/// Index types for consensus operations
///
/// These are the index types supported by the blockchain SQL database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IndexType {
    /// B-tree index for range queries
    BTree = 0,
    /// Hash index for equality lookups
    Hash = 1,
    /// Bitmap index for low-cardinality columns
    Bitmap = 2,
    /// HNSW index for vector similarity search
    Hnsw = 3,
}

impl IndexType {
    /// Convert to u8 for serialization
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create IndexType from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(IndexType::BTree),
            1 => Some(IndexType::Hash),
            2 => Some(IndexType::Bitmap),
            3 => Some(IndexType::Hnsw),
            _ => None,
        }
    }
}

/// Data types for consensus operations
///
/// These are the data types supported by the blockchain SQL database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DataType {
    /// 64-bit signed integer
    Integer = 0,
    /// 64-bit floating point number
    Float = 1,
    /// UTF-8 text string
    Text = 2,
    /// Boolean true/false
    Boolean = 3,
    /// Timestamp with timezone
    Timestamp = 4,
    /// Vector of f32 values
    Vector = 5,
    /// JSON document
    Json = 6,
}

impl DataType {
    /// Convert to u8 for serialization
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create DataType from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(DataType::Integer),
            1 => Some(DataType::Float),
            2 => Some(DataType::Text),
            3 => Some(DataType::Boolean),
            4 => Some(DataType::Timestamp),
            5 => Some(DataType::Vector),
            6 => Some(DataType::Json),
            _ => None,
        }
    }
}

/// Column definition for consensus operations
///
/// Defines a column in a table schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnDef {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: DataType,
    /// Whether the column can be NULL
    pub nullable: bool,
}

impl ColumnDef {
    /// Create a new column definition
    pub fn new(name: String, data_type: DataType, nullable: bool) -> Self {
        Self {
            name,
            data_type,
            nullable,
        }
    }
}

/// Operation type byte identifiers
#[repr(u8)]
enum OperationType {
    Insert = 0,
    Update = 1,
    Delete = 2,
    CreateTable = 3,
    DropTable = 4,
    CreateIndex = 5,
    DropIndex = 6,
}

impl OperationType {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(OperationType::Insert),
            1 => Some(OperationType::Update),
            2 => Some(OperationType::Delete),
            3 => Some(OperationType::CreateTable),
            4 => Some(OperationType::DropTable),
            5 => Some(OperationType::CreateIndex),
            6 => Some(OperationType::DropIndex),
            _ => None,
        }
    }
}

/// Database operation for blockchain consensus
///
/// Represents a single database operation that is recorded in the blockchain's
/// operation log. Operations are replicated across nodes for consistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Insert a new row
    Insert {
        table_name: String,
        row_id: i64,
        row_data: Vec<u8>,
    },

    /// Update a specific column in a row
    Update {
        table_name: String,
        row_id: i64,
        column_index: usize,
        old_value: Option<Vec<u8>>,
        new_value: Vec<u8>,
    },

    /// Delete a row
    Delete { table_name: String, row_id: i64 },

    /// Create a new table
    CreateTable {
        table_name: String,
        schema: Vec<ColumnDef>,
    },

    /// Drop a table
    DropTable { table_name: String },

    /// Create an index
    CreateIndex {
        table_name: String,
        index_name: String,
        index_type: IndexType,
        columns: Vec<usize>,
    },

    /// Drop an index
    DropIndex {
        table_name: String,
        index_name: String,
    },
}

impl Operation {
    /// Compute the hash of this operation
    ///
    /// Uses a simple XOR hashing approach for now. In production, this should
    /// be replaced with a proper cryptographic hash function like SHA-256.
    pub fn hash(&self) -> [u8; 32] {
        let mut result = [0u8; 32];

        // Start with the operation type
        let op_type_byte = self.operation_type().as_u8();
        result[0] = result[0].wrapping_add(op_type_byte);

        // Hash the encoded representation
        let encoded = self.encode();
        for (i, &byte) in encoded.iter().enumerate() {
            result[i % 32] ^= byte;
        }

        // Add some mixing based on position
        for i in 0..32 {
            result[i] = result[i].wrapping_add(i as u8).wrapping_mul(31);
        }

        result
    }

    /// Get the operation type for this operation
    fn operation_type(&self) -> OperationType {
        match self {
            Operation::Insert { .. } => OperationType::Insert,
            Operation::Update { .. } => OperationType::Update,
            Operation::Delete { .. } => OperationType::Delete,
            Operation::CreateTable { .. } => OperationType::CreateTable,
            Operation::DropTable { .. } => OperationType::DropTable,
            Operation::CreateIndex { .. } => OperationType::CreateIndex,
            Operation::DropIndex { .. } => OperationType::DropIndex,
        }
    }

    /// Serialize the operation to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Write operation type
        buffer.push(self.operation_type().as_u8());

        match self {
            Operation::Insert {
                table_name,
                row_id,
                row_data,
            } => {
                encode_string(&mut buffer, table_name);
                encode_i64(&mut buffer, *row_id);
                encode_bytes(&mut buffer, row_data);
            }

            Operation::Update {
                table_name,
                row_id,
                column_index,
                old_value,
                new_value,
            } => {
                encode_string(&mut buffer, table_name);
                encode_i64(&mut buffer, *row_id);
                encode_usize(&mut buffer, *column_index);
                encode_option_bytes(&mut buffer, old_value);
                encode_bytes(&mut buffer, new_value);
            }

            Operation::Delete { table_name, row_id } => {
                encode_string(&mut buffer, table_name);
                encode_i64(&mut buffer, *row_id);
            }

            Operation::CreateTable { table_name, schema } => {
                encode_string(&mut buffer, table_name);
                encode_vec(&mut buffer, schema, |buf, col| {
                    encode_string(buf, &col.name);
                    buf.push(col.data_type.as_u8());
                    buf.push(col.nullable as u8);
                });
            }

            Operation::DropTable { table_name } => {
                encode_string(&mut buffer, table_name);
            }

            Operation::CreateIndex {
                table_name,
                index_name,
                index_type,
                columns,
            } => {
                encode_string(&mut buffer, table_name);
                encode_string(&mut buffer, index_name);
                buffer.push(index_type.as_u8());
                encode_vec(&mut buffer, columns, |buf, col| {
                    encode_usize(buf, *col);
                });
            }

            Operation::DropIndex {
                table_name,
                index_name,
            } => {
                encode_string(&mut buffer, table_name);
                encode_string(&mut buffer, index_name);
            }
        }

        buffer
    }

    /// Deserialize an operation from bytes
    pub fn decode(data: &[u8]) -> OpResult<Self> {
        if data.is_empty() {
            return Err(OpError::DecodeError("Empty data".to_string()));
        }

        let op_type = OperationType::from_u8(data[0])
            .ok_or_else(|| OpError::InvalidOperationType(data[0]))?;

        let mut pos = 1;

        let operation = match op_type {
            OperationType::Insert => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (row_id, new_pos) = decode_i64(data, pos)?;
                pos = new_pos;
                let (row_data, new_pos) = decode_bytes(data, pos)?;
                pos = new_pos;

                Operation::Insert {
                    table_name,
                    row_id,
                    row_data,
                }
            }

            OperationType::Update => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (row_id, new_pos) = decode_i64(data, pos)?;
                pos = new_pos;
                let (column_index, new_pos) = decode_usize(data, pos)?;
                pos = new_pos;
                let (old_value, new_pos) = decode_option_bytes(data, pos)?;
                pos = new_pos;
                let (new_value, new_pos) = decode_bytes(data, pos)?;
                pos = new_pos;

                Operation::Update {
                    table_name,
                    row_id,
                    column_index,
                    old_value,
                    new_value,
                }
            }

            OperationType::Delete => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (row_id, new_pos) = decode_i64(data, pos)?;
                pos = new_pos;

                Operation::Delete { table_name, row_id }
            }

            OperationType::CreateTable => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (schema, new_pos) = decode_vec(data, pos, |data, pos| {
                    let (name, pos) = decode_string(data, pos)?;
                    if pos >= data.len() {
                        return Err(OpError::DecodeError("Incomplete ColumnDef".to_string()));
                    }
                    let data_type = DataType::from_u8(data[pos])
                        .ok_or_else(|| OpError::InvalidDataType(data[pos]))?;
                    let pos = pos + 1;
                    if pos >= data.len() {
                        return Err(OpError::DecodeError("Incomplete ColumnDef".to_string()));
                    }
                    let nullable = data[pos] != 0;
                    let pos = pos + 1;
                    Ok((
                        ColumnDef {
                            name,
                            data_type,
                            nullable,
                        },
                        pos,
                    ))
                })?;
                pos = new_pos;

                Operation::CreateTable { table_name, schema }
            }

            OperationType::DropTable => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;

                Operation::DropTable { table_name }
            }

            OperationType::CreateIndex => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (index_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                if pos >= data.len() {
                    return Err(OpError::DecodeError("Incomplete CreateIndex".to_string()));
                }
                let index_type = IndexType::from_u8(data[pos])
                    .ok_or_else(|| OpError::InvalidIndexType(data[pos]))?;
                pos += 1;
                let (columns, new_pos) =
                    decode_vec(data, pos, decode_usize)?;
                pos = new_pos;

                Operation::CreateIndex {
                    table_name,
                    index_name,
                    index_type,
                    columns,
                }
            }

            OperationType::DropIndex => {
                let (table_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;
                let (index_name, new_pos) = decode_string(data, pos)?;
                pos = new_pos;

                Operation::DropIndex {
                    table_name,
                    index_name,
                }
            }
        };

        Ok(operation)
    }
}

// ============================================================================
// Encoding helper functions
// ============================================================================

fn encode_string(buffer: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    encode_usize(buffer, bytes.len());
    buffer.extend_from_slice(bytes);
}

fn decode_string(data: &[u8], pos: usize) -> OpResult<(String, usize)> {
    let (len, pos) = decode_usize(data, pos)?;
    if pos + len > data.len() {
        return Err(OpError::DecodeError(format!(
            "String length {} exceeds remaining data {}",
            len,
            data.len() - pos
        )));
    }
    let s = String::from_utf8(data[pos..pos + len].to_vec())
        .map_err(|e| OpError::DecodeError(format!("Invalid UTF-8: {}", e)))?;
    Ok((s, pos + len))
}

fn encode_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    encode_usize(buffer, bytes.len());
    buffer.extend_from_slice(bytes);
}

fn decode_bytes(data: &[u8], pos: usize) -> OpResult<(Vec<u8>, usize)> {
    let (len, pos) = decode_usize(data, pos)?;
    if pos + len > data.len() {
        return Err(OpError::DecodeError(format!(
            "Bytes length {} exceeds remaining data {}",
            len,
            data.len() - pos
        )));
    }
    Ok((data[pos..pos + len].to_vec(), pos + len))
}

fn encode_option_bytes(buffer: &mut Vec<u8>, bytes: &Option<Vec<u8>>) {
    match bytes {
        Some(b) => {
            buffer.push(1);
            encode_bytes(buffer, b);
        }
        None => {
            buffer.push(0);
        }
    }
}

fn decode_option_bytes(data: &[u8], pos: usize) -> OpResult<(Option<Vec<u8>>, usize)> {
    if pos >= data.len() {
        return Err(OpError::DecodeError("Incomplete option".to_string()));
    }
    match data[pos] {
        0 => Ok((None, pos + 1)),
        1 => {
            let (bytes, new_pos) = decode_bytes(data, pos + 1)?;
            Ok((Some(bytes), new_pos))
        }
        _ => Err(OpError::DecodeError("Invalid option flag".to_string())),
    }
}

fn encode_i64(buffer: &mut Vec<u8>, value: i64) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn decode_i64(data: &[u8], pos: usize) -> OpResult<(i64, usize)> {
    if pos + 8 > data.len() {
        return Err(OpError::DecodeError("Incomplete i64".to_string()));
    }
    let bytes = &data[pos..pos + 8];
    let value = i64::from_be_bytes(bytes.try_into().unwrap());
    Ok((value, pos + 8))
}

fn encode_usize(buffer: &mut Vec<u8>, value: usize) {
    // Encode as u64 for fixed width
    let value = value as u64;
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn decode_usize(data: &[u8], pos: usize) -> OpResult<(usize, usize)> {
    if pos + 8 > data.len() {
        return Err(OpError::DecodeError("Incomplete usize".to_string()));
    }
    let bytes = &data[pos..pos + 8];
    let value = u64::from_be_bytes(bytes.try_into().unwrap()) as usize;
    Ok((value, pos + 8))
}

fn encode_vec<T, F>(buffer: &mut Vec<u8>, vec: &[T], mut encode_fn: F)
where
    F: FnMut(&mut Vec<u8>, &T),
{
    encode_usize(buffer, vec.len());
    for item in vec {
        encode_fn(buffer, item);
    }
}

fn decode_vec<T, F>(data: &[u8], pos: usize, mut decode_fn: F) -> OpResult<(Vec<T>, usize)>
where
    F: FnMut(&[u8], usize) -> OpResult<(T, usize)>,
{
    let (len, mut pos) = decode_usize(data, pos)?;
    let mut vec = Vec::with_capacity(len);
    for _ in 0..len {
        let (item, new_pos) = decode_fn(data, pos)?;
        vec.push(item);
        pos = new_pos;
    }
    Ok((vec, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_type_conversion() {
        assert_eq!(IndexType::BTree.as_u8(), 0);
        assert_eq!(IndexType::Hash.as_u8(), 1);
        assert_eq!(IndexType::Bitmap.as_u8(), 2);
        assert_eq!(IndexType::Hnsw.as_u8(), 3);

        assert_eq!(IndexType::from_u8(0), Some(IndexType::BTree));
        assert_eq!(IndexType::from_u8(1), Some(IndexType::Hash));
        assert_eq!(IndexType::from_u8(2), Some(IndexType::Bitmap));
        assert_eq!(IndexType::from_u8(3), Some(IndexType::Hnsw));
        assert_eq!(IndexType::from_u8(99), None);
    }

    #[test]
    fn test_data_type_conversion() {
        assert_eq!(DataType::Integer.as_u8(), 0);
        assert_eq!(DataType::Float.as_u8(), 1);
        assert_eq!(DataType::Text.as_u8(), 2);
        assert_eq!(DataType::Boolean.as_u8(), 3);
        assert_eq!(DataType::Timestamp.as_u8(), 4);
        assert_eq!(DataType::Vector.as_u8(), 5);
        assert_eq!(DataType::Json.as_u8(), 6);

        assert_eq!(DataType::from_u8(0), Some(DataType::Integer));
        assert_eq!(DataType::from_u8(1), Some(DataType::Float));
        assert_eq!(DataType::from_u8(2), Some(DataType::Text));
        assert_eq!(DataType::from_u8(3), Some(DataType::Boolean));
        assert_eq!(DataType::from_u8(4), Some(DataType::Timestamp));
        assert_eq!(DataType::from_u8(5), Some(DataType::Vector));
        assert_eq!(DataType::from_u8(6), Some(DataType::Json));
        assert_eq!(DataType::from_u8(99), None);
    }

    #[test]
    fn test_column_def() {
        let col = ColumnDef::new("id".to_string(), DataType::Integer, false);
        assert_eq!(col.name, "id");
        assert_eq!(col.data_type, DataType::Integer);
        assert!(!col.nullable);
    }

    #[test]
    fn test_encode_decode_insert() {
        let original = Operation::Insert {
            table_name: "users".to_string(),
            row_id: 42,
            row_data: vec![1, 2, 3, 4],
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_update() {
        let original = Operation::Update {
            table_name: "users".to_string(),
            row_id: 42,
            column_index: 2,
            old_value: Some(vec![1, 2]),
            new_value: vec![3, 4],
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_delete() {
        let original = Operation::Delete {
            table_name: "users".to_string(),
            row_id: 42,
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_create_table() {
        let original = Operation::CreateTable {
            table_name: "users".to_string(),
            schema: vec![
                ColumnDef::new("id".to_string(), DataType::Integer, false),
                ColumnDef::new("name".to_string(), DataType::Text, false),
                ColumnDef::new("email".to_string(), DataType::Text, true),
            ],
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_drop_table() {
        let original = Operation::DropTable {
            table_name: "users".to_string(),
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_create_index() {
        let original = Operation::CreateIndex {
            table_name: "users".to_string(),
            index_name: "idx_users_name".to_string(),
            index_type: IndexType::BTree,
            columns: vec![0, 1],
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_drop_index() {
        let original = Operation::DropIndex {
            table_name: "users".to_string(),
            index_name: "idx_users_name".to_string(),
        };

        let encoded = original.encode();
        let decoded = Operation::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_hash_consistency() {
        let op1 = Operation::Insert {
            table_name: "test".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        };

        let op2 = Operation::Insert {
            table_name: "test".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        };

        assert_eq!(op1.hash(), op2.hash());
    }

    #[test]
    fn test_hash_different_operations() {
        let op1 = Operation::Insert {
            table_name: "test".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        };

        let op2 = Operation::Insert {
            table_name: "test".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 4], // Different data
        };

        assert_ne!(op1.hash(), op2.hash());
    }

    #[test]
    fn test_empty_decode() {
        let result = Operation::decode(&[]);
        assert!(matches!(result, Err(OpError::DecodeError(_))));
    }

    #[test]
    fn test_invalid_operation_type() {
        let mut data = vec![99u8]; // Invalid operation type
        data.extend_from_slice(&0u64.to_be_bytes()); // Add some data for string length
        data.extend_from_slice(b"test");

        let result = Operation::decode(&data);
        assert!(matches!(result, Err(OpError::InvalidOperationType(99))));
    }
}
