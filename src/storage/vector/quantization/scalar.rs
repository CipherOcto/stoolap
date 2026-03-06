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

//! Scalar Quantization (SQ) implementation
//!
//! SQ maps f32 values to lower-precision integers (e.g., uint8).
//! Provides 4x compression with low quality loss.

/// Scalar Quantizer: maps f32 vectors to uint8 vectors
///
/// Encoding: maps float range [min, max] to []
///0, 255 This achieves 4x compression (float32 → uint8)
pub struct ScalarQuantizer {
    dimension: usize,
    min_val: f32,
    max_val: f32,
    scale: f32,
}

impl ScalarQuantizer {
    /// Create new scalar quantizer with default range [-1.0, 1.0]
    pub fn new(dimension: usize) -> Self {
        Self::with_range(dimension, -1.0, 1.0)
    }

    /// Create scalar quantizer with custom range
    pub fn with_range(dimension: usize, min_val: f32, max_val: f32) -> Self {
        let scale = if max_val > min_val {
            255.0 / (max_val - min_val)
        } else {
            1.0
        };
        Self {
            dimension,
            min_val,
            max_val,
            scale,
        }
    }

    /// Create scalar quantizer from computed statistics
    pub fn with_stats(dimension: usize, min_val: f32, max_val: f32) -> Self {
        Self::with_range(dimension, min_val, max_val)
    }

    /// Get dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Get min value
    pub fn min_val(&self) -> f32 {
        self.min_val
    }

    /// Get max value
    pub fn max_val(&self) -> f32 {
        self.max_val
    }

    /// Encode vector: map floats to uint8
    ///
    /// # Arguments
    /// * `vector` - f32 vector to encode
    ///
    /// # Returns
    /// Quantized vector where each value is in [0, 255]
    pub fn encode(&self, vector: &[f32]) -> Vec<u8> {
        vector
            .iter()
            .map(|&v| {
                let normalized = (v - self.min_val) * self.scale;
                normalized.clamp(0.0, 255.0) as u8
            })
            .collect()
    }

    /// Decode uint8 vector back to f32
    ///
    /// # Arguments
    /// * `data` - Encoded uint8 vector
    ///
    /// # Returns
    /// f32 vector reconstructed from quantized values
    pub fn decode(&self, data: &[u8]) -> Vec<f32> {
        data.iter()
            .map(|&v| {
                let normalized = v as f32 / 255.0;
                self.min_val + normalized * (self.max_val - self.min_val)
            })
            .collect()
    }

    /// Encode query vector the same way as stored vectors
    pub fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        self.encode(query)
    }

    /// Get size of encoded vector in bytes
    pub fn encoded_size(&self) -> usize {
        self.dimension // 1 byte per dimension
    }

    /// Compute statistics from a set of vectors
    pub fn compute_stats(vectors: &[&[f32]]) -> (f32, f32) {
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;

        for vector in vectors {
            for &v in *vector {
                min_val = min_val.min(v);
                max_val = max_val.max(v);
            }
        }

        // Add small epsilon to max to ensure full range
        let epsilon = (max_val - min_val) * 0.01;
        (min_val, max_val + epsilon)
    }
}

impl Default for ScalarQuantizer {
    fn default() -> Self {
        Self::new(128)
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

impl Quantizer for ScalarQuantizer {
    fn encode(&self, vector: &[f32]) -> Vec<u8> {
        ScalarQuantizer::encode(self, vector)
    }

    fn decode(&self, data: &[u8]) -> Vec<f32> {
        ScalarQuantizer::decode(self, data)
    }

    fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        ScalarQuantizer::encode_query(self, query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let quantizer = ScalarQuantizer::new(4);
        let original = vec![-1.0, -0.5, 0.0, 0.5, 1.0];

        let encoded = quantizer.encode(&original);
        assert_eq!(encoded.len(), 5);

        let decoded = quantizer.decode(&encoded);
        assert_eq!(decoded.len(), 5);

        // Check approximate reconstruction
        for (o, d) in original.iter().zip(decoded.iter()) {
            assert!((o - d).abs() < 0.1);
        }
    }

    #[test]
    fn test_custom_range() {
        let quantizer = ScalarQuantizer::with_range(4, 0.0, 100.0);
        let original = vec![0.0, 25.0, 50.0, 75.0, 100.0];

        let encoded = quantizer.encode(&original);
        let decoded = quantizer.decode(&encoded);

        // Check approximate reconstruction
        for (o, d) in original.iter().zip(decoded.iter()) {
            assert!((o - d).abs() < 1.0);
        }
    }

    #[test]
    fn test_compute_stats() {
        let v1 = vec![1.0, 2.0, 3.0];
        let v2 = vec![4.0, 5.0, 6.0];
        let vectors: Vec<&[f32]> = vec![&v1, &v2];
        let (min, max) = ScalarQuantizer::compute_stats(&vectors);
        assert!((min - 1.0).abs() < 0.01);
        assert!((max - 6.0).abs() < 0.1);
    }

    #[test]
    fn test_dimension_768() {
        let quantizer = ScalarQuantizer::new(768);
        let vector = vec![0.0; 768];
        let encoded = quantizer.encode(&vector);
        assert_eq!(encoded.len(), 768);
    }
}
