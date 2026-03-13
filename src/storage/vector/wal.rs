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

//! Vector WAL (Write-Ahead Log) integration
//!
//! Provides serialization/deserialization of vector operations for WAL recovery.
//! Uses the table_name as the vector table name, row_id as vector_id,
//! and data field for serialized embedding bytes.

use crate::core::Result;
use crate::storage::mvcc::wal_manager::{WALEntry, WALOperationType};

/// Vector WAL entry data format
#[derive(Debug, Clone)]
pub struct VectorWalData {
    /// Segment ID where the vector is stored
    pub segment_id: u64,
    /// Vector dimension
    pub dimension: usize,
    /// Original embedding (for insert/update)
    pub embedding: Option<Vec<f32>>,
}

impl VectorWalData {
    /// Serialize vector WAL data
    pub fn serialize(&self) -> Vec<u8> {
        let mut data =
            Vec::with_capacity(8 + 4 + self.embedding.as_ref().map(|e| e.len() * 4).unwrap_or(0));

        // segment_id (u64)
        data.extend_from_slice(&self.segment_id.to_le_bytes());

        // dimension (usize as u32)
        data.extend_from_slice(&(self.dimension as u32).to_le_bytes());

        // embedding (optional)
        if let Some(ref emb) = self.embedding {
            data.extend_from_slice(&(emb.len() as u32).to_le_bytes());
            for v in emb {
                data.extend_from_slice(&v.to_le_bytes());
            }
        } else {
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        data
    }

    /// Deserialize vector WAL data
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(crate::core::Error::Parse(
                "vector WAL data too short".to_string(),
            ));
        }

        let mut offset = 0;

        // segment_id
        let segment_id = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        offset += 8;

        // dimension
        let dimension = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        offset += 4;

        // embedding
        let embedding_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
        offset += 4;

        let mut embedding = None;
        if embedding_len > 0 {
            let mut emb = Vec::with_capacity(embedding_len);
            for _ in 0..embedding_len {
                let val = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                emb.push(val);
                offset += 4;
            }
            embedding = Some(emb);
        }

        Ok(Self {
            segment_id,
            dimension,
            embedding,
        })
    }
}

/// Create a vector insert WAL entry
pub fn vector_insert_entry(
    txn_id: i64,
    table_name: &str,
    vector_id: i64,
    segment_id: u64,
    dimension: usize,
    embedding: &[f32],
) -> WALEntry {
    let wal_data = VectorWalData {
        segment_id,
        dimension,
        embedding: Some(embedding.to_vec()),
    };

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        vector_id,
        WALOperationType::VectorInsert,
        wal_data.serialize(),
    )
}

/// Create a vector update WAL entry
pub fn vector_update_entry(
    txn_id: i64,
    table_name: &str,
    vector_id: i64,
    segment_id: u64,
    dimension: usize,
    embedding: &[f32],
) -> WALEntry {
    let wal_data = VectorWalData {
        segment_id,
        dimension,
        embedding: Some(embedding.to_vec()),
    };

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        vector_id,
        WALOperationType::VectorUpdate,
        wal_data.serialize(),
    )
}

/// Create a vector delete WAL entry
pub fn vector_delete_entry(
    txn_id: i64,
    table_name: &str,
    vector_id: i64,
    segment_id: u64,
    dimension: usize,
) -> WALEntry {
    let wal_data = VectorWalData {
        segment_id,
        dimension,
        embedding: None,
    };

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        vector_id,
        WALOperationType::VectorDelete,
        wal_data.serialize(),
    )
}

/// Create a segment create WAL entry
pub fn segment_create_entry(
    txn_id: i64,
    table_name: &str,
    segment_id: u64,
    dimension: usize,
    capacity: usize,
) -> WALEntry {
    let mut data = Vec::with_capacity(8 + 4 + 4);
    data.extend_from_slice(&segment_id.to_le_bytes());
    data.extend_from_slice(&(dimension as u32).to_le_bytes());
    data.extend_from_slice(&(capacity as u32).to_le_bytes());

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        0,
        WALOperationType::SegmentCreate,
        data,
    )
}

/// Create a segment merge WAL entry
pub fn segment_merge_entry(
    txn_id: i64,
    table_name: &str,
    source_segments: &[u64],
    target_segment_id: u64,
) -> WALEntry {
    let mut data = Vec::with_capacity(4 + 4 + source_segments.len() * 8);
    data.extend_from_slice(&(source_segments.len() as u32).to_le_bytes());
    data.extend_from_slice(&target_segment_id.to_le_bytes());
    for seg_id in source_segments {
        data.extend_from_slice(&seg_id.to_le_bytes());
    }

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        0,
        WALOperationType::SegmentMerge,
        data,
    )
}

/// Create an index build WAL entry
pub fn index_build_entry(
    txn_id: i64,
    table_name: &str,
    segment_id: u64,
    index_type: &str,
) -> WALEntry {
    let mut data = Vec::with_capacity(8 + index_type.len());
    data.extend_from_slice(&segment_id.to_le_bytes());
    data.extend_from_slice(index_type.as_bytes());

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        0,
        WALOperationType::IndexBuild,
        data,
    )
}

/// Create a compaction start WAL entry
pub fn compaction_start_entry(txn_id: i64, table_name: &str, segments: &[u64]) -> WALEntry {
    let mut data = Vec::with_capacity(4 + segments.len() * 8);
    data.extend_from_slice(&(segments.len() as u32).to_le_bytes());
    for seg_id in segments {
        data.extend_from_slice(&seg_id.to_le_bytes());
    }

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        0,
        WALOperationType::CompactionStart,
        data,
    )
}

/// Create a compaction finish WAL entry
pub fn compaction_finish_entry(
    txn_id: i64,
    table_name: &str,
    old_segments: &[u64],
    new_segment_id: u64,
) -> WALEntry {
    let mut data = Vec::with_capacity(4 + 8 + old_segments.len() * 8);
    data.extend_from_slice(&(old_segments.len() as u32).to_le_bytes());
    data.extend_from_slice(&new_segment_id.to_le_bytes());
    for seg_id in old_segments {
        data.extend_from_slice(&seg_id.to_le_bytes());
    }

    WALEntry::new(
        txn_id,
        table_name.to_string(),
        0,
        WALOperationType::CompactionFinish,
        data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_wal_data_serialize() {
        let data = VectorWalData {
            segment_id: 1,
            dimension: 3,
            embedding: Some(vec![1.0, 2.0, 3.0]),
        };

        let serialized = data.serialize();
        let deserialized = VectorWalData::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.segment_id, 1);
        assert_eq!(deserialized.dimension, 3);
        assert_eq!(deserialized.embedding.unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vector_delete_wal_data() {
        let data = VectorWalData {
            segment_id: 5,
            dimension: 128,
            embedding: None,
        };

        let serialized = data.serialize();
        let deserialized = VectorWalData::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.segment_id, 5);
        assert_eq!(deserialized.dimension, 128);
        assert!(deserialized.embedding.is_none());
    }
}
