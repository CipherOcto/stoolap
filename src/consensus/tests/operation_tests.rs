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

// Tests for consensus operation types

use crate::consensus::operation::{Operation, IndexType, ColumnDef, DataType};

/// Test that insert operations produce consistent hashes
#[test]
fn test_operation_insert_hash() {
    let op1 = Operation::Insert {
        table_name: "users".to_string(),
        row_id: 1,
        row_data: vec![1, 2, 3, 4],
    };

    let op2 = Operation::Insert {
        table_name: "users".to_string(),
        row_id: 1,
        row_data: vec![1, 2, 3, 4],
    };

    // Same operation should produce same hash
    assert_eq!(op1.hash(), op2.hash());

    // Different data should produce different hash
    let op3 = Operation::Insert {
        table_name: "users".to_string(),
        row_id: 1,
        row_data: vec![1, 2, 3, 5],
    };
    assert_ne!(op1.hash(), op3.hash());
}

/// Test that update operations produce consistent hashes
#[test]
fn test_operation_update_hash() {
    let op1 = Operation::Update {
        table_name: "users".to_string(),
        row_id: 1,
        column_index: 2,
        old_value: Some(vec![1, 2]),
        new_value: vec![3, 4],
    };

    let op2 = Operation::Update {
        table_name: "users".to_string(),
        row_id: 1,
        column_index: 2,
        old_value: Some(vec![1, 2]),
        new_value: vec![3, 4],
    };

    // Same operation should produce same hash
    assert_eq!(op1.hash(), op2.hash());

    // Different new_value should produce different hash
    let op3 = Operation::Update {
        table_name: "users".to_string(),
        row_id: 1,
        column_index: 2,
        old_value: Some(vec![1, 2]),
        new_value: vec![3, 5],
    };
    assert_ne!(op1.hash(), op3.hash());
}

/// Test that operations can be encoded and decoded
#[test]
fn test_operation_encode_roundtrip() {
    let operations = vec![
        Operation::Insert {
            table_name: "users".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3, 4],
        },
        Operation::Update {
            table_name: "users".to_string(),
            row_id: 2,
            column_index: 1,
            old_value: Some(vec![5]),
            new_value: vec![6],
        },
        Operation::Delete {
            table_name: "users".to_string(),
            row_id: 3,
        },
    ];

    for original_op in operations {
        // Encode the operation
        let encoded = original_op.encode();

        // Decode it back
        let decoded_op = Operation::decode(&encoded).expect("Failed to decode operation");

        // The decoded operation should match the original
        assert_eq!(original_op, decoded_op);

        // Hash should be preserved
        assert_eq!(original_op.hash(), decoded_op.hash());
    }
}

/// Test all operation types can be created and hashed
#[test]
fn test_all_operation_types() {
    let schema = vec![
        ColumnDef {
            name: "id".to_string(),
            data_type: DataType::Integer,
            nullable: false,
        },
        ColumnDef {
            name: "name".to_string(),
            data_type: DataType::Text,
            nullable: false,
        },
    ];

    let operations = vec![
        Operation::Insert {
            table_name: "test_table".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        },
        Operation::Update {
            table_name: "test_table".to_string(),
            row_id: 1,
            column_index: 0,
            old_value: Some(vec![1]),
            new_value: vec![2],
        },
        Operation::Delete {
            table_name: "test_table".to_string(),
            row_id: 1,
        },
        Operation::CreateTable {
            table_name: "test_table".to_string(),
            schema: schema.clone(),
        },
        Operation::DropTable {
            table_name: "test_table".to_string(),
        },
        Operation::CreateIndex {
            table_name: "test_table".to_string(),
            index_name: "idx_test".to_string(),
            index_type: IndexType::BTree,
            columns: vec![0, 1],
        },
        Operation::DropIndex {
            table_name: "test_table".to_string(),
            index_name: "idx_test".to_string(),
        },
    ];

    // All operations should produce valid hashes (non-zero)
    for op in &operations {
        let hash = op.hash();
        // Hash should not be all zeros
        let has_nonzero = hash.iter().any(|&b| b != 0);
        assert!(has_nonzero, "Operation produced all-zero hash: {:?}", op);
    }

    // Test all IndexType variants
    let index_types = vec![
        IndexType::BTree,
        IndexType::Hash,
        IndexType::Bitmap,
        IndexType::Hnsw,
    ];

    for index_type in index_types {
        let op = Operation::CreateIndex {
            table_name: "test".to_string(),
            index_name: "idx".to_string(),
            index_type,
            columns: vec![0],
        };
        let hash = op.hash();
        let has_nonzero = hash.iter().any(|&b| b != 0);
        assert!(has_nonzero, "IndexType {:?} produced all-zero hash", index_type);
    }

    // Test all DataType variants
    let data_types = vec![
        DataType::Integer,
        DataType::Float,
        DataType::Text,
        DataType::Boolean,
        DataType::Timestamp,
        DataType::Vector,
        DataType::Json,
    ];

    for data_type in data_types {
        let schema = vec![ColumnDef {
            name: "col".to_string(),
            data_type,
            nullable: false,
        }];
        let op = Operation::CreateTable {
            table_name: "test".to_string(),
            schema,
        };
        let hash = op.hash();
        let has_nonzero = hash.iter().any(|&b| b != 0);
        assert!(has_nonzero, "DataType {:?} produced all-zero hash", data_type);
    }
}
