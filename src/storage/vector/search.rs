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

//! Vector search with HNSW integration
//!
//! This module provides search functionality that integrates the existing
//! HNSW index with our segment-based storage.

use std::sync::Arc;

use crate::storage::index::hnsw::{HnswDistanceMetric, HnswIndex};
use crate::storage::vector::config::VectorConfig;
use crate::storage::vector::mvcc::VectorMvcc;
use crate::storage::vector::segment::VectorSegment;

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Vector ID
    pub id: i64,
    /// Distance score
    pub distance: f64,
    /// Segment ID where found
    pub segment_id: u64,
}

/// Vector search engine
pub struct VectorSearch {
    mvcc: Arc<VectorMvcc>,
    config: VectorConfig,
    /// HNSW indexes per segment
    indexes: std::collections::HashMap<u64, HnswIndex>,
}

impl VectorSearch {
    /// Create new search engine
    pub fn new(mvcc: Arc<VectorMvcc>, config: VectorConfig) -> Self {
        Self {
            mvcc,
            config,
            indexes: std::collections::HashMap::new(),
        }
    }

    /// Get or create HNSW index for a segment
    pub fn get_or_create_index(&mut self, segment: &VectorSegment) -> &HnswIndex {
        if !self.indexes.contains_key(&segment.id) {
            let index = HnswIndex::new(
                format!("hnsw_{}", segment.id),
                "vectors".to_string(),
                "embedding".to_string(),
                0,
                segment.dimensions,
                self.config.hnsw_m,
                self.config.hnsw_ef_construction,
                self.config.hnsw_ef_search,
                HnswDistanceMetric::Cosine,
            );
            self.indexes.insert(segment.id, index);
        }
        self.indexes.get(&segment.id).unwrap()
    }

    /// Build index for a segment from its vectors
    pub fn build_index_for_segment(&mut self, segment: &VectorSegment) {
        let _index = self.get_or_create_index(segment);

        // TODO: Add vectors to HNSW
        // This requires integrating with the Index trait
        // For now, we'll do a simpler approach: search all vectors directly
    }

    /// Search across all visible segments
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult> {
        let segments = self.mvcc.visible_segments();

        if segments.is_empty() {
            return Vec::new();
        }

        // For MVP: brute-force search across all segments
        // TODO: Use HNSW indexes for large datasets
        let mut results: Vec<SearchResult> = Vec::new();

        for segment in segments {
            for i in 0..segment.count {
                // Check segment's internal deleted flag
                if segment.is_deleted(i) {
                    continue;
                }

                if let Some(embedding) = segment.get_embedding(i) {
                    let distance = cosine_distance(query, embedding);
                    if let Some(vector_id) = segment.get_vector_id(i) {
                        // Also check MVCC-level tombstone set
                        if self.mvcc.is_deleted(vector_id) {
                            continue;
                        }
                        results.push(SearchResult {
                            id: vector_id,
                            distance,
                            segment_id: segment.id,
                        });
                    }
                }
            }
        }

        // Sort by distance and take top k
        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        results.truncate(k);
        results
    }

    /// Search with HNSW (if available)
    pub fn search_with_index(&mut self, query: &[f32], k: usize) -> Vec<SearchResult> {
        // First, ensure indexes are built for all segments
        let segments = self.mvcc.visible_segments();

        for segment in segments {
            if !self.indexes.contains_key(&segment.id) {
                // Build from Arc reference - would need to dereference
                let seg_ref: &VectorSegment = &segment;
                self.build_index_for_segment(seg_ref);
            }
        }

        // TODO: Use HNSW indexes
        // For now, fall back to brute force
        self.search(query, k)
    }
}

/// Compute cosine distance between two vectors
fn cosine_distance(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for i in 0..a.len() {
        let ai = a[i] as f64;
        let bi = b[i] as f64;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 {
        return f64::MAX;
    }

    // Cosine distance = 1 - cosine similarity
    1.0 - (dot / denom)
}

/// Re-rank candidates with exact distance computation
///
/// Layer 2 of three-layer verification:
/// - Layer 1: HNSW fast search (returns approximate top-K)
/// - Layer 2: Software float re-rank (verifies with exact distance)
/// - Layer 3: Merkle proof generation
pub fn rerank(query: &[f32], candidates: &mut [SearchResult], segment: &VectorSegment) {
    // Re-compute exact distances for all candidates
    for candidate in candidates.iter_mut() {
        if let Some(embedding) = segment.get_embedding_by_id(candidate.id) {
            candidate.distance = cosine_distance(query, embedding);
        }
    }

    // Re-sort by exact distance
    candidates.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::vector::config::VectorConfig;

    #[test]
    fn test_cosine_distance() {
        let a = &[1.0f32, 0.0, 0.0];
        let b = &[1.0f32, 0.0, 0.0];

        let dist = cosine_distance(a, b);
        assert!(
            dist < 0.001,
            "identical vectors should have distance near 0"
        );
    }

    #[test]
    fn test_search() {
        let config = VectorConfig::new(3);
        let mvcc = Arc::new(VectorMvcc::new(config));

        mvcc.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        mvcc.insert(2, vec![0.0, 1.0, 0.0]).unwrap();
        mvcc.insert(3, vec![0.0, 0.0, 1.0]).unwrap();

        let search = VectorSearch::new(mvcc, VectorConfig::new(3));

        // Search for something close to [1, 0, 0]
        let results = search.search(&[1.0, 0.0, 0.0], 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1); // Should be the closest
    }
}
