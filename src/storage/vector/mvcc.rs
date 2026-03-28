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

//! Vector MVCC: Segment-level MVCC for vector storage
//!
//! Unlike row-based MVCC, vectors use segment-level visibility.
//! This is because HNSW graph traversal doesn't work well with per-vector
//! visibility checks (causes branch mispredictions, 20-40% performance loss).

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::config::VectorConfig;
use super::segment::VectorSegment;
use crate::common::I64Set;
use crate::core::{Error, Result};

/// State of a vector segment
#[derive(Clone)]
enum SegmentState {
    /// Active segment - new vectors go here
    Active(Arc<VectorSegment>),
    /// Immutable - read-only, can be searched
    Immutable(Arc<VectorSegment>),
    /// Being merged - excluded from queries
    Merging(Vec<u64>),
}

/// Version tracker for in-place updates
#[derive(Default)]
struct VersionTracker {
    /// vector_id -> (segment_id, index_in_segment)
    locations: HashMap<i64, (u64, usize)>,
    /// Soft-deleted vector IDs (tombstones)
    deleted: I64Set,
    next_segment_id: u64,
}

/// Vector MVCC with segment-level visibility
pub struct VectorMvcc {
    segments: RwLock<HashMap<u64, SegmentState>>,
    active_segment_id: RwLock<Option<u64>>,
    version_tracker: RwLock<VersionTracker>,
    config: VectorConfig,
    /// WAL logger for persistence
    wal: Option<super::wal_logger::VectorWalLogger>,
    /// Table name for WAL logging
    table_name: String,
}

impl VectorMvcc {
    /// Create new VectorMVCC (backward compatible)
    pub fn new(config: VectorConfig) -> Self {
        Self::with_wal(config, None, "vectors".to_string())
    }

    /// Create new VectorMVCC with WAL logging
    pub fn with_wal(
        config: VectorConfig,
        wal: Option<super::wal_logger::VectorWalLogger>,
        table_name: String,
    ) -> Self {
        let tracker = VersionTracker {
            locations: HashMap::new(),
            deleted: I64Set::new(),
            next_segment_id: 1,
        };

        // Create first active segment
        let segment = Arc::new(VectorSegment::new(
            config.default_dimension,
            config.segment_capacity,
            1,
        ));
        let first_id = segment.id;
        let mut segments = HashMap::new();
        segments.insert(first_id, SegmentState::Active(segment));

        Self {
            segments: RwLock::new(segments),
            active_segment_id: RwLock::new(Some(first_id)),
            version_tracker: RwLock::new(tracker),
            config,
            wal,
            table_name,
        }
    }

    /// Insert a vector
    pub fn insert(&self, vector_id: i64, embedding: Vec<f32>) -> Result<()> {
        let active_id = *self.active_segment_id.read();

        if let Some(seg_id) = active_id {
            let mut segments = self.segments.write();
            if let Some(state) = segments.get_mut(&seg_id) {
                if let SegmentState::Active(segment) = state {
                    if let Some(seg) = Arc::get_mut(segment) {
                        let idx = seg.push(vector_id, &embedding)?;
                        self.version_tracker
                            .write()
                            .locations
                            .insert(vector_id, (seg_id, idx));

                        // WAL logging
                        if let Some(ref wal) = self.wal {
                            let _ = wal.log_insert(&self.table_name, vector_id, seg_id, &embedding);
                        }
                        return Ok(());
                    }
                }
            }
        }

        Err(Error::NoActiveSegment)
    }

    /// Update a vector
    pub fn update(&self, vector_id: i64, new_embedding: Vec<f32>) -> Result<()> {
        self.insert(vector_id, new_embedding)
    }

    /// Delete a vector (soft delete via tombstone)
    pub fn delete(&self, vector_id: i64) -> Result<()> {
        // Check if vector exists and get segment_id
        let segment_id = {
            let tracker = self.version_tracker.read();
            if !tracker.locations.contains_key(&vector_id) {
                return Err(Error::SegmentNotFound);
            }
            // Check if already deleted
            if tracker.deleted.contains(vector_id) {
                return Ok(()); // Already deleted
            }
            tracker.locations.get(&vector_id).map(|(s, _)| *s)
        };

        // Mark as deleted in tombstone set
        let mut tracker = self.version_tracker.write();
        tracker.deleted.insert(vector_id);

        // WAL logging
        if let (Some(seg_id), Some(ref wal)) = (segment_id, &self.wal) {
            let _ = wal.log_delete(&self.table_name, vector_id, seg_id);
        }

        Ok(())
    }

    /// Check if a vector is deleted
    pub fn is_deleted(&self, vector_id: i64) -> bool {
        let tracker = self.version_tracker.read();
        tracker.deleted.contains(vector_id)
    }

    /// Get all visible segments for reading
    pub fn visible_segments(&self) -> Vec<Arc<VectorSegment>> {
        let segments = self.segments.read();
        segments
            .values()
            .filter_map(|state| match state {
                SegmentState::Active(s) | SegmentState::Immutable(s) => Some(s.clone()),
                SegmentState::Merging(_) => None,
            })
            .collect()
    }

    /// Get active segment for writing
    pub fn active_segment(&self) -> Option<Arc<VectorSegment>> {
        let active_id = *self.active_segment_id.read();
        if let Some(seg_id) = active_id {
            let segments = self.segments.read();
            if let Some(SegmentState::Active(segment)) = segments.get(&seg_id) {
                return Some(segment.clone());
            }
        }
        None
    }

    /// Get all segments
    pub fn all_segments(&self) -> Vec<Arc<VectorSegment>> {
        let segments = self.segments.read();
        segments
            .values()
            .filter_map(|state| match state {
                SegmentState::Active(s) => Some(s.clone()),
                SegmentState::Immutable(s) => Some(s.clone()),
                SegmentState::Merging(_) => None,
            })
            .collect()
    }

    /// Get segment by ID
    pub fn get_segment(&self, segment_id: u64) -> Option<Arc<VectorSegment>> {
        let segments = self.segments.read();
        match segments.get(&segment_id) {
            Some(SegmentState::Active(s)) => Some(s.clone()),
            Some(SegmentState::Immutable(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Get current segment count
    pub fn segment_count(&self) -> usize {
        self.segments.read().len()
    }

    /// Get total vector count
    pub fn total_vector_count(&self) -> usize {
        let segments = self.segments.read();
        segments
            .values()
            .map(|s| match s {
                SegmentState::Active(seg) => seg.count,
                SegmentState::Immutable(seg) => seg.count,
                SegmentState::Merging(_) => 0,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let config = VectorConfig::new(3);
        let mvcc = VectorMvcc::new(config);

        mvcc.insert(1, vec![1.0, 2.0, 3.0]).unwrap();
        mvcc.insert(2, vec![4.0, 5.0, 6.0]).unwrap();

        assert_eq!(mvcc.total_vector_count(), 2);
    }

    #[test]
    fn test_visible_segments() {
        let config = VectorConfig::new(3);
        let mvcc = VectorMvcc::new(config);

        mvcc.insert(1, vec![1.0, 2.0, 3.0]).unwrap();

        let segments = mvcc.visible_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn test_delete() {
        let config = VectorConfig::new(3);
        let mvcc = VectorMvcc::new(config);

        mvcc.insert(1, vec![1.0, 2.0, 3.0]).unwrap();
        mvcc.insert(2, vec![4.0, 5.0, 6.0]).unwrap();

        assert_eq!(mvcc.total_vector_count(), 2);

        // Delete vector 1
        mvcc.delete(1).unwrap();
        assert!(mvcc.is_deleted(1));
        assert!(!mvcc.is_deleted(2));

        // Deleting again should be idempotent
        mvcc.delete(1).unwrap();
        assert!(mvcc.is_deleted(1));
    }

    #[test]
    fn test_delete_nonexistent() {
        let config = VectorConfig::new(3);
        let mvcc = VectorMvcc::new(config);

        let result = mvcc.delete(999);
        assert!(result.is_err());
    }
}
