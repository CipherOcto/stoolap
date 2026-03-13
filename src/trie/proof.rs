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
/// * `path_nibble_count` - Number of nibbles in the path (needed for odd-length paths)
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

    /// Number of nibbles in the path (needed to correctly unpack odd-length paths)
    pub path_nibble_count: usize,
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
            path_nibble_count: 0,
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
            path_nibble_count: 0,
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
    /// proof.set_path(vec![5]);
    /// assert_eq!(proof.path, vec![0x50]);
    /// ```
    pub fn set_path(&mut self, path: Vec<u8>) {
        self.path_nibble_count = path.len();
        self.path = pack_nibbles(&path);
    }

    /// Verify the hexary Merkle proof
    ///
    /// Performs streaming verification from leaf to root by reconstructing
    /// each level's hash from the siblings and path nibbles.
    ///
    /// # Returns
    ///
    /// `true` if the proof is valid (reconstructed hash matches expected root), `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::{HexaryProof, hash_16_children, reconstruct_children};
    ///
    /// let value_hash = [5u8; 32];
    /// let bitmap = (1u16 << 3) | (1u16 << 5);
    /// let siblings = vec![[3u8; 32]];
    /// let children = reconstruct_children(bitmap, &siblings, 5, value_hash);
    /// let root = hash_16_children(&children);
    ///
    /// let mut proof = HexaryProof::new();
    /// proof.value_hash = value_hash;
    /// proof.add_level(bitmap, siblings);
    /// proof.set_root(root);
    /// proof.set_path(vec![0x50]);
    ///
    /// assert!(proof.verify());
    /// ```
    pub fn verify(&self) -> bool {
        // If no levels, value should equal root directly
        if self.levels.is_empty() {
            return self.value_hash == self.root;
        }

        // Unpack the path to get individual nibbles
        let full_nibbles = unpack_nibbles(&self.path);
        // Truncate to the actual path length (important for odd-length paths)
        let nibbles = &full_nibbles[..self.path_nibble_count.min(full_nibbles.len())];

        // Start with the value hash
        let mut current_hash = self.value_hash;

        // Verify each level from bottom (leaf) to top (root)
        // levels are stored root-to-leaf, so we iterate in reverse
        for (rev_idx, level) in self.levels.iter().enumerate().rev() {
            // rev_idx gives us the position from the end
            // levels = [root, leaf], so rev() gives us [(1, leaf), (0, root)]
            // rev_idx=1 means leaf (last), rev_idx=0 means root (first)

            // Get the path nibble for this level
            // Nibbles are ordered root-to-leaf: [ext_nibble, level_nibble, ...]
            // Extension nibbles come first, then level nibbles
            // Number of extension nibbles = total_nibbles - num_levels
            let ext_nibbles = nibbles.len().saturating_sub(self.levels.len());

            // For level at position rev_idx from the end (0-indexed from leaf),
            // we need the nibble at position ext_nibbles + rev_idx
            let nibble_idx = ext_nibbles + rev_idx;

            let path_nibble = if nibble_idx < nibbles.len() {
                nibbles[nibble_idx]
            } else {
                // If we've exhausted the path, use 0 (shouldn't happen in valid proofs)
                0
            };

            // Reconstruct the 16-child array
            let children =
                reconstruct_children(level.bitmap, &level.siblings, path_nibble, current_hash);

            // Hash the children to get the parent hash
            current_hash = hash_16_children(&children);
        }

        // Final hash should match the expected root
        current_hash == self.root
    }

    /// Verify multiple proofs in parallel
    ///
    /// Uses rayon for parallel verification across CPU cores.
    ///
    /// # Arguments
    ///
    /// * `proofs` - Slice of proofs to verify
    ///
    /// # Returns
    ///
    /// `true` if all proofs are valid, `false` if any proof is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "parallel")]
    /// # {
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let proofs = vec![
    ///     HexaryProof {
    ///         value_hash: [1u8; 32],
    ///         levels: vec![],
    ///         root: [1u8; 32],
    ///         path: vec![],
    ///     },
    /// ];
    ///
    /// assert!(HexaryProof::verify_batch(&proofs));
    /// # }
    /// ```
    #[cfg(feature = "parallel")]
    pub fn verify_batch(proofs: &[Self]) -> bool {
        use rayon::prelude::*;
        proofs.par_iter().all(|p| p.verify())
    }

    /// Verify multiple proofs sequentially
    ///
    /// Single-threaded version for environments without rayon.
    ///
    /// # Arguments
    ///
    /// * `proofs` - Slice of proofs to verify
    ///
    /// # Returns
    ///
    /// `true` if all proofs are valid, `false` if any proof is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::proof::HexaryProof;
    ///
    /// let proofs = vec![
    ///     HexaryProof {
    ///         value_hash: [1u8; 32],
    ///         levels: vec![],
    ///         root: [1u8; 32],
    ///         path: vec![],
    ///     },
    /// ];
    ///
    /// assert!(HexaryProof::verify_batch_sequential(&proofs));
    /// ```
    pub fn verify_batch_sequential(proofs: &[Self]) -> bool {
        proofs.iter().all(|p| p.verify())
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

/// Error type for serialization/deserialization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializationError {
    InsufficientData { expected: usize, found: usize },
    InvalidData(String),
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationError::InsufficientData { expected, found } => {
                write!(
                    f,
                    "Insufficient data: expected {} bytes, found {}",
                    expected, found
                )
            }
            SerializationError::InvalidData(msg) => {
                write!(f, "Invalid data: {}", msg)
            }
        }
    }
}

impl std::error::Error for SerializationError {}

/// Trait for Solana-style serialization
pub trait SolanaSerialize: Sized {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(data: &[u8]) -> std::result::Result<Self, SerializationError>;
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

/// Reconstruct the 16-child array from bitmap, siblings, and path
///
/// Given the bitmap of which children exist and the sibling hashes,
/// reconstruct the full 16-element child array with our hash at the path position.
///
/// # Arguments
///
/// * `bitmap` - 16-bit bitmap of existing children
/// * `siblings` - Sibling hashes (non-path children)
/// * `path_nibble` - Which child position we took (0-15)
/// * `our_hash` - Our hash to place at path_nibble position
///
/// # Returns
///
/// Array of 16 child hashes (empty positions are [0; 32])
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::reconstruct_children;
///
/// let bitmap = 0b101u16; // bits 0 and 2 set
/// let siblings = vec![[2u8; 32]];
/// let children = reconstruct_children(bitmap, &siblings, 0, [1u8; 32]);
/// assert_eq!(children[0], [1u8; 32]); // Our hash
/// assert_eq!(children[2], [2u8; 32]); // Sibling
/// ```
pub fn reconstruct_children(
    bitmap: u16,
    siblings: &[[u8; 32]],
    path_nibble: u8,
    our_hash: [u8; 32],
) -> [[u8; 32]; 16] {
    let mut children = [[0u8; 32]; 16];
    let mut sibling_idx = 0;

    for i in 0..16 {
        if bitmap & (1 << i) != 0 {
            if i == path_nibble as usize {
                children[i] = our_hash;
            } else {
                if sibling_idx < siblings.len() {
                    children[i] = siblings[sibling_idx];
                    sibling_idx += 1;
                }
            }
        }
    }

    children
}

/// Hash all 16 child hashes together using SHA-256
///
/// This function computes the hash of a branch node by hashing all 16 child hashes
/// together. This is used for computing parent node hashes in hexary tries.
///
/// # Arguments
///
/// * `children` - Array of 16 child hashes (each 32 bytes)
///
/// # Returns
///
/// SHA-256 hash of all concatenated child hashes
///
/// # Examples
///
/// ```
/// use stoolap::trie::proof::hash_16_children;
///
/// let children = [[0u8; 32]; 16];
/// let hash = hash_16_children(&children);
/// assert_eq!(hash.len(), 32);
/// ```
pub fn hash_16_children(children: &[[u8; 32]; 16]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for child in children.iter() {
        hasher.update(child);
    }
    hasher.finalize().into()
}

impl SolanaSerialize for HexaryProof {
    fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Value hash (32 bytes)
        out.extend_from_slice(&self.value_hash);

        // Root hash (32 bytes)
        out.extend_from_slice(&self.root);

        // Path nibble count (u8) + path length (u8) + path data
        out.push(self.path_nibble_count as u8);
        out.push(self.path.len() as u8);
        out.extend_from_slice(&self.path);

        // Number of levels (u8)
        out.push(self.levels.len() as u8);

        // Each level: bitmap (u16 LE) + sibling count (u8) + sibling hashes
        for level in &self.levels {
            out.extend_from_slice(&level.bitmap.to_le_bytes());
            out.push(level.siblings.len() as u8);
            for sibling in &level.siblings {
                out.extend_from_slice(sibling);
            }
        }

        out
    }

    fn deserialize(data: &[u8]) -> std::result::Result<Self, SerializationError> {
        use SerializationError::*;

        let mut cursor = 0;

        // Value hash
        if data.len() < cursor + 32 {
            return Err(InsufficientData {
                expected: cursor + 32,
                found: data.len(),
            });
        }
        let value_hash = data[cursor..cursor + 32].try_into().unwrap();
        cursor += 32;

        // Root hash
        if data.len() < cursor + 32 {
            return Err(InsufficientData {
                expected: cursor + 32,
                found: data.len(),
            });
        }
        let root = data[cursor..cursor + 32].try_into().unwrap();
        cursor += 32;

        // Path nibble count
        if data.len() < cursor + 1 {
            return Err(InsufficientData {
                expected: cursor + 1,
                found: data.len(),
            });
        }
        let path_nibble_count = data[cursor] as usize;
        cursor += 1;

        // Path length
        if data.len() < cursor + 1 {
            return Err(InsufficientData {
                expected: cursor + 1,
                found: data.len(),
            });
        }
        let path_len = data[cursor] as usize;
        cursor += 1;

        // Path data
        if data.len() < cursor + path_len {
            return Err(InsufficientData {
                expected: cursor + path_len,
                found: data.len(),
            });
        }
        let path = data[cursor..cursor + path_len].to_vec();
        cursor += path_len;

        // Levels length
        if data.len() < cursor + 1 {
            return Err(InsufficientData {
                expected: cursor + 1,
                found: data.len(),
            });
        }
        let levels_len = data[cursor] as usize;
        cursor += 1;

        let mut levels = Vec::new();
        for _ in 0..levels_len {
            // Bitmap (u16 LE)
            if data.len() < cursor + 2 {
                return Err(InsufficientData {
                    expected: cursor + 2,
                    found: data.len(),
                });
            }
            let bitmap = u16::from_le_bytes(data[cursor..cursor + 2].try_into().unwrap());
            cursor += 2;

            // Sibling count
            if data.len() < cursor + 1 {
                return Err(InsufficientData {
                    expected: cursor + 1,
                    found: data.len(),
                });
            }
            let sibling_count = data[cursor] as usize;
            cursor += 1;

            // Sibling hashes
            let mut siblings = Vec::new();
            for _ in 0..sibling_count {
                if data.len() < cursor + 32 {
                    return Err(InsufficientData {
                        expected: cursor + 32,
                        found: data.len(),
                    });
                }
                siblings.push(data[cursor..cursor + 32].try_into().unwrap());
                cursor += 32;
            }

            levels.push(ProofLevel { bitmap, siblings });
        }

        Ok(HexaryProof {
            value_hash,
            levels,
            root,
            path,
            path_nibble_count,
        })
    }
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
            levels: vec![ProofLevel {
                bitmap: 0b1000000000001000,
                siblings: vec![[2u8; 32]],
            }],
            root: [3u8; 32],
            path: vec![0x35], // nibbles [5]
            path_nibble_count: 1,
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
        // set_path now takes unpacked nibbles and packs them internally
        proof.set_path(vec![5, 11]);
        // [5, 11] packs to [0xB5] (5 in low, 11=0xB in high)
        assert_eq!(proof.path, vec![0xB5]);
        assert_eq!(proof.path_nibble_count, 2);
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

    #[test]
    fn test_reconstruct_children() {
        use crate::trie::proof::reconstruct_children;

        // At level with path nibble 5, siblings at positions 3 and 12
        // bits 3, 5, 12 set: (1<<3) | (1<<5) | (1<<12) = 8 | 32 | 4096 = 4136
        let bitmap = (1u16 << 3) | (1u16 << 5) | (1u16 << 12);
        let siblings = vec![[3u8; 32], [12u8; 32]];
        let path_nibble = 5;
        let our_hash = [5u8; 32];

        let children = reconstruct_children(bitmap, &siblings, path_nibble, our_hash);

        assert_eq!(children[3], [3u8; 32]);
        assert_eq!(children[5], [5u8; 32]); // Our hash
        assert_eq!(children[12], [12u8; 32]);
        assert_eq!(children[0], [0u8; 32]); // Empty
    }

    #[test]
    fn test_hash_16_children_empty() {
        use crate::trie::proof::hash_16_children;

        let children = [[0u8; 32]; 16];
        let hash = hash_16_children(&children);
        // All zeros should hash to a specific value
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_hash_16_children_single() {
        use crate::trie::proof::hash_16_children;

        let mut children = [[0u8; 32]; 16];
        children[0] = [1u8; 32];
        let hash = hash_16_children(&children);
        // Should be different from empty children
        assert_ne!(hash, [0u8; 32]);
    }

    #[test]
    fn test_hash_16_children_deterministic() {
        use crate::trie::proof::hash_16_children;

        let mut children = [[0u8; 32]; 16];
        children[5] = [42u8; 32];
        children[12] = [99u8; 32];

        let hash1 = hash_16_children(&children);
        let hash2 = hash_16_children(&children);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hexary_proof_verify_single_level() {
        use crate::trie::proof::{
            hash_16_children, pack_nibbles, reconstruct_children, HexaryProof,
        };

        // Create a simple proof with one level
        // Path nibble is 5, siblings at positions 3 and 12
        let bitmap = (1u16 << 3) | (1u16 << 5) | (1u16 << 12);
        let siblings = vec![[3u8; 32], [12u8; 32]];
        let value_hash = [5u8; 32]; // Our value

        // Reconstruct children and compute expected root
        let children = reconstruct_children(bitmap, &siblings, 5, value_hash);
        let expected_root = hash_16_children(&children);

        let mut proof = HexaryProof::new();
        proof.value_hash = value_hash;
        proof.add_level(bitmap, siblings);
        proof.set_root(expected_root);
        proof.set_path(vec![5]); // set_path now packs internally

        assert!(proof.verify());
    }

    #[test]
    fn test_hexary_proof_verify_two_levels() {
        use crate::trie::proof::{
            hash_16_children, pack_nibbles, reconstruct_children, HexaryProof,
        };

        // Create a two-level proof
        let value_hash = [1u8; 32];

        // Level 0 (leaf): path nibble 5, sibling at position 3
        let level0_bitmap = (1u16 << 3) | (1u16 << 5);
        let level0_siblings = vec![[3u8; 32]];
        let level0_children = reconstruct_children(level0_bitmap, &level0_siblings, 5, value_hash);
        let level0_hash = hash_16_children(&level0_children);

        // Level 1 (root): path nibble 12, sibling at position 7
        let level1_bitmap = (1u16 << 7) | (1u16 << 12);
        let level1_siblings = vec![[7u8; 32]];
        let level1_children =
            reconstruct_children(level1_bitmap, &level1_siblings, 12, level0_hash);
        let expected_root = hash_16_children(&level1_children);

        // Path is [12, 5] - level 1 uses nibble 12, level 0 uses nibble 5
        let mut proof = HexaryProof::new();
        proof.value_hash = value_hash;
        proof.add_level(level1_bitmap, level1_siblings); // Root level first
        proof.add_level(level0_bitmap, level0_siblings); // Leaf level
        proof.set_root(expected_root);
        proof.set_path(vec![12, 5]); // set_path now packs internally

        assert!(proof.verify());
    }

    #[test]
    fn test_hexary_proof_verify_invalid_root() {
        use crate::trie::proof::HexaryProof;

        // Create a proof with wrong root
        let mut proof = HexaryProof::new();
        proof.value_hash = [1u8; 32];
        proof.add_level(0b1000000000001000, vec![[2u8; 32]]);
        proof.set_root([99u8; 32]); // Wrong root
        proof.set_path(vec![1]); // set_path now packs internally

        assert!(!proof.verify());
    }

    #[test]
    fn test_hexary_proof_serialization_roundtrip() {
        use crate::trie::proof::{HexaryProof, ProofLevel, SolanaSerialize};

        let original = HexaryProof {
            value_hash: [1u8; 32],
            levels: vec![ProofLevel {
                bitmap: 0x1234,
                siblings: vec![[2u8; 32], [3u8; 32]],
            }],
            root: [4u8; 32],
            path: vec![0x35, 0xC3],
            path_nibble_count: 4, // 2 bytes * 2 nibbles
        };

        let serialized = original.serialize();
        let deserialized = HexaryProof::deserialize(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_hexary_proof_batch_verify() {
        use crate::trie::proof::HexaryProof;

        let proofs = vec![
            HexaryProof {
                value_hash: [1u8; 32],
                levels: vec![],
                root: [1u8; 32],
                path: vec![],
                path_nibble_count: 0,
            },
            HexaryProof {
                value_hash: [2u8; 32],
                levels: vec![],
                root: [2u8; 32],
                path: vec![],
                path_nibble_count: 0,
            },
        ];

        assert!(HexaryProof::verify_batch_sequential(&proofs));

        // With invalid proof
        let invalid_proofs = vec![HexaryProof {
            value_hash: [1u8; 32],
            levels: vec![],
            root: [99u8; 32], // Wrong
            path: vec![],
            path_nibble_count: 0,
        }];

        assert!(!HexaryProof::verify_batch_sequential(&invalid_proofs));
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_hexary_proof_batch_verify_parallel() {
        use crate::trie::proof::HexaryProof;

        let proofs = vec![
            HexaryProof {
                value_hash: [1u8; 32],
                levels: vec![],
                root: [1u8; 32],
                path: vec![],
                path_nibble_count: 0,
            },
            HexaryProof {
                value_hash: [2u8; 32],
                levels: vec![],
                root: [2u8; 32],
                path: vec![],
                path_nibble_count: 0,
            },
        ];

        assert!(HexaryProof::verify_batch(&proofs));

        // With invalid proof
        let invalid_proofs = vec![HexaryProof {
            value_hash: [1u8; 32],
            levels: vec![],
            root: [99u8; 32], // Wrong
            path: vec![],
            path_nibble_count: 0,
        }];

        assert!(!HexaryProof::verify_batch(&invalid_proofs));
    }
}
