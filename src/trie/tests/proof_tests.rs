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

use crate::trie::proof::{hash_pair, merkle_root, MerkleProof};
use crate::trie::row_trie::RowTrie;
use crate::determ::{DetermRow, DetermValue};

// Tests for deprecated MerkleProof - kept for documentation purposes
#[allow(deprecated)]
#[test]
fn test_merkle_root_empty() {
    let leaves: &[[u8; 32]] = &[];
    let root = merkle_root(leaves);
    // Empty list should produce all-zero hash
    assert_eq!(root, [0u8; 32]);
}

#[allow(deprecated)]
#[test]
fn test_merkle_root_single() {
    let leaves = [[1u8; 32]];
    let root = merkle_root(&leaves);
    // Single leaf should be the root itself
    assert_eq!(root, [1u8; 32]);
}

#[allow(deprecated)]
#[test]
fn test_merkle_root_two() {
    let leaf1 = [1u8; 32];
    let leaf2 = [2u8; 32];
    let leaves = [leaf1, leaf2];
    let root = merkle_root(&leaves);
    // With SHA-256 hash, root is H(1 || 2)
    let expected = hash_pair(&leaf1, &leaf2);
    assert_eq!(root, expected);
}

#[allow(deprecated)]
#[test]
fn test_merkle_proof_verify() {
    // Create a Merkle tree with 4 leaves
    let leaf1 = [1u8; 32];
    let leaf2 = [2u8; 32];
    let leaf3 = [3u8; 32];
    let leaf4 = [4u8; 32];
    let leaves = [leaf1, leaf2, leaf3, leaf4];

    let root = merkle_root(&leaves);

    // Create a proof for leaf1 (index 0)
    // In a 4-leaf tree:
    // Level 0 (leaves): [leaf1=1], [leaf2=2], [leaf3=3], [leaf4=4]
    // Level 1: [hash(leaf1,leaf2)], [hash(leaf3,leaf4)]
    // Level 2 (root): [hash(hash(leaf1,leaf2), hash(leaf3,leaf4))]
    //
    // For leaf1 (index 0):
    // - Sibling at level 0: leaf2 = [2u8; 32]
    // - Sibling at level 1: hash(leaf3, leaf4)
    let sibling_leaf2 = leaf2;
    let sibling_34 = hash_pair(&leaf3, &leaf4);

    let mut proof = MerkleProof::new();
    proof.set_value_hash(leaf1);
    proof.add_sibling(sibling_leaf2); // Sibling at level 0
    proof.add_sibling(sibling_34); // Sibling at level 1
    proof.set_root(root);

    // For verification, we need the path (index in binary)
    // leaf1 is at index 0 (binary: 00), path is [0, 0] (left, left)
    proof.set_path(vec![0, 0]);

    assert!(proof.verify());
}

#[test]
fn test_row_trie_get_hexary_proof_single_row() {
    let mut trie = RowTrie::new();
    let row = DetermRow::from_values(vec![DetermValue::integer(42)]);
    let (root, _) = trie.insert(1, row.clone());

    let proof = trie.get_hexary_proof(1);
    assert!(proof.is_some());

    let proof = proof.unwrap();
    assert_eq!(proof.root, root);
    assert_eq!(proof.value_hash, row.hash());
    assert!(proof.verify());
}

#[test]
fn test_row_trie_get_hexary_proof_nonexistent() {
    let trie = RowTrie::new();
    let proof = trie.get_hexary_proof(999);
    assert!(proof.is_none());
}

#[test]
fn test_row_trie_get_hexary_proof_multiple_rows() {
    let mut trie = RowTrie::new();

    // Insert rows that diverge at different nibble positions
    // Row 256 has nibbles [0,0,1,0,...] (first byte is 0, second byte starts with 0)
    // Row 1 has nibbles [0,1,0,0,...]
    let row1 = DetermRow::from_values(vec![DetermValue::integer(1)]);
    let row256 = DetermRow::from_values(vec![DetermValue::integer(256)]);

    let (_root1, _) = trie.insert(1, row1);
    let (root2, _) = trie.insert(256, row256.clone());

    // Verify row 256 exists
    let retrieved = trie.get(256);
    assert!(retrieved.is_some(), "Row 256 should exist");

    // Get proof for row 256
    let proof = trie.get_hexary_proof(256);
    assert!(proof.is_some());

    let proof = proof.unwrap();
    assert_eq!(proof.value_hash, row256.hash());

    // Debug: check levels and path
    println!("Proof levels: {}", proof.levels.len());
    println!("Path nibbles: {:?}", proof.path);

    assert!(proof.verify(), "Proof should verify");
}

#[test]
fn test_row_trie_get_hexary_proof_branch_siblings() {
    let mut trie = RowTrie::new();

    // Insert two rows that will be in the same branch
    let row1 = DetermRow::from_values(vec![DetermValue::integer(1)]);
    let row16 = DetermRow::from_values(vec![DetermValue::integer(16)]);

    let (_root, _) = trie.insert(1, row1);
    let (root2, _) = trie.insert(16, row16.clone());

    // Verify both rows exist
    assert!(trie.get(1).is_some(), "Row 1 should exist");
    assert!(trie.get(16).is_some(), "Row 16 should exist");

    // Get proof for row 16
    let proof = trie.get_hexary_proof(16);
    assert!(proof.is_some(), "Proof for row 16 should exist");

    let proof = proof.unwrap();
    assert_eq!(proof.value_hash, row16.hash());
    assert_eq!(proof.root, root2);

    // Debug
    println!("Row 16 - Proof levels: {}", proof.levels.len());
    println!("Row 16 - Path nibbles: {:?}", proof.path);
    println!("Row 16 - Unpacked path: {:?}", {
        let mut result = Vec::new();
        for &byte in &proof.path {
            result.push(byte & 0x0F);
            result.push((byte >> 4) & 0x0F);
        }
        result
    });
    if !proof.levels.is_empty() {
        println!("Row 16 - Level 0 bitmap: {:b}", proof.levels[0].bitmap);
    }

    assert!(proof.verify(), "Proof should verify");
}

#[test]
fn test_row_trie_get_hexary_proof_verify_end_to_end() {
    let mut trie = RowTrie::new();

    // Insert a row and get a proof
    let row = DetermRow::from_values(vec![DetermValue::integer(42)]);
    let (root, _) = trie.insert(100, row.clone());

    let proof = trie.get_hexary_proof(100);
    assert!(proof.is_some());

    let proof = proof.unwrap();
    assert_eq!(proof.root, root);
    assert_eq!(proof.value_hash, row.hash());

    // Verify the proof
    assert!(proof.verify());
}
