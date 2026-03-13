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

//! Product Quantization (PQ) implementation
//!
//! PQ splits vectors into sub-vectors and quantizes each using k-means.
//! Provides 4-64x compression with configurable sub-vectors and codebook size.

use std::collections::HashMap;

/// Product Quantizer: splits vectors into sub-vectors, quantizes each
///
/// Compression is achieved by:
/// 1. Split dimension D into M sub-vectors of size D/M
/// 2. Build codebook using k-means for each sub-vector position
/// 3. Store only codebook indices (much smaller than original)
pub struct ProductQuantizer {
    /// Original vector dimension
    dimension: usize,
    /// Number of sub-vectors
    sub_vectors: usize,
    /// Size of each sub-vector
    sub_vector_size: usize,
    /// Number of centroids (2^bits, typically 256 for 8 bits)
    num_centroids: usize,
    /// Codebook: [sub_vector_idx][centroid_idx] -> centroid values
    codebook: Vec<Vec<f32>>,
    /// Trained centroids for each sub-vector position
    centroids: Vec<Vec<f32>>,
}

impl ProductQuantizer {
    /// Create new PQ with default settings
    ///
    /// # Arguments
    /// * `dimension` - Vector dimension (must be divisible by sub_vectors)
    /// * `sub_vectors` - Number of sub-vectors (typical: 8, 16)
    /// * `bits_per_subvector` - Bits per sub-vector (typical: 8 = 256 centroids)
    pub fn new(dimension: usize, sub_vectors: usize, bits_per_subvector: u32) -> Self {
        let sub_vector_size = dimension / sub_vectors;
        let num_centroids = 1 << bits_per_subvector; // 2^bits

        Self {
            dimension,
            sub_vectors,
            sub_vector_size,
            num_centroids,
            codebook: Vec::new(),
            centroids: Vec::new(),
        }
    }

    /// Create PQ with 8 sub-vectors and 8 bits (256 centroids)
    pub fn new_default(dimension: usize) -> Self {
        Self::new(dimension, 8, 8)
    }

    /// Get dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get number of sub-vectors
    pub fn sub_vectors(&self) -> usize {
        self.sub_vectors
    }

    /// Get encoded size in bytes
    pub fn encoded_size(&self) -> usize {
        self.sub_vectors // 1 byte per sub-vector
    }

    /// Train PQ on a set of vectors
    ///
    /// Uses k-means clustering for each sub-vector position
    pub fn train(&mut self, vectors: &[&[f32]]) {
        self.centroids.clear();
        self.codebook.clear();

        // For each sub-vector position
        for sub_idx in 0..self.sub_vectors {
            let start = sub_idx * self.sub_vector_size;
            let end = start + self.sub_vector_size;

            // Collect all sub-vectors at this position
            let mut sub_vectors: Vec<Vec<f32>> =
                vectors.iter().map(|v| v[start..end].to_vec()).collect();

            // Run k-means
            let centroids = self.kmeans(&mut sub_vectors, self.num_centroids, 20);
            self.centroids.push(centroids.clone());
            self.codebook.push(centroids);
        }
    }

    /// Simple k-means clustering
    fn kmeans(&self, vectors: &mut Vec<Vec<f32>>, k: usize, iterations: usize) -> Vec<f32> {
        if vectors.is_empty() || k == 0 {
            return vec![0.0; self.sub_vector_size * k];
        }

        // Initialize centroids randomly from data
        let mut centroids: Vec<Vec<f32>> =
            (0..k).map(|i| vectors[i % vectors.len()].clone()).collect();

        for _ in 0..iterations {
            // Assign each vector to nearest centroid
            let mut assignments: Vec<usize> = Vec::with_capacity(vectors.len());
            for v in vectors.iter() {
                let mut min_dist = f32::MAX;
                let mut best_centroid = 0;
                for (idx, c) in centroids.iter().enumerate() {
                    let dist = self.squared_distance(v, c);
                    if dist < min_dist {
                        min_dist = dist;
                        best_centroid = idx;
                    }
                }
                assignments.push(best_centroid);
            }

            // Update centroids
            let mut new_centroids: Vec<Vec<f32>> = vec![vec![0.0; self.sub_vector_size]; k];
            let mut counts: Vec<usize> = vec![0; k];

            for (v, &assign) in vectors.iter().zip(assignments.iter()) {
                for (i, &val) in v.iter().enumerate() {
                    new_centroids[assign][i] += val;
                }
                counts[assign] += 1;
            }

            for (idx, c) in new_centroids.iter_mut().enumerate() {
                if counts[idx] > 0 {
                    for val in c.iter_mut() {
                        *val /= counts[idx] as f32;
                    }
                } else {
                    // Keep old centroid if no assignment
                    *c = centroids[idx].clone();
                }
            }

            centroids = new_centroids;
        }

        // Flatten centroids for storage
        centroids.into_iter().flatten().collect()
    }

    /// Squared Euclidean distance
    fn squared_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
    }

    /// Encode a vector
    ///
    /// Returns indices into codebook for each sub-vector
    pub fn encode(&self, vector: &[f32]) -> Vec<u8> {
        let mut codes = Vec::with_capacity(self.sub_vectors);

        for sub_idx in 0..self.sub_vectors {
            let start = sub_idx * self.sub_vector_size;
            let end = start + self.sub_vector_size;
            let sub_vec = &vector[start..end];

            // Find nearest centroid
            let centroids = &self.codebook[sub_idx];
            let mut min_dist = f32::MAX;
            let mut best_code = 0u8;

            for code in 0..self.num_centroids as u8 {
                let c_start = (code as usize) * self.sub_vector_size;
                let centroid = &centroids[c_start..c_start + self.sub_vector_size];
                let dist = self.squared_distance(sub_vec, centroid);
                if dist < min_dist {
                    min_dist = dist;
                    best_code = code;
                }
            }

            codes.push(best_code);
        }

        codes
    }

    /// Decode a vector from codes
    ///
    /// Reconstructs approximation from codebook
    pub fn decode(&self, codes: &[u8]) -> Vec<f32> {
        let mut result = vec![0.0; self.dimension];

        for (sub_idx, &code) in codes.iter().enumerate() {
            let start = sub_idx * self.sub_vector_size;
            let centroids = &self.codebook[sub_idx];
            let c_start = (code as usize) * self.sub_vector_size;

            for i in 0..self.sub_vector_size {
                result[start + i] = centroids[c_start + i];
            }
        }

        result
    }

    /// Encode query vector
    pub fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        self.encode(query)
    }

    /// Check if trained
    pub fn is_trained(&self) -> bool {
        !self.centroids.is_empty()
    }
}

impl Default for ProductQuantizer {
    fn default() -> Self {
        Self::new(128, 8, 8)
    }
}

/// Trait for quantizers
pub trait Quantizer: Send + Sync {
    /// Encode a vector
    fn encode(&self, vector: &[f32]) -> Vec<u8>;

    /// Decode an encoded vector back to f32
    fn decode(&self, data: &[u8]) -> Vec<f32>;

    /// Encode a query vector
    fn encode_query(&self, query: &[f32]) -> Vec<u8>;
}

impl Quantizer for ProductQuantizer {
    fn encode(&self, vector: &[f32]) -> Vec<u8> {
        ProductQuantizer::encode(self, vector)
    }

    fn decode(&self, data: &[u8]) -> Vec<f32> {
        ProductQuantizer::decode(self, data)
    }

    fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        ProductQuantizer::encode_query(self, query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pq = ProductQuantizer::new(128, 8, 8);
        assert_eq!(pq.dimension(), 128);
        assert_eq!(pq.sub_vectors(), 8);
        assert_eq!(pq.encoded_size(), 8); // 8 bytes for 8 sub-vectors
    }

    #[test]
    fn test_encode_decode() {
        let mut pq = ProductQuantizer::new(16, 4, 8);

        // Train on sample vectors
        let vectors: Vec<Vec<f32>> = (0..100)
            .map(|i| (0..16).map(|j| (i as f32 * 0.1 + j as f32).sin()).collect())
            .collect();

        let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();
        pq.train(&refs);

        // Encode and decode
        let original = vec![0.5; 16];
        let codes = pq.encode(&original);
        assert_eq!(codes.len(), 4);

        let decoded = pq.decode(&codes);
        assert_eq!(decoded.len(), 16);

        // Check that decoded is different from original (lossy compression)
        let diff: f32 = original
            .iter()
            .zip(decoded.iter())
            .map(|(o, d)| (o - d).abs())
            .sum();
        // There should be some difference due to quantization
        assert!(diff > 0.0);
    }

    #[test]
    fn test_compression_ratio() {
        let pq = ProductQuantizer::new(128, 8, 8);
        let original_size = 128 * 4; // 128 f32 = 512 bytes
        let compressed_size = pq.encoded_size(); // 8 bytes
        let ratio = original_size as f32 / compressed_size as f32;

        // Should be around 64x (512 / 8 = 64)
        assert!(ratio > 60.0 && ratio < 70.0);
    }

    #[test]
    fn test_default() {
        let pq = ProductQuantizer::default();
        assert_eq!(pq.dimension(), 128);
        assert_eq!(pq.sub_vectors(), 8);
    }
}
