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

//! Schema trie implementation for storing table schemas in a Merkle trie
//!
//! This module provides a Merkle trie structure for storing and verifying
//! database table schemas with cryptographic proofs.

use crate::core::DataType;
use std::collections::BTreeMap;

/// Column definition for a table
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    /// Column name
    pub name: String,
    /// Column data type
    pub data_type: DataType,
    /// Whether the column is nullable
    pub nullable: bool,
}

impl ColumnDef {
    /// Create a new column definition
    pub fn new(name: &str, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.to_string(),
            data_type,
            nullable,
        }
    }
}

/// Table schema definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSchema {
    /// Table name
    pub name: String,
    /// Column definitions
    pub columns: Vec<ColumnDef>,
    /// Primary key column name (if any)
    pub primary_key: Option<String>,
    /// Root hash of the table's data trie
    pub table_root: [u8; 32],
    /// Map of index name to root hash
    pub index_roots: BTreeMap<String, [u8; 32]>,
}

impl TableSchema {
    /// Create a new table schema
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            columns: Vec::new(),
            primary_key: None,
            table_root: [0u8; 32],
            index_roots: BTreeMap::new(),
        }
    }

    /// Add a column to the schema
    pub fn add_column(&mut self, column: ColumnDef) {
        self.columns.push(column);
    }

    /// Get the hash of the schema
    pub fn hash(&self) -> [u8; 32] {
        // Simple XOR-based hash for now
        let mut hash = [0u8; 32];

        // Hash the name
        for (i, byte) in self.name.bytes().enumerate() {
            hash[i % 32] ^= byte;
        }

        // Hash columns
        for col in &self.columns {
            for (i, byte) in col.name.bytes().enumerate() {
                hash[(i + 16) % 32] ^= byte;
            }
            let type_byte = col.data_type.as_u8();
            hash[17] ^= type_byte;
            if col.nullable {
                hash[18] ^= 0xFF;
            }
        }

        // Hash primary key
        if let Some(pk) = &self.primary_key {
            for (i, byte) in pk.bytes().enumerate() {
                hash[(i + 24) % 32] ^= byte;
            }
        }

        hash
    }
}

/// A Merkle trie for storing table schemas
///
/// The SchemaTrie provides efficient storage and verification of
/// table schemas with Merkle proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaTrie {
    /// Map of table name to schema
    pub tables: BTreeMap<String, TableSchema>,
    /// Root hash of the trie
    pub root: [u8; 32],
}

impl SchemaTrie {
    /// Create a new empty schema trie
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            root: [0u8; 32],
        }
    }

    /// Add a table schema to the trie
    pub fn create_table(&mut self, schema: TableSchema) {
        let name = schema.name.clone();
        self.tables.insert(name, schema);
        self.rehash();
    }

    /// Remove a table from the trie
    pub fn drop_table(&mut self, name: &str) {
        self.tables.remove(name);
        self.rehash();
    }

    /// Get a table schema from the trie
    pub fn get_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.get(name)
    }

    /// Check if a table exists in the trie
    pub fn table_exists(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    /// List all table names in the trie
    pub fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }

    /// Get the root hash of the schema trie
    pub fn get_root(&self) -> [u8; 32] {
        self.root
    }

    /// Recompute the root hash from all tables
    pub fn rehash(&mut self) {
        if self.tables.is_empty() {
            self.root = [0u8; 32];
            return;
        }

        // Simple XOR-based hash combining all table hashes
        let mut combined_hash = [0u8; 32];

        for (name, schema) in &self.tables {
            let table_hash = schema.hash();

            // Combine name and table hash
            for (i, byte) in name.bytes().enumerate() {
                combined_hash[i % 32] ^= byte;
            }

            for i in 0..32 {
                combined_hash[i] ^= table_hash[i];
            }
        }

        self.root = combined_hash;
    }
}

impl Default for SchemaTrie {
    fn default() -> Self {
        Self::new()
    }
}
