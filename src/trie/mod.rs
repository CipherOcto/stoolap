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

//! Merkle trie structures for state verification
//!
//! This module provides Merkle tree implementations for verifying
//! database state in a blockchain context.
//!
//! # HexaryProof
//!
//! The primary proof type is [`HexaryProof`], designed for 16-way hexary tries
//! like [`RowTrie`]. It provides compact proofs and efficient verification.
//!
//! ## Example
//!
//! ```no_run
//! use stoolap::trie::{RowTrie, HexaryProof};
//! use stoolap::determ::{DetermRow, DetermValue};
//!
//! let mut trie = RowTrie::new();
//! let row = DetermRow::from_values(vec![DetermValue::integer(42)]);
//! trie.insert(42, row);
//!
//! // Generate and verify a proof
//! if let Some(proof) = trie.get_hexary_proof(42) {
//!     assert!(proof.verify());
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`proof`] - Hexary proof types and verification
//! - [`row_trie`] - Row-storage hexary trie
//! - [`schema_trie`] - Schema metadata trie

pub mod proof;
pub mod row_trie;
pub mod schema_trie;

// Primary exports for HexaryProof system
pub use proof::{HexaryProof, ProofLevel};
pub use row_trie::{RowNode, RowTrie, StateDiff};
pub use schema_trie::{ColumnDef, SchemaTrie, TableSchema};

// Re-export hexary proof utility functions for convenience
pub use proof::{
    hash_16_children, pack_nibbles, reconstruct_children, unpack_nibbles, SerializationError,
    SolanaSerialize,
};

#[cfg(test)]
mod tests;
