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

#[test]
fn test_merkle_root_empty() {
    let leaves: &[[u8; 32]] = &[];
    let root = merkle_root(leaves);
    // Empty list should produce all-zero hash
    assert_eq!(root, [0u8; 32]);
}

#[test]
fn test_merkle_root_single() {
    let leaves = [[1u8; 32]];
    let root = merkle_root(&leaves);
    // Single leaf should be the root itself
    assert_eq!(root, [1u8; 32]);
}

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
