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

//! Vector storage module
//!
//! This module provides vector-specific storage including:
//! - VectorSegment: Immutable segments with Struct-of-Arrays layout
//! - VectorMVCC: Segment-level MVCC visibility
//! - VectorMerkle: Merkle tree for blockchain verification

pub mod config;
pub mod merkle;
pub mod mmap;
pub mod mvcc;
pub mod search;
pub mod segment;
pub mod snapshot;
pub mod wal;
pub mod wal_logger;
pub mod wal_recovery;

pub use config::VectorConfig;
pub use merkle::{MerkleProof, VectorMerkle};
pub use mmap::{delete_segment, is_segment_ready, MmapVectorSegment, MmapVectorSegmentMut};
pub use mvcc::VectorMvcc;
pub use search::{SearchResult, VectorSearch};
pub use segment::VectorSegment;
pub use wal::{compaction_finish_entry, compaction_start_entry, segment_create_entry,
    segment_merge_entry, vector_delete_entry, vector_insert_entry, vector_update_entry,
    VectorWalData};
pub use wal_logger::{VectorWalEntry, VectorWalLogger};
