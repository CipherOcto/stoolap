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

//! VectorSegment: Immutable vector segment with Struct-of-Arrays layout
//!
//! Segments are the basic unit of vector storage. Each segment contains
//! up to `capacity` vectors and is immutable once full.

use crate::core::Result;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique segment ID generator
static NEXT_SEGMENT_ID: AtomicU64 = AtomicU64::new(1);

/// Immutable vector segment with Struct-of-Arrays (SoA) layout
///
/// SoA layout enables efficient SIMD operations for distance computation:
/// - All dimension 0 values are contiguous
/// - All dimension 1 values are contiguous
/// - etc.
///
/// This is more cache-friendly for vector operations than Array-of-Structs.
pub struct VectorSegment {
    /// Unique segment identifier
    pub id: u64,
    /// Vector IDs - one per vector
    pub vector_ids: Vec<i64>,
    /// Embeddings in SoA layout: [dim0_val0, dim0_val1, ..., dim1_val0, dim1_val1, ...]
    pub embeddings: Vec<f32>,
    /// Deleted flags (tombstones)
    pub deleted: Vec<bool>,
    /// Vector dimension
    pub dimensions: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Current count
    pub count: usize,
    /// Creating transaction ID
    pub created_txn: u64,
    /// Whether segment is immutable (no more inserts)
    pub is_immutable: bool,
}

impl VectorSegment {
    /// Create a new segment
    pub fn new(dimensions: usize, capacity: usize, created_txn: u64) -> Self {
        let id = NEXT_SEGMENT_ID.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            vector_ids: Vec::with_capacity(capacity),
            // SoA: dimensions * capacity floats
            embeddings: vec![0.0; dimensions * capacity],
            deleted: Vec::with_capacity(capacity),
            dimensions,
            capacity,
            count: 0,
            created_txn,
            is_immutable: false,
        }
    }

    /// Create with specific ID (for recovery)
    pub fn with_id(id: u64, dimensions: usize, capacity: usize, created_txn: u64) -> Self {
        Self {
            id,
            vector_ids: Vec::with_capacity(capacity),
            embeddings: vec![0.0; dimensions * capacity],
            deleted: Vec::with_capacity(capacity),
            dimensions,
            capacity,
            count: 0,
            created_txn,
            is_immutable: false,
        }
    }

    /// Check if segment is full
    pub fn is_full(&self) -> bool {
        self.count >= self.capacity
    }

    /// Check if segment can accept more vectors
    pub fn can_insert(&self) -> bool {
        !self.is_immutable && !self.is_full()
    }

    /// Add a vector to the segment
    ///
    /// Returns the index of the inserted vector, or error if full
    pub fn push(&mut self, vector_id: i64, embedding: &[f32]) -> Result<usize> {
        if self.is_immutable {
            return Err(crate::core::Error::SegmentImmutable(self.id));
        }
        if self.count >= self.capacity {
            return Err(crate::core::Error::SegmentFull(self.id));
        }
        if embedding.len() != self.dimensions {
            return Err(crate::core::Error::InvalidVectorDimension {
                expected: self.dimensions,
                got: embedding.len(),
            });
        }

        let idx = self.count;
        self.vector_ids.push(vector_id);

        // SoA layout: copy embedding to correct offset
        let offset = idx * self.dimensions;
        self.embeddings[offset..offset + self.dimensions].copy_from_slice(embedding);
        self.deleted.push(false);
        self.count += 1;

        Ok(idx)
    }

    /// Get embedding by index (zero-copy)
    pub fn get_embedding(&self, idx: usize) -> Option<&[f32]> {
        if idx >= self.count {
            return None;
        }
        let offset = idx * self.dimensions;
        Some(&self.embeddings[offset..offset + self.dimensions])
    }

    /// Get embedding by index (mutable)
    pub fn get_embedding_mut(&mut self, idx: usize) -> Option<&mut [f32]> {
        if idx >= self.count {
            return None;
        }
        let offset = idx * self.dimensions;
        Some(&mut self.embeddings[offset..offset + self.dimensions])
    }

    /// Mark vector as deleted (soft delete)
    pub fn delete(&mut self, idx: usize) -> Result<()> {
        if idx >= self.count {
            return Err(crate::core::Error::IndexOutOfBounds(idx, self.count));
        }
        self.deleted[idx] = true;
        Ok(())
    }

    /// Check if vector at index is deleted
    pub fn is_deleted(&self, idx: usize) -> bool {
        idx < self.deleted.len() && self.deleted[idx]
    }

    /// Get vector ID at index
    pub fn get_vector_id(&self, idx: usize) -> Option<i64> {
        self.vector_ids.get(idx).copied()
    }

    /// Find index by vector ID
    pub fn find_by_vector_id(&self, vector_id: i64) -> Option<usize> {
        self.vector_ids.iter().position(|&id| id == vector_id)
    }

    /// Get embedding by vector ID (for re-ranking)
    pub fn get_embedding_by_id(&self, vector_id: i64) -> Option<&[f32]> {
        if let Some(idx) = self.find_by_vector_id(vector_id) {
            self.get_embedding(idx)
        } else {
            None
        }
    }

    /// Mark segment as immutable
    pub fn make_immutable(&mut self) {
        self.is_immutable = true;
    }

    /// Get all (vector_id, embedding) pairs for Merkle tree
    pub fn iter_vectors(&self) -> impl Iterator<Item = (i64, &[f32])> {
        self.vector_ids
            .iter()
            .copied()
            .zip(self.embeddings.chunks(self.dimensions))
            .filter(|(id, _)| !self.deleted[self.vector_ids.iter().position(|&x| x == *id).unwrap_or(0)])
    }

    /// Get count of non-deleted vectors
    pub fn live_count(&self) -> usize {
        self.deleted.iter().filter(|&d| !d).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_push() {
        let mut segment = VectorSegment::new(3, 10, 1);
        assert_eq!(segment.count, 0);

        segment.push(1, &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(segment.count, 1);

        let emb = segment.get_embedding(0).unwrap();
        assert_eq!(emb, &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_segment_full() {
        let mut segment = VectorSegment::new(3, 2, 1);
        assert!(!segment.is_full());

        segment.push(1, &[1.0, 2.0, 3.0]).unwrap();
        assert!(!segment.is_full());

        segment.push(2, &[4.0, 5.0, 6.0]).unwrap();
        assert!(segment.is_full());
    }

    #[test]
    fn test_segment_delete() {
        let mut segment = VectorSegment::new(3, 10, 1);
        segment.push(1, &[1.0, 2.0, 3.0]).unwrap();

        assert!(!segment.is_deleted(0));
        segment.delete(0).unwrap();
        assert!(segment.is_deleted(0));
    }

    #[test]
    fn test_segment_immutable() {
        let mut segment = VectorSegment::new(3, 10, 1);
        segment.push(1, &[1.0, 2.0, 3.0]).unwrap();

        segment.make_immutable();
        assert!(segment.is_immutable);

        let result = segment.push(2, &[4.0, 5.0, 6.0]);
        assert!(result.is_err());
    }
}
