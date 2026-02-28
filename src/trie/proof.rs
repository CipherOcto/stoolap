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

//! Merkle proof types for state verification
//!
//! This module provides types and functions for creating and verifying
//! Merkle proofs, which allow efficient verification of data inclusion
//! in a Merkle tree without requiring the full tree.

use sha2::{Digest, Sha256};

/// A Merkle proof that demonstrates inclusion of a value in a Merkle tree
///
/// # Fields
///
/// * `value_hash` - The hash of the value being proven
/// * `siblings` - The sibling hashes needed to reconstruct the root
/// * `root` - The expected Merkle root
/// * `path` - The path (indices) from root to leaf in the tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// Hash of the value being proven
    pub value_hash: [u8; 32],
    /// Sibling hashes along the path to root
    pub siblings: Vec<[u8; 32]>,
    /// Expected Merkle root
    pub root: [u8; 32],
    /// Path from root to leaf (as bit vector)
    pub path: Vec<u8>,
}

/// A single level in a hexary Merkle proof
///
/// Contains the sibling information needed to verify one level
/// of a 16-way hexary trie.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProofLevel {
    /// 16-bit bitmap indicating which child positions have hashes
    /// Bit i is set if child at position i (0-15) exists
    pub bitmap: u16,

    /// Sibling hashes (non-path children only)
    /// Order corresponds to set bits in bitmap (excluding path bit)
    pub siblings: Vec<[u8; 32]>,
}

/// Hexary Merkle proof for 16-way trie verification
///
/// This proof type is designed for hexary (16-way branching) tries like
/// RowTrie, providing compact proofs and efficient verification.
///
/// # Fields
///
/// * `value_hash` - Hash of the value being proven (row hash)
/// * `levels` - Proof levels from root to leaf
/// * `root` - Expected Merkle root
/// * `path` - Nibble path (2 nibbles packed per byte, LSB first)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexaryProof {
    /// Hash of the value being proven
    pub value_hash: [u8; 32],

    /// Proof levels from root to leaf
    pub levels: Vec<ProofLevel>,

    /// Expected Merkle root
    pub root: [u8; 32],

    /// Nibble path (2 nibbles packed per byte, LSB first)
    /// If path length is odd, last byte uses only low nibble
    pub path: Vec<u8>,
}

impl HexaryProof {
    /// Create a new empty hexary proof
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let proof = HexaryProof::new();
    /// assert_eq!(proof.value_hash, [0u8; 32]);
    /// assert!(proof.levels.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            value_hash: [0u8; 32],
            levels: Vec::new(),
            root: [0u8; 32],
            path: Vec::new(),
        }
    }

    /// Create a proof with a value hash
    ///
    /// # Arguments
    ///
    /// * `value_hash` - The hash of the value to prove
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let proof = HexaryProof::with_value_hash([1u8; 32]);
    /// assert_eq!(proof.value_hash, [1u8; 32]);
    /// assert!(proof.levels.is_empty());
    /// ```
    pub fn with_value_hash(value_hash: [u8; 32]) -> Self {
        Self {
            value_hash,
            levels: Vec::new(),
            root: [0u8; 32],
            path: Vec::new(),
        }
    }

    /// Add a proof level
    ///
    /// # Arguments
    ///
    /// * `bitmap` - 16-bit bitmap indicating which child positions have hashes
    /// * `siblings` - Sibling hashes for non-path children
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let mut proof = HexaryProof::new();
    /// proof.add_level(0b1000000000001000, vec![[2u8; 32]]);
    /// assert_eq!(proof.levels.len(), 1);
    /// ```
    pub fn add_level(&mut self, bitmap: u16, siblings: Vec<[u8; 32]>) {
        self.levels.push(ProofLevel { bitmap, siblings });
    }

    /// Set the root hash
    ///
    /// # Arguments
    ///
    /// * `root` - The expected root hash
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let mut proof = HexaryProof::new();
    /// proof.set_root([1u8; 32]);
    /// assert_eq!(proof.root, [1u8; 32]);
    /// ```
    pub fn set_root(&mut self, root: [u8; 32]) {
        self.root = root;
    }

    /// Set the path
    ///
    /// # Arguments
    ///
    /// * `path` - The nibble path (2 nibbles packed per byte, LSB first)
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let mut proof = HexaryProof::new();
    /// proof.set_path(vec![0x35]);
    /// assert_eq!(proof.path, vec![0x35]);
    /// ```
    pub fn set_path(&mut self, path: Vec<u8>) {
        self.path = path;
    }
}

impl Default for HexaryProof {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleProof {
    /// Create a new empty Merkle proof
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::MerkleProof;
    ///
    /// let proof = MerkleProof::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new proof with a value hash
    ///
    /// # Arguments
    ///
    /// * `value_hash` - The hash of the value to prove
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::MerkleProof;
    ///
    /// let proof = MerkleProof::with_value([1u8; 32]);
    /// assert_eq!(proof.value_hash, [1u8; 32]);
    /// ```
    pub fn with_value(value_hash: [u8; 32]) -> Self {
        Self {
            value_hash,
            ..Default::default()
        }
    }

    /// Set the value hash
    ///
    /// # Arguments
    ///
    /// * `value_hash` - The hash of the value to prove
    pub fn set_value_hash(&mut self, value_hash: [u8; 32]) {
        self.value_hash = value_hash;
    }

    /// Add a sibling hash to the proof
    ///
    /// # Arguments
    ///
    /// * `sibling` - A sibling hash to add
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::MerkleProof;
    ///
    /// let mut proof = MerkleProof::new();
    /// proof.add_sibling([2u8; 32]);
    /// assert_eq!(proof.siblings.len(), 1);
    /// ```
    pub fn add_sibling(&mut self, sibling: [u8; 32]) {
        self.siblings.push(sibling);
    }

    /// Set the expected Merkle root
    ///
    /// # Arguments
    ///
    /// * `root` - The expected root hash
    pub fn set_root(&mut self, root: [u8; 32]) {
        self.root = root;
    }

    /// Set the path from root to leaf
    ///
    /// # Arguments
    ///
    /// * `path` - The path as a vector of bits (0 for left, 1 for right)
    pub fn set_path(&mut self, path: Vec<u8>) {
        self.path = path;
    }

    /// Verify the Merkle proof
    ///
    /// Returns true if the proof is valid, i.e., reconstructing the Merkle root
    /// from the value hash and siblings matches the expected root.
    ///
    /// # Returns
    ///
    /// `true` if the proof is valid, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::{MerkleProof, hash_pair};
    ///
    /// let mut proof = MerkleProof::new();
    /// proof.set_value_hash([1u8; 32]);
    /// proof.add_sibling([2u8; 32]);
    /// let expected_root = hash_pair(&[1u8; 32], &[2u8; 32]);
    /// proof.set_root(expected_root);
    /// proof.set_path(vec![0]);
    ///
    /// assert!(proof.verify());
    /// ```
    pub fn verify(&self) -> bool {
        if self.siblings.is_empty() {
            // If no siblings, value should equal root directly
            return self.value_hash == self.root;
        }

        let mut current = self.value_hash;

        for (i, &sibling) in self.siblings.iter().enumerate() {
            // Check path bit to determine if current is left or right child
            // If path doesn't have enough bits, assume we're at the correct position
            let is_left = if i < self.path.len() {
                self.path[i] == 0
            } else {
                true // Default to left if path is exhausted
            };

            current = if is_left {
                // Current is left child, sibling is right
                hash_pair(&current, &sibling)
            } else {
                // Current is right child, sibling is left
                hash_pair(&sibling, &current)
            };
        }

        current == self.root
    }
}

impl Default for MerkleProof {
    fn default() -> Self {
        Self {
            value_hash: [0u8; 32],
            siblings: Vec::new(),
            root: [0u8; 32],
            path: Vec::new(),
        }
    }
}

/// Compute the Merkle root from a list of leaf hashes
///
/// This function builds a complete Merkle tree from the leaves and returns
/// the root hash. For an empty list, returns all zeros.
///
/// # Arguments
///
/// * `leaves` - Slice of leaf hashes
///
/// # Returns
///
/// The Merkle root hash
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::{merkle_root, hash_pair};
///
/// let leaves = [
///     [1u8; 32],
///     [2u8; 32],
/// ];
/// let root = merkle_root(&leaves);
/// // With SHA-256 hash, root is H(1 || 2)
/// assert_eq!(root, hash_pair(&[1u8; 32], &[2u8; 32]));
/// ```
pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }

    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current_level: Vec<[u8; 32]> = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in current_level.chunks(2) {
            if chunk.len() == 2 {
                next_level.push(hash_pair(&chunk[0], &chunk[1]));
            } else {
                // Odd number of nodes, promote the last one
                next_level.push(chunk[0]);
            }
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Hash two values together using SHA-256
///
/// Combines two hashes by concatenating them and computing SHA-256.
/// This provides cryptographic security for Merkle tree proofs.
///
/// # Arguments
///
/// * `a` - First hash (left child)
/// * `b` - Second hash (right child)
///
/// # Returns
///
/// The SHA-256 hash of the concatenated inputs
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::hash_pair;
///
/// let a = [1u8; 32];
/// let b = [2u8; 32];
/// let result = hash_pair(&a, &b);
/// // Result is SHA-256(a || b)
/// assert_ne!(result, [0u8; 32]);
/// assert!(result.len() == 32);
/// ```
pub fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(a);
    hasher.update(b);
    hasher.finalize().into()
}

/// Pack nibbles into bytes (2 nibbles per byte, LSB first)
///
/// Each byte contains two nibbles: low nibble first, then high nibble.
/// If the input has odd length, the final byte has the nibble in the low position.
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::pack_nibbles;
///
/// let packed = pack_nibbles(&[5, 12]);
/// assert_eq!(packed, vec![0xC5]); // 5 in low nibble, 12 (0xC) in high
/// ```
pub fn pack_nibbles(nibbles: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity((nibbles.len() + 1) / 2);

    for chunk in nibbles.chunks(2) {
        let low = chunk[0] & 0x0F;
        let high = if chunk.len() > 1 {
            (chunk[1] & 0x0F) << 4
        } else {
            0
        };
        result.push(low | high);
    }

    result
}

/// Unpack bytes into nibbles (2 nibbles per byte, LSB first)
///
/// Returns exactly 2 nibbles per input byte, preserving all nibbles including zeros.
/// The caller is responsible for knowing the expected length of the original path.
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::unpack_nibbles;
///
/// let nibbles = unpack_nibbles(&[0xC5]);
/// assert_eq!(nibbles, vec![5, 12]);
/// ```
pub fn unpack_nibbles(packed: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(packed.len() * 2);

    for &byte in packed {
        result.push(byte & 0x0F); // Low nibble
        result.push((byte >> 4) & 0x0F); // High nibble
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pair_simple() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let result = hash_pair(&a, &b);
        // SHA-256 produces a cryptographic hash, not a simple value
        assert_ne!(result, [0u8; 32]);
        assert_ne!(result, a);
        assert_ne!(result, b);
    }

    #[test]
    fn test_hash_pair_commutative() {
        // Note: SHA-256 is NOT commutative - hash_pair(a, b) != hash_pair(b, a)
        // because H(a || b) != H(b || a)
        let a = [1u8; 32];
        let b = [2u8; 32];
        let result1 = hash_pair(&a, &b);
        let result2 = hash_pair(&b, &a);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_hash_pair_deterministic() {
        let a = [5u8; 32];
        let b = [7u8; 32];
        let result1 = hash_pair(&a, &b);
        let result2 = hash_pair(&a, &b);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_merkle_proof_default() {
        let proof = MerkleProof::default();
        assert_eq!(proof.value_hash, [0u8; 32]);
        assert!(proof.siblings.is_empty());
        assert_eq!(proof.root, [0u8; 32]);
        assert!(proof.path.is_empty());
    }

    #[test]
    fn test_merkle_proof_builder() {
        let mut proof = MerkleProof::new();
        proof.set_value_hash([42u8; 32]);
        proof.add_sibling([1u8; 32]);
        proof.add_sibling([2u8; 32]);
        proof.set_root(hash_pair(&[1u8; 32], &[42u8; 32]));
        proof.set_path(vec![0, 1]);

        assert_eq!(proof.value_hash, [42u8; 32]);
        assert_eq!(proof.siblings.len(), 2);
        assert_eq!(proof.siblings[0], [1u8; 32]);
        assert_eq!(proof.siblings[1], [2u8; 32]);
        assert_eq!(proof.path, vec![0, 1]);
    }

    #[test]
    fn test_merkle_proof_with_value() {
        let proof = MerkleProof::with_value([99u8; 32]);
        assert_eq!(proof.value_hash, [99u8; 32]);
        assert!(proof.siblings.is_empty());
    }

    #[test]
    fn test_merkle_proof_verify_single() {
        let mut proof = MerkleProof::new();
        proof.set_value_hash([1u8; 32]);
        proof.set_root([1u8; 32]);

        assert!(proof.verify());
    }

    #[test]
    fn test_merkle_proof_verify_single_fail() {
        let mut proof = MerkleProof::new();
        proof.set_value_hash([1u8; 32]);
        proof.set_root([2u8; 32]);

        assert!(!proof.verify());
    }

    #[test]
    fn test_merkle_proof_verify_two_leaves() {
        // leaf1 = [1], leaf2 = [2]
        // root = SHA-256(1 || 2)
        let mut proof = MerkleProof::new();
        proof.set_value_hash([1u8; 32]);
        proof.add_sibling([2u8; 32]);
        let expected_root = hash_pair(&[1u8; 32], &[2u8; 32]);
        proof.set_root(expected_root);
        proof.set_path(vec![0]); // leaf1 is on the left

        assert!(proof.verify());
    }

    #[test]
    fn test_merkle_root_empty_list() {
        let leaves: &[[u8; 32]] = &[];
        let root = merkle_root(leaves);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_root_single_element() {
        let leaves = [[5u8; 32]];
        let root = merkle_root(&leaves);
        assert_eq!(root, [5u8; 32]);
    }

    #[test]
    fn test_merkle_root_two_elements() {
        let leaves = [[1u8; 32], [2u8; 32]];
        let root = merkle_root(&leaves);
        let expected = hash_pair(&[1u8; 32], &[2u8; 32]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_four_elements() {
        // Level 0: [1], [2], [3], [4]
        // Level 1: hash_pair([1], [2]), hash_pair([3], [4])
        // Level 2: hash_pair(hash_pair([1], [2]), hash_pair([3], [4]))
        let leaves = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let root = merkle_root(&leaves);
        let level1_left = hash_pair(&[1u8; 32], &[2u8; 32]);
        let level1_right = hash_pair(&[3u8; 32], &[4u8; 32]);
        let expected = hash_pair(&level1_left, &level1_right);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_three_elements() {
        // Level 0: [1], [2], [3]
        // Level 1: hash_pair([1], [2]), [3] (odd, promoted)
        // Level 2: hash_pair(hash_pair([1], [2]), [3])
        let leaves = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = merkle_root(&leaves);
        let level1_left = hash_pair(&[1u8; 32], &[2u8; 32]);
        let expected = hash_pair(&level1_left, &[3u8; 32]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_hexary_proof_basic_structure() {
        let proof = HexaryProof {
            value_hash: [1u8; 32],
            levels: vec![
                ProofLevel {
                    bitmap: 0b1000000000001000,
                    siblings: vec![[2u8; 32]],
                }
            ],
            root: [3u8; 32],
            path: vec![0x35], // nibbles [5]
        };

        assert_eq!(proof.value_hash, [1u8; 32]);
        assert_eq!(proof.levels.len(), 1);
        assert_eq!(proof.levels[0].bitmap, 0b1000000000001000);
        assert_eq!(proof.path, vec![0x35]);
    }

    #[test]
    fn test_hexary_proof_new() {
        let proof = HexaryProof::new();
        assert_eq!(proof.value_hash, [0u8; 32]);
        assert!(proof.levels.is_empty());
        assert_eq!(proof.root, [0u8; 32]);
        assert!(proof.path.is_empty());
    }

    #[test]
    fn test_hexary_proof_with_value_hash() {
        let proof = HexaryProof::with_value_hash([42u8; 32]);
        assert_eq!(proof.value_hash, [42u8; 32]);
        assert!(proof.levels.is_empty());
        assert_eq!(proof.root, [0u8; 32]);
        assert!(proof.path.is_empty());
    }

    #[test]
    fn test_hexary_proof_add_level() {
        let mut proof = HexaryProof::new();
        proof.add_level(0b1000000000001000, vec![[2u8; 32]]);
        assert_eq!(proof.levels.len(), 1);
        assert_eq!(proof.levels[0].bitmap, 0b1000000000001000);
        assert_eq!(proof.levels[0].siblings.len(), 1);
        assert_eq!(proof.levels[0].siblings[0], [2u8; 32]);
    }

    #[test]
    fn test_hexary_proof_set_root() {
        let mut proof = HexaryProof::new();
        proof.set_root([99u8; 32]);
        assert_eq!(proof.root, [99u8; 32]);
    }

    #[test]
    fn test_hexary_proof_set_path() {
        let mut proof = HexaryProof::new();
        proof.set_path(vec![0x35, 0xAB]);
        assert_eq!(proof.path, vec![0x35, 0xAB]);
    }

    #[test]
    fn test_hexary_proof_default() {
        let proof = HexaryProof::default();
        assert_eq!(proof.value_hash, [0u8; 32]);
        assert!(proof.levels.is_empty());
        assert_eq!(proof.root, [0u8; 32]);
        assert!(proof.path.is_empty());
    }

    #[test]
    fn test_proof_level_default() {
        let level = ProofLevel::default();
        assert_eq!(level.bitmap, 0);
        assert!(level.siblings.is_empty());
    }

    #[test]
    fn test_pack_nibbles() {
        use crate::trie::proof::pack_nibbles;

        // Even length: [5, 12] -> [0xC5] (5 in low, 12=0xC in high)
        let result = pack_nibbles(&[5, 12]);
        assert_eq!(result, vec![0xC5]);

        // Odd length: [5, 12, 3] -> [0xC5, 0x03] (5 in low, 12 in high; 3 in low, 0 in high)
        let result = pack_nibbles(&[5, 12, 3]);
        assert_eq!(result, vec![0xC5, 0x03]);
    }

    #[test]
    fn test_unpack_nibbles() {
        use crate::trie::proof::unpack_nibbles;

        // [0xC5] -> [5, 12] (always returns 2 nibbles per byte)
        let result = unpack_nibbles(&[0xC5]);
        assert_eq!(result, vec![5, 12]);

        // [0xC5, 0x03] -> [5, 12, 3, 0] (always returns 2 nibbles per byte)
        let result = unpack_nibbles(&[0xC5, 0x03]);
        assert_eq!(result, vec![5, 12, 3, 0]);
    }

    #[test]
    fn test_nibble_roundtrip() {
        use crate::trie::proof::{pack_nibbles, unpack_nibbles};

        let original = vec![1, 5, 12, 15, 7, 3];
        let packed = pack_nibbles(&original);
        let unpacked = unpack_nibbles(&packed);
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_nibble_roundtrip_with_trailing_zeros() {
        use crate::trie::proof::{pack_nibbles, unpack_nibbles};

        // Test case that would fail with buggy implementation
        let original = vec![1, 2, 3, 0]; // ends with zero
        let packed = pack_nibbles(&original);
        let unpacked = unpack_nibbles(&packed);
        assert_eq!(original, unpacked);
    }

    #[test]
    fn test_nibble_roundtrip_row_id_encoding() {
        use crate::trie::proof::{pack_nibbles, unpack_nibbles};

        // Simulate row_id=1 encoding: [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        let original = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let packed = pack_nibbles(&original);
        let unpacked = unpack_nibbles(&packed);
        assert_eq!(original, unpacked);
    }
}
