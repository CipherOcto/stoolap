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

//! Block structure for blockchain consensus
//!
//! This module defines the block structure that contains operations in the
//! blockchain's operation log.

use crate::consensus::operation::Operation;
use crate::execution::StateSnapshot;
use sha2::{Digest, Sha256};

/// Block header contains metadata about the block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    /// Block number in the chain
    pub block_number: u64,

    /// Hash of the parent block
    pub parent_hash: [u8; 32],

    /// State root before executing this block
    pub state_root_before: [u8; 32],

    /// State root after executing this block
    pub state_root_after: [u8; 32],

    /// Merkle root of operations in this block
    pub operation_root: [u8; 32],

    /// Timestamp when block was proposed
    pub timestamp: u64,

    /// Maximum gas allowed in this block
    pub gas_limit: u64,

    /// Total gas used by operations in this block
    pub gas_used: u64,

    /// Address of the block proposer
    pub proposer: [u8; 32],

    /// Additional data (e.g., validator signatures)
    pub extra_data: Vec<u8>,
}

impl BlockHeader {
    /// Compute the hash of this header
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.block_number.to_be_bytes());
        hasher.update(self.parent_hash);
        hasher.update(self.state_root_before);
        hasher.update(self.state_root_after);
        hasher.update(self.operation_root);
        hasher.update(self.timestamp.to_be_bytes());
        hasher.update(self.gas_limit.to_be_bytes());
        hasher.update(self.gas_used.to_be_bytes());
        hasher.update(self.proposer);
        hasher.update(&self.extra_data);
        hasher.finalize().into()
    }
}

/// Operations contained in a block with their commitment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockOperations {
    /// Block number
    pub block_number: u64,

    /// List of operations in this block
    pub operations: Vec<Operation>,

    /// State root before executing operations
    pub state_root_before: [u8; 32],

    /// State root after executing operations
    pub state_root_after: [u8; 32],

    /// Merkle root of operations
    pub operation_root: [u8; 32],
}

impl BlockOperations {
    /// Compute the Merkle root of operations
    pub fn compute_operation_root(operations: &[Operation]) -> [u8; 32] {
        if operations.is_empty() {
            return [0u8; 32];
        }

        // Hash each operation
        let mut hashes: Vec<[u8; 32]> = operations.iter().map(|op| op.hash()).collect();

        // Build Merkle tree bottom-up
        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                if chunk.len() == 2 {
                    let mut hasher = Sha256::new();
                    hasher.update(chunk[0]);
                    hasher.update(chunk[1]);
                    next_level.push(hasher.finalize().into());
                } else {
                    // Odd number - hash with self
                    let mut hasher = Sha256::new();
                    hasher.update(chunk[0]);
                    hasher.update(chunk[0]);
                    next_level.push(hasher.finalize().into());
                }
            }

            hashes = next_level;
        }

        hashes[0]
    }

    /// Create a new BlockOperations by computing the operation root
    pub fn new(
        block_number: u64,
        operations: Vec<Operation>,
        state_root_before: [u8; 32],
        state_root_after: [u8; 32],
    ) -> Self {
        let operation_root = Self::compute_operation_root(&operations);

        Self {
            block_number,
            operations,
            state_root_before,
            state_root_after,
            operation_root,
        }
    }

    /// Verify that the operation root matches the operations
    pub fn verify_operation_root(&self) -> bool {
        self.operation_root == Self::compute_operation_root(&self.operations)
    }
}

/// A block in the blockchain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Block header with metadata
    pub header: BlockHeader,

    /// Operations in this block
    pub operations: BlockOperations,

    /// State commitment after executing this block
    pub state_commitment: StateSnapshot,

    /// Signatures from validators
    pub signatures: Vec<[u8; 64]>,
}

impl Block {
    /// Verify the block's integrity
    ///
    /// Checks:
    /// - Operation root matches operations
    /// - State roots are consistent
    /// - Gas used does not exceed gas limit
    pub fn verify(&self) -> Result<(), BlockError> {
        // Verify operation root
        if self.header.operation_root != self.operations.operation_root {
            return Err(BlockError::InvalidOperationRoot);
        }

        // Verify operations internally
        if !self.operations.verify_operation_root() {
            return Err(BlockError::InvalidOperationRoot);
        }

        // Verify state roots match
        if self.header.state_root_before != self.operations.state_root_before {
            return Err(BlockError::StateRootMismatch);
        }

        if self.header.state_root_after != self.operations.state_root_after {
            return Err(BlockError::StateRootMismatch);
        }

        // Verify gas used does not exceed gas limit
        if self.header.gas_used > self.header.gas_limit {
            return Err(BlockError::GasLimitExceeded {
                used: self.header.gas_used,
                limit: self.header.gas_limit,
            });
        }

        // Verify block numbers match
        if self.header.block_number != self.operations.block_number {
            return Err(BlockError::BlockNumberMismatch);
        }

        Ok(())
    }

    /// Get the hash of this block (hash of header)
    pub fn hash(&self) -> [u8; 32] {
        self.header.hash()
    }

    /// Create a new block
    pub fn new(
        header: BlockHeader,
        operations: Vec<Operation>,
        state_commitment: StateSnapshot,
        signatures: Vec<[u8; 64]>,
    ) -> Self {
        let block_operations = BlockOperations::new(
            header.block_number,
            operations,
            header.state_root_before,
            header.state_root_after,
        );

        Self {
            header,
            operations: block_operations,
            state_commitment,
            signatures,
        }
    }
}

/// Errors that can occur during block verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockError {
    /// Operation root does not match the operations
    InvalidOperationRoot,

    /// State roots are inconsistent
    StateRootMismatch,

    /// Gas used exceeds gas limit
    GasLimitExceeded { used: u64, limit: u64 },

    /// Block number mismatch between header and operations
    BlockNumberMismatch,
}

impl std::fmt::Display for BlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockError::InvalidOperationRoot => write!(f, "Invalid operation root"),
            BlockError::StateRootMismatch => write!(f, "State root mismatch"),
            BlockError::GasLimitExceeded { used, limit } => {
                write!(f, "Gas limit exceeded: {} > {}", used, limit)
            }
            BlockError::BlockNumberMismatch => write!(f, "Block number mismatch"),
        }
    }
}

impl std::error::Error for BlockError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_hash() {
        let header1 = BlockHeader {
            block_number: 1,
            parent_hash: [0u8; 32],
            state_root_before: [0u8; 32],
            state_root_after: [0u8; 32],
            operation_root: [0u8; 32],
            timestamp: 100,
            gas_limit: 1_000_000,
            gas_used: 0,
            proposer: [0u8; 32],
            extra_data: vec![],
        };

        let header2 = header1.clone();

        // Same header should produce same hash
        assert_eq!(header1.hash(), header2.hash());

        // Different data should produce different hash
        let header3 = BlockHeader {
            block_number: 2,
            extra_data: header1.extra_data.clone(),
            ..header1
        };

        assert_ne!(header1.hash(), header3.hash());
    }

    #[test]
    fn test_compute_operation_root_empty() {
        let root = BlockOperations::compute_operation_root(&[]);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn test_compute_operation_root_single() {
        use crate::consensus::operation::Operation;

        let op = Operation::Insert {
            table_name: "test".to_string(),
            row_id: 1,
            row_data: vec![1, 2, 3],
        };

        let root = BlockOperations::compute_operation_root(&[op.clone()]);
        let expected = op.hash();

        // For single operation, the root is just the hash of that operation
        assert_eq!(root, expected);
    }

    #[test]
    fn test_block_verify_success() {
        let state = StateSnapshot::new();
        let header = BlockHeader {
            block_number: 1,
            parent_hash: [0u8; 32],
            state_root_before: [0u8; 32],
            state_root_after: [0u8; 32],
            operation_root: [0u8; 32],
            timestamp: 100,
            gas_limit: 1_000_000,
            gas_used: 0,
            proposer: [0u8; 32],
            extra_data: vec![],
        };

        let block = Block::new(header, vec![], state, vec![]);

        assert!(block.verify().is_ok());
    }

    #[test]
    fn test_block_verify_gas_limit_exceeded() {
        let state = StateSnapshot::new();
        let header = BlockHeader {
            block_number: 1,
            parent_hash: [0u8; 32],
            state_root_before: [0u8; 32],
            state_root_after: [0u8; 32],
            operation_root: [0u8; 32],
            timestamp: 100,
            gas_limit: 100,
            gas_used: 200, // Exceeds limit
            proposer: [0u8; 32],
            extra_data: vec![],
        };

        let operations =
            BlockOperations::new(1, vec![], header.state_root_before, header.state_root_after);

        let block = Block {
            header,
            operations,
            state_commitment: state,
            signatures: vec![],
        };

        assert!(matches!(
            block.verify(),
            Err(BlockError::GasLimitExceeded {
                used: 200,
                limit: 100
            })
        ));
    }
}
