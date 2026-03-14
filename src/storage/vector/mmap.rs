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

//! Memory-mapped vector segment storage
//!
//! Provides persistent storage for vector segments using memory-mapped files.
//! Follows Qdrant's approach: typed wrappers, crash-safe file format.

use memmap2::Mmap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use crate::core::Result;

/// File header magic bytes
const VECTORS_HEADER: &[u8; 4] = b"vec1";
const DELETED_HEADER: &[u8; 4] = b"del1";
const METADATA_HEADER: &[u8; 4] = b"meta";

/// Memory-mapped vector segment (immutable, loaded from disk)
pub struct MmapVectorSegment {
    /// Segment ID
    pub id: u64,
    /// Memory-mapped vector data (SoA layout)
    vectors: Mmap,
    /// Bit-packed deletion flags
    deleted: Vec<u8>,
    /// Vector dimension
    pub dimension: usize,
    /// Number of vectors
    pub count: usize,
    /// Vector IDs (stored in metadata)
    vector_ids: Vec<i64>,
}

impl MmapVectorSegment {
    /// Load segment from disk
    pub fn load_from(path: &Path) -> Result<Self> {
        // Load vectors
        let vectors_path = path.join("vectors.bin");
        let vectors_file = File::open(&vectors_path)?;
        let vectors = unsafe { Mmap::map(&vectors_file)? };

        // Load deletion flags
        let deleted_path = path.join("deleted.bin");
        let deleted_file = File::open(&deleted_path)?;
        let deleted_mmap = unsafe { Mmap::map(&deleted_file)? };
        let deleted: Vec<u8> = deleted_mmap.to_vec();

        // Load metadata
        let metadata = Self::load_metadata(path)?;

        Ok(Self {
            id: metadata.segment_id,
            vectors,
            deleted,
            dimension: metadata.dimension,
            count: metadata.count,
            vector_ids: metadata.vector_ids,
        })
    }

    /// Get embedding by index
    pub fn get_embedding(&self, idx: usize) -> Option<&[f32]> {
        if idx >= self.count {
            return None;
        }
        let offset = idx * self.dimension;
        let data: &[f32] = unsafe {
            std::slice::from_raw_parts(
                self.vectors.as_ptr().add(offset) as *const f32,
                self.dimension,
            )
        };
        Some(data)
    }

    /// Check if vector at index is deleted
    pub fn is_deleted(&self, idx: usize) -> bool {
        if idx >= self.count {
            return false;
        }
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        if byte_idx >= self.deleted.len() {
            return false;
        }
        (self.deleted[byte_idx] & (1 << bit_idx)) != 0
    }

    /// Get vector ID at index
    pub fn get_vector_id(&self, idx: usize) -> Option<i64> {
        self.vector_ids.get(idx).copied()
    }

    /// Get segment ID
    pub fn segment_id(&self) -> u64 {
        self.id
    }

    /// Load metadata from file
    fn load_metadata(path: &Path) -> Result<SegmentMetadata> {
        let metadata_path = path.join("metadata.json");
        let mut file = File::open(&metadata_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let meta: SegmentMetadata = serde_json::from_str(&contents)
            .map_err(|e| crate::core::Error::Parse(e.to_string()))?;
        Ok(meta)
    }
}

/// Mutable segment for writes (kept in memory, flushed to disk)
pub struct MmapVectorSegmentMut {
    /// Segment ID
    pub id: u64,
    /// Vector data in SoA layout
    vectors: Vec<f32>,
    /// Bit-packed deletion flags
    deleted: Vec<u8>,
    /// Vector IDs
    vector_ids: Vec<i64>,
    /// Vector dimension
    dimension: usize,
    /// Capacity
    capacity: usize,
    /// Current count
    count: usize,
}

impl MmapVectorSegmentMut {
    /// Create new mutable segment
    pub fn new(id: u64, dimension: usize, capacity: usize) -> Self {
        let vectors = vec![0.0; dimension * capacity];
        // Pre-allocate deleted flags (1 bit per vector)
        let deleted_len = capacity.div_ceil(8);
        let deleted = vec![0u8; deleted_len];
        let vector_ids = Vec::with_capacity(capacity);

        Self {
            id,
            vectors,
            deleted,
            vector_ids,
            dimension,
            capacity,
            count: 0,
        }
    }

    /// Push a vector with ID
    pub fn push(&mut self, vector_id: i64, embedding: &[f32]) -> Result<usize> {
        if self.count >= self.capacity {
            return Err(crate::core::Error::SegmentFull(self.id));
        }
        if embedding.len() != self.dimension {
            return Err(crate::core::Error::InvalidVectorDimension {
                expected: self.dimension,
                got: embedding.len(),
            });
        }

        let idx = self.count;
        let offset = idx * self.dimension;
        self.vectors[offset..offset + self.dimension].copy_from_slice(embedding);
        self.vector_ids.push(vector_id);
        self.count += 1;

        Ok(idx)
    }

    /// Mark as deleted
    pub fn delete(&mut self, idx: usize) -> Result<()> {
        if idx >= self.count {
            return Err(crate::core::Error::IndexOutOfBounds(idx, self.count));
        }
        let byte_idx = idx / 8;
        let bit_idx = idx % 8;
        self.deleted[byte_idx] |= 1 << bit_idx;
        Ok(())
    }

    /// Check if full
    pub fn is_full(&self) -> bool {
        self.count >= self.capacity
    }

    /// Get count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Flush to disk
    pub fn flush_to_disk(&self, path: &Path) -> Result<()> {
        // Create directory
        fs::create_dir_all(path)?;

        // Write vectors.bin
        self.write_vectors(path)?;

        // Write deleted.bin
        self.write_deleted(path)?;

        // Write metadata.json
        self.write_metadata(path)?;

        // Write version marker (last = ready)
        self.write_version(path)?;

        Ok(())
    }

    fn write_vectors(&self, path: &Path) -> Result<()> {
        let file_path = path.join("vectors.bin");
        let mut file = File::create(&file_path)?;

        // Write header
        file.write_all(VECTORS_HEADER)?;

        // Write data
        let data: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.vectors.as_ptr() as *const u8,
                self.dimension * self.count * std::mem::size_of::<f32>(),
            )
        };
        file.write_all(data)?;

        // Sync to disk
        file.sync_all()?;

        Ok(())
    }

    fn write_deleted(&self, path: &Path) -> Result<()> {
        let file_path = path.join("deleted.bin");
        let mut file = File::create(&file_path)?;

        // Write deletion flags directly (no header - metadata has count)
        let deleted_len = self.count.div_ceil(8);
        file.write_all(&self.deleted[..deleted_len])?;

        // Sync
        file.sync_all()?;

        Ok(())
    }

    fn write_metadata(&self, path: &Path) -> Result<()> {
        let file_path = path.join("metadata.json");
        let metadata = SegmentMetadata {
            segment_id: self.id,
            dimension: self.dimension,
            count: self.count,
            capacity: self.capacity,
            version: 1,
            vector_ids: self.vector_ids.clone(),
        };
        let json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| crate::core::Error::Parse(e.to_string()))?;
        fs::write(&file_path, json)?;

        Ok(())
    }

    fn write_version(&self, path: &Path) -> Result<()> {
        // Version file marks segment as "ready"
        let file_path = path.join("version.info");
        fs::write(&file_path, "1")?;
        Ok(())
    }
}

/// Segment metadata stored in JSON
#[derive(serde::Serialize, serde::Deserialize)]
struct SegmentMetadata {
    segment_id: u64,
    dimension: usize,
    count: usize,
    capacity: usize,
    version: u32,
    vector_ids: Vec<i64>,
}

/// Check if a segment directory is valid (has version.info)
pub fn is_segment_ready(path: &Path) -> bool {
    path.join("version.info").exists()
}

/// Delete segment from disk
pub fn delete_segment(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mut_segment_push() {
        let mut seg = MmapVectorSegmentMut::new(1, 3, 10);
        assert_eq!(seg.count(), 0);

        seg.push(1, &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(seg.count(), 1);

        seg.push(2, &[4.0, 5.0, 6.0]).unwrap();
        assert_eq!(seg.count(), 2);
    }

    #[test]
    fn test_mut_segment_delete() {
        let mut seg = MmapVectorSegmentMut::new(1, 3, 10);
        seg.push(1, &[1.0, 2.0, 3.0]).unwrap();

        seg.delete(0).unwrap();
        assert!(seg.deleted[0] & 1 != 0);
    }

    #[test]
    fn test_flush_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("segment_1");

        let mut seg = MmapVectorSegmentMut::new(1, 3, 10);
        seg.push(1, &[1.0, 2.0, 3.0]).unwrap();
        seg.push(2, &[4.0, 5.0, 6.0]).unwrap();
        seg.delete(0).unwrap();

        seg.flush_to_disk(&path).unwrap();
        assert!(is_segment_ready(&path));

        let loaded = MmapVectorSegment::load_from(&path).unwrap();
        assert_eq!(loaded.count, 2);
        assert_eq!(loaded.dimension, 3);
        assert!(loaded.is_deleted(0));
        assert!(!loaded.is_deleted(1));
    }
}
