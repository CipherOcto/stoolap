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

//! Vector Snapshot Manager
//!
//! Provides snapshot creation and loading for fast recovery.
//! Snapshots capture segment state at a point in time.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::core::Result;
use super::mmap::MmapVectorSegmentMut;

/// Snapshot metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotMeta {
    /// Snapshot version
    pub version: u32,
    /// Creation timestamp (nanoseconds since epoch)
    pub created_at: u64,
    /// Number of segments in snapshot
    pub segment_count: usize,
    /// Total vector count
    pub vector_count: usize,
    /// Merkle root
    pub merkle_root: Vec<u8>,
}

/// Snapshot manager for vector storage
pub struct VectorSnapshotManager {
    /// Base path for snapshots
    base_path: PathBuf,
}

impl VectorSnapshotManager {
    /// Create new snapshot manager
    pub fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
        }
    }

    /// Create snapshot from segment
    pub fn create_snapshot(
        &self,
        segment: &MmapVectorSegmentMut,
        merkle_root: &[u8],
    ) -> Result<PathBuf> {
        // Create snapshot directory with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let snapshot_dir = self.base_path.join(format!("snapshot_{}", timestamp));
        fs::create_dir_all(&snapshot_dir)?;

        // Flush segment to disk
        segment.flush_to_disk(&snapshot_dir)?;

        // Write snapshot metadata
        let meta = SnapshotMeta {
            version: 1,
            created_at: timestamp,
            segment_count: 1,
            vector_count: segment.count(),
            merkle_root: merkle_root.to_vec(),
        };

        let meta_path = snapshot_dir.join("meta.json");
        let json = serde_json::to_string_pretty(&meta)
            .map_err(|e| crate::core::Error::Parse(e.to_string()))?;
        fs::write(&meta_path, json)?;

        Ok(snapshot_dir)
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<(PathBuf, SnapshotMeta)>> {
        let mut snapshots = Vec::new();

        if !self.base_path.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("snapshot_") {
                        if let Ok(meta) = self.load_metadata(&path) {
                            snapshots.push((path, meta));
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));

        Ok(snapshots)
    }

    /// Get latest snapshot
    pub fn get_latest_snapshot(&self) -> Result<Option<(PathBuf, SnapshotMeta)>> {
        let snapshots = self.list_snapshots()?;
        Ok(snapshots.into_iter().next())
    }

    /// Load metadata from snapshot
    pub fn load_metadata(&self, snapshot_path: &Path) -> Result<SnapshotMeta> {
        let meta_path = snapshot_path.join("meta.json");
        let contents = fs::read_to_string(&meta_path)?;
        let meta: SnapshotMeta = serde_json::from_str(&contents)
            .map_err(|e| crate::core::Error::Parse(e.to_string()))?;
        Ok(meta)
    }

    /// Delete old snapshots (keep latest N)
    pub fn cleanup_snapshots(&self, keep: usize) -> Result<()> {
        let mut snapshots = self.list_snapshots()?;

        // Keep only the latest `keep` snapshots
        if snapshots.len() > keep {
            for (path, _) in snapshots.drain(keep..) {
                fs::remove_dir_all(path)?;
            }
        }

        Ok(())
    }

    /// Get snapshot path for a segment
    pub fn get_segment_path(&self, segment_id: u64) -> PathBuf {
        self.base_path.join(format!("segment_{}", segment_id))
    }
}

impl Default for VectorSnapshotManager {
    fn default() -> Self {
        Self::new(&PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_create_and_list() {
        let dir = tempdir().unwrap();
        let mgr = VectorSnapshotManager::new(dir.path());

        // Create a segment
        let mut segment = MmapVectorSegmentMut::new(1, 3, 10);
        segment.push(1, &[1.0, 2.0, 3.0]).unwrap();
        segment.push(2, &[4.0, 5.0, 6.0]).unwrap();

        // Create snapshot
        let merkle_root = vec![0u8; 32];
        let snapshot_path = mgr.create_snapshot(&segment, &merkle_root).unwrap();
        assert!(snapshot_path.exists());

        // List snapshots
        let snapshots = mgr.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 1);

        // Get latest
        let (path, meta) = mgr.get_latest_snapshot().unwrap().unwrap();
        assert_eq!(meta.vector_count, 2);
    }
}
