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

//! Vector storage configuration

/// Configuration for vector storage
#[derive(Debug, Clone)]
pub struct VectorConfig {
    /// Default dimension for vectors (can be overridden per-table)
    pub default_dimension: usize,
    /// Maximum vectors per segment (fixed-size segments)
    pub segment_capacity: usize,
    /// Maximum segments in memory
    pub max_segments_in_memory: usize,
    /// Enable SIMD alignment (32-byte for AVX2, 64-byte for AVX-512)
    pub simd_alignment: Option<usize>,
    /// HNSW construction parameters
    pub hnsw_m: usize,
    pub hnsw_ef_construction: usize,
    pub hnsw_ef_search: usize,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            default_dimension: 384,
            segment_capacity: 100_000,
            max_segments_in_memory: 10,
            simd_alignment: None, // Will be auto-detected
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 200,
        }
    }
}

impl VectorConfig {
    pub fn new(dimension: usize) -> Self {
        Self {
            default_dimension: dimension,
            ..Default::default()
        }
    }

    /// Auto-detect best SIMD alignment
    pub fn with_auto_simd(mut self) -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx512f") {
                self.simd_alignment = Some(64);
            } else if is_x86_feature_detected!("avx2") {
                self.simd_alignment = Some(32);
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            self.simd_alignment = Some(16); // NEON
        }
        self
    }
}
