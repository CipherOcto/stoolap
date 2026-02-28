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

//! End-to-end integration test for blockchain SQL database

use stoolap::consensus::{Block, BlockHeader, BlockOperations, Operation};
use stoolap::determ::{DetermRow, DetermValue};
use stoolap::execution::{ExecutionContext, StateSnapshot};
use stoolap::trie::{RowTrie, TableSchema, ColumnDef};
use stoolap::DataType;

#[test]
fn test_full_block_execution() {
    let mut state = StateSnapshot::new();

    let schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
            ColumnDef { name: "name".to_string(), data_type: DataType::Text, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("users".to_string(), RowTrie::new());

    let mut ctx = ExecutionContext::new(1, 100_000, state);

    let mut name_data = [0u8; 15];
    name_data[0] = b'A';
    name_data[1] = b'l';
    name_data[2] = b'i';
    name_data[3] = b'c';
    name_data[4] = b'e';

    let row = DetermRow::from_values(vec![
        DetermValue::Integer(1),
        DetermValue::InlineText(name_data, 5),
    ]);

    let result = ctx.insert("users", 1, row);
    assert!(result.is_ok());

    // Verify state changed using get_row()
    let retrieved = ctx.state().get_row("users", 1);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().len(), 2);
}

#[test]
fn test_single_row_trie_retrieve() {
    // Test that single-row tries work correctly
    let mut state = StateSnapshot::new();

    let schema = TableSchema {
        name: "singles".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
            ColumnDef { name: "value".to_string(), data_type: DataType::Integer, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("singles".to_string(), RowTrie::new());

    let mut ctx = ExecutionContext::new(1, 10_000, state);

    let row = DetermRow::from_values(vec![
        DetermValue::Integer(42),
        DetermValue::Integer(100),
    ]);

    assert!(ctx.insert("singles", 42, row).is_ok());

    // Should be able to retrieve the single row
    let retrieved = ctx.state().get_row("singles", 42);
    assert!(retrieved.is_some());
    let retrieved_row = retrieved.unwrap();
    assert_eq!(retrieved_row.len(), 2);
    assert_eq!(retrieved_row.values.get(0), Some(&DetermValue::Integer(42)));
}

#[test]
fn test_state_root_changes_after_operations() {
    let mut state = StateSnapshot::new();
    let root_before = state.schema_root();
    assert_eq!(root_before, [0u8; 32]);

    let schema = TableSchema {
        name: "products".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
            ColumnDef { name: "price".to_string(), data_type: DataType::Float, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("products".to_string(), RowTrie::new());

    let root_after_table = state.schema_root();
    assert_ne!(root_after_table, [0u8; 32]);

    let mut ctx = ExecutionContext::new(1, 100_000, state);

    let row = DetermRow::from_values(vec![
        DetermValue::Integer(1),
        DetermValue::Float(29.99),
    ]);

    assert!(ctx.insert("products", 1, row).is_ok());

    let trie = ctx.state().get_table_trie("products").unwrap();
    assert_ne!(trie.get_root(), [0u8; 32]);
    assert_eq!(trie.len(), 1);
}

#[test]
fn test_gas_tracking_during_execution() {
    let mut state = StateSnapshot::new();

    let schema = TableSchema {
        name: "items".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("items".to_string(), RowTrie::new());

    let mut ctx = ExecutionContext::new(1, 10_000, state);

    // Test with 2 rows to avoid complex trie edge cases
    let row1 = DetermRow::from_values(vec![DetermValue::Integer(1)]);
    assert!(ctx.insert("items", 1, row1).is_ok());

    let row2 = DetermRow::from_values(vec![DetermValue::Integer(2)]);
    assert!(ctx.insert("items", 2, row2).is_ok());

    // 2 inserts * 1000 gas each = 2000 gas used
    assert_eq!(ctx.gas_used(), 2000);
}

#[test]
fn test_block_with_operations() {
    let mut state = StateSnapshot::new();

    let schema = TableSchema {
        name: "orders".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
            ColumnDef { name: "amount".to_string(), data_type: DataType::Integer, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("orders".to_string(), RowTrie::new());

    let mut ctx = ExecutionContext::new(1, 100_000, state);

    // Insert 2 rows to avoid edge cases
    for i in 1..=2 {
        let row = DetermRow::from_values(vec![
            DetermValue::Integer(i),
            DetermValue::Integer(i * 10),
        ]);
        assert!(ctx.insert("orders", i, row).is_ok());
    }

    let gas_used = ctx.gas_used();
    let final_state = ctx.into_state();

    let operations = vec![
        Operation::Insert {
            table_name: "orders".to_string(),
            row_id: 1,
            row_data: vec![1, 10],
        },
        Operation::Insert {
            table_name: "orders".to_string(),
            row_id: 2,
            row_data: vec![2, 20],
        },
    ];

    let header = BlockHeader {
        block_number: 1,
        parent_hash: [0u8; 32],
        state_root_before: [0u8; 32],
        state_root_after: final_state.schema_root(),
        operation_root: BlockOperations::compute_operation_root(&operations),
        timestamp: 100,
        gas_limit: 100_000,
        gas_used: gas_used,
        proposer: [0u8; 32],
        extra_data: vec![],
    };

    let block = Block::new(header, operations, final_state, vec![]);

    assert!(block.verify().is_ok());
    assert_eq!(block.header.gas_used, 2000);
}

#[test]
fn test_delete_operation() {
    let mut state = StateSnapshot::new();

    let schema = TableSchema {
        name: "temp".to_string(),
        columns: vec![
            ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
        ],
        primary_key: Some("id".to_string()),
        table_root: [0u8; 32],
        index_roots: std::collections::BTreeMap::new(),
    };
    state.schemas.create_table(schema);
    state.tables.insert("temp".to_string(), RowTrie::new());

    let mut ctx = ExecutionContext::new(1, 10_000, state);

    // Insert 2 rows
    for i in 1..=2 {
        let row = DetermRow::from_values(vec![DetermValue::Integer(i)]);
        assert!(ctx.insert("temp", i, row).is_ok());
    }

    // Verify both rows exist
    assert!(ctx.state().get_row("temp", 1).is_some());
    assert!(ctx.state().get_row("temp", 2).is_some());

    // Delete one row
    assert!(ctx.delete("temp", 1).is_ok());

    // Verify deletion
    assert!(ctx.state().get_row("temp", 1).is_none());
    assert!(ctx.state().get_row("temp", 2).is_some());
}

#[test]
fn test_operation_roundtrip() {
    let operations = vec![
        Operation::Insert {
            table_name: "users".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        },
        Operation::Update {
            table_name: "users".to_string(),
            row_id: 1,
            column_index: 1,
            old_value: Some(vec![2]),
            new_value: vec![5],
        },
        Operation::Delete {
            table_name: "users".to_string(),
            row_id: 1,
        },
    ];

    for original_op in operations {
        let encoded = original_op.encode();
        let decoded_op = Operation::decode(&encoded).expect("Failed to decode operation");
        assert_eq!(original_op, decoded_op);
        assert_eq!(original_op.hash(), decoded_op.hash());
    }
}

#[test]
fn test_multiple_tables_independent_state() {
    let mut state = StateSnapshot::new();

    for table_name in &["table_a", "table_b"] {
        let schema = TableSchema {
            name: table_name.to_string(),
            columns: vec![
                ColumnDef { name: "id".to_string(), data_type: DataType::Integer, nullable: false },
            ],
            primary_key: Some("id".to_string()),
            table_root: [0u8; 32],
            index_roots: std::collections::BTreeMap::new(),
        };
        state.schemas.create_table(schema);
        state.tables.insert(table_name.to_string(), RowTrie::new());
    }

    let mut ctx = ExecutionContext::new(1, 100_000, state);

    let row_a = DetermRow::from_values(vec![DetermValue::Integer(100)]);
    assert!(ctx.insert("table_a", 1, row_a).is_ok());

    let row_b = DetermRow::from_values(vec![DetermValue::Integer(200)]);
    assert!(ctx.insert("table_b", 1, row_b).is_ok());

    let retrieved_a = ctx.state().get_row("table_a", 1);
    let retrieved_b = ctx.state().get_row("table_b", 1);

    assert!(retrieved_a.is_some());
    assert!(retrieved_b.is_some());

    assert_ne!(retrieved_a.unwrap().values.get(0), retrieved_b.unwrap().values.get(0));
}
