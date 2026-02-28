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

use crate::consensus::{Block, BlockHeader, BlockOperations};
use crate::execution::StateSnapshot;

#[test]
fn test_block_verify_empty_block() {
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

    let operations = BlockOperations {
        block_number: 1,
        operations: vec![],
        state_root_before: [0u8; 32],
        state_root_after: [0u8; 32],
        operation_root: [0u8; 32],
    };

    let block = Block {
        header,
        operations,
        state_commitment: state,
        signatures: vec![],
    };

    assert!(block.verify().is_ok());
}

#[test]
fn test_block_operations_compute_root() {
    let ops = vec![];
    let root = BlockOperations::compute_operation_root(&ops);
    assert_eq!(root, [0u8; 32]);

    // For empty operations, root is all zeros
    // For single operation, root is hash of that operation
    // We'll verify the basic function works
}

#[test]
fn test_block_header_fields() {
    let header = BlockHeader {
        block_number: 42,
        parent_hash: [1u8; 32],
        state_root_before: [2u8; 32],
        state_root_after: [3u8; 32],
        operation_root: [4u8; 32],
        timestamp: 12345,
        gas_limit: 500_000,
        gas_used: 1000,
        proposer: [99u8; 32],
        extra_data: vec![1, 2, 3],
    };

    assert_eq!(header.block_number, 42);
    assert_eq!(header.parent_hash, [1u8; 32]);
    assert_eq!(header.timestamp, 12345);
}
