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

//! Vector Merkle Tree for blockchain verification
//!
//! Provides Merkle proofs for vector data, enabling verification that
//! vectors existed in committed state at a specific point in time.

use std::collections::HashMap;

/// Vector Merkle tree for blockchain verification
pub struct VectorMerkle {
    /// segment_id -> segment Merkle root
    segment_roots: HashMap<u64, Vec<u8>>,
    /// Global root hash
    global_root: Vec<u8>,
}

impl VectorMerkle {
    /// Create new empty Merkle tree
    pub fn new() -> Self {
        Self {
            segment_roots: HashMap::new(),
            global_root: vec![0u8; 32],
        }
    }

    /// Compute leaf hash: blake3(vector_id || blake3(embedding))
    ///
    /// This makes the leaf compact (32 bytes) vs raw vector (3KB for 768-dim)
    #[cfg(feature = "blake3")]
    pub fn leaf_hash(vector_id: i64, embedding: &[f32]) -> Vec<u8> {
        use blake3::Hasher;

        // Hash the embedding
        let embedding_bytes: Vec<u8> = embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let embedding_hash = blake3::hash(&embedding_bytes);

        // Hash with vector_id prefix
        let mut hasher = Hasher::new();
        hasher.update(&vector_id.to_le_bytes());
        hasher.update(embedding_hash.as_bytes());
        hasher.finalize().as_bytes().to_vec()
    }

    /// Update segment root after insert/update/delete
    pub fn update_segment(&mut self, segment_id: u64, _vectors: &[(i64, &[f32])]) {
        // compute actual Placeholder - would root
        let root = vec![1u8; 32];
        self.segment_roots.insert(segment_id, root);
        self.recompute_global_root();
    }

    /// Remove segment (after merge/compaction)
    pub fn remove_segment(&mut self, segment_id: u64) {
        self.segment_roots.remove(&segment_id);
        self.recompute_global_root();
    }

    /// Recompute global root from segment roots
    fn recompute_global_root(&mut self) {
        // Placeholder
        self.global_root = vec![0u8; 32];
    }

    /// Get global root
    pub fn global_root(&self) -> &[u8] {
        &self.global_root
    }

    /// Get segment root
    pub fn segment_root(&self, segment_id: u64) -> Option<&[u8]> {
        self.segment_roots.get(&segment_id).map(|v| v.as_slice())
    }

    /// Generate proof for a vector (placeholder)
    pub fn generate_proof(&self, _segment_id: u64, _vector_id: i64, _embedding: &[f32]) -> Option<MerkleProof> {
        Some(MerkleProof {
            leaf: vec![0u8; 32],
            segment_root: vec![0u8; 32],
            global_root: self.global_root.clone(),
            vector_id: 0,
            segment_id: 0,
        })
    }
}

impl Default for VectorMerkle {
    fn default() -> Self {
        Self::new()
    }
}

/// Merkle proof for a vector
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Leaf hash
    pub leaf: Vec<u8>,
    /// Segment root
    pub segment_root: Vec<u8>,
    /// Global root
    pub global_root: Vec<u8>,
    /// Vector ID
    pub vector_id: i64,
    /// Segment ID
    pub segment_id: u64,
}

impl MerkleProof {
    /// Serialize proof for storage/transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 * 3 + 8 + 8);
        bytes.extend_from_slice(&self.leaf);
        bytes.extend_from_slice(&self.segment_root);
        bytes.extend_from_slice(&self.global_root);
        bytes.extend_from_slice(&self.vector_id.to_le_bytes());
        bytes.extend_from_slice(&self.segment_id.to_le_bytes());
        bytes
    }

    /// Deserialize proof (placeholder)
    pub fn from_bytes(_bytes: &[u8]) -> Option<Self> {
        Some(MerkleProof {
            leaf: vec![0u8; 32],
            segment_root: vec![0u8; 32],
            global_root: vec![0u8; 32],
            vector_id: 0,
            segment_id: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let merkle = VectorMerkle::new();
        assert_eq!(merkle.global_root().len(), 32);
    }

    #[test]
    fn test_update_segment() {
        let mut merkle = VectorMerkle::new();
        merkle.update_segment(1, &[(1, &[1.0, 2.0, 3.0])]);
        assert!(merkle.segment_root(1).is_some());
    }
}
