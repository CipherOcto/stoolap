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

//! SchemaTrie tests

use crate::core::DataType;
use crate::trie::schema_trie::{ColumnDef, SchemaTrie, TableSchema};
use std::collections::BTreeMap;

#[test]
fn test_schema_trie_create_table() {
    let mut trie = SchemaTrie::new();

    // Create a table schema
    let mut schema = TableSchema::new("users");
    schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    schema.add_column(ColumnDef::new("name", DataType::Text, true));
    schema.add_column(ColumnDef::new("email", DataType::Text, true));
    schema.primary_key = Some("id".to_string());

    // Add table to trie
    trie.create_table(schema);

    // Verify table exists
    assert!(trie.table_exists("users"));

    // Verify we can retrieve the table
    let retrieved = trie.get_table("users");
    assert!(retrieved.is_some());
    let table = retrieved.unwrap();
    assert_eq!(table.name, "users");
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.columns[0].name, "id");
    assert_eq!(table.columns[0].data_type, DataType::Integer);
    assert!(!table.columns[0].nullable);
    assert_eq!(table.primary_key, Some("id".to_string()));
}

#[test]
fn test_schema_trie_get_table() {
    let mut trie = SchemaTrie::new();

    // Create multiple tables
    let mut users_schema = TableSchema::new("users");
    users_schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    users_schema.primary_key = Some("id".to_string());

    let mut posts_schema = TableSchema::new("posts");
    posts_schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    posts_schema.add_column(ColumnDef::new("user_id", DataType::Integer, false));
    posts_schema.add_column(ColumnDef::new("title", DataType::Text, false));
    posts_schema.primary_key = Some("id".to_string());

    trie.create_table(users_schema);
    trie.create_table(posts_schema);

    // Test getting existing tables
    let users = trie.get_table("users");
    assert!(users.is_some());
    assert_eq!(users.unwrap().columns.len(), 1);

    let posts = trie.get_table("posts");
    assert!(posts.is_some());
    assert_eq!(posts.unwrap().columns.len(), 3);

    // Test getting non-existent table
    let nonexistent = trie.get_table("comments");
    assert!(nonexistent.is_none());
}

#[test]
fn test_schema_trie_drop_table() {
    let mut trie = SchemaTrie::new();

    // Create tables
    let mut schema1 = TableSchema::new("users");
    schema1.add_column(ColumnDef::new("id", DataType::Integer, false));

    let mut schema2 = TableSchema::new("posts");
    schema2.add_column(ColumnDef::new("id", DataType::Integer, false));

    trie.create_table(schema1);
    trie.create_table(schema2);

    // Verify both exist
    assert!(trie.table_exists("users"));
    assert!(trie.table_exists("posts"));

    // Drop one table
    trie.drop_table("users");

    // Verify it's gone
    assert!(!trie.table_exists("users"));
    assert!(trie.table_exists("posts"));
    assert!(trie.get_table("users").is_none());

    // Dropping non-existent table should not panic
    trie.drop_table("nonexistent");
}

#[test]
fn test_schema_trie_list_tables() {
    let mut trie = SchemaTrie::new();

    // Empty trie should return empty list
    assert!(trie.list_tables().is_empty());

    // Add tables in non-alphabetical order
    let mut schema1 = TableSchema::new("zebra");
    schema1.add_column(ColumnDef::new("id", DataType::Integer, false));

    let mut schema2 = TableSchema::new("apple");
    schema2.add_column(ColumnDef::new("id", DataType::Integer, false));

    let mut schema3 = TableSchema::new("banana");
    schema3.add_column(ColumnDef::new("id", DataType::Integer, false));

    trie.create_table(schema1);
    trie.create_table(schema2);
    trie.create_table(schema3);

    // List should be sorted alphabetically (BTreeMap property)
    let tables = trie.list_tables();
    assert_eq!(tables.len(), 3);
    assert_eq!(tables[0], "apple");
    assert_eq!(tables[1], "banana");
    assert_eq!(tables[2], "zebra");
}

#[test]
fn test_schema_trie_get_root() {
    let trie = SchemaTrie::new();
    let root = trie.get_root();

    // Empty trie should have a root hash (all zeros)
    assert_eq!(root, [0u8; 32]);
}

#[test]
fn test_schema_trie_rehash() {
    let mut trie = SchemaTrie::new();

    // Get initial root
    let root1 = trie.get_root();

    // Add a table
    let mut schema = TableSchema::new("users");
    schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    trie.create_table(schema);

    // Root should change after rehash
    trie.rehash();
    let root2 = trie.get_root();
    assert_ne!(root1, root2);

    // Add another table
    let mut schema2 = TableSchema::new("posts");
    schema2.add_column(ColumnDef::new("id", DataType::Integer, false));
    trie.create_table(schema2);

    trie.rehash();
    let root3 = trie.get_root();
    assert_ne!(root2, root3);
}

#[test]
fn test_table_schema_with_index_roots() {
    let mut schema = TableSchema::new("users");
    schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    schema.add_column(ColumnDef::new("email", DataType::Text, true));

    // Set table root
    schema.table_root = [1u8; 32];

    // Add index roots
    let mut index_roots = BTreeMap::new();
    index_roots.insert("idx_users_email".to_string(), [2u8; 32]);
    index_roots.insert("idx_users_id".to_string(), [3u8; 32]);
    schema.index_roots = index_roots;

    assert_eq!(schema.table_root, [1u8; 32]);
    assert_eq!(schema.index_roots.len(), 2);
    assert_eq!(schema.index_roots["idx_users_email"], [2u8; 32]);
    assert_eq!(schema.index_roots["idx_users_id"], [3u8; 32]);
}

#[test]
fn test_column_def_all_data_types() {
    let types = vec![
        DataType::Integer,
        DataType::Float,
        DataType::Text,
        DataType::Boolean,
        DataType::Timestamp,
        DataType::Vector,
        DataType::Json,
    ];

    for dt in types {
        let col = ColumnDef::new("test_col", dt, true);
        assert_eq!(col.data_type, dt);
        assert!(col.nullable);
    }
}

#[test]
fn test_schema_trie_multiple_operations() {
    let mut trie = SchemaTrie::new();

    // Create table
    let mut schema = TableSchema::new("test");
    schema.add_column(ColumnDef::new("id", DataType::Integer, false));
    trie.create_table(schema);
    assert!(trie.table_exists("test"));

    // Try to create same table again (should replace)
    let mut schema2 = TableSchema::new("test");
    schema2.add_column(ColumnDef::new("id", DataType::Integer, false));
    schema2.add_column(ColumnDef::new("name", DataType::Text, true));
    trie.create_table(schema2);

    let retrieved = trie.get_table("test");
    assert_eq!(retrieved.unwrap().columns.len(), 2);

    // Drop table
    trie.drop_table("test");
    assert!(!trie.table_exists("test"));
}
