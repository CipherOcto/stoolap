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

//! Binary Quantization implementation

/// Binary Quantizer: maps f32 vectors to bitstreams
///
/// Encoding: positive values → 1, negative/zero → 0
/// This achieves 32x compression for 768-dim vectors (768 bits = 96 bytes)
pub struct BinaryQuantizer {
    dimension: usize,
}

impl BinaryQuantizer {
    /// Create new binary quantizer
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Get dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Encode vector: positive → 1, negative/zero → 0
    ///
    /// # Arguments
    /// * `vector` - f32 vector to encode
    ///
    /// # Returns
    /// Bitstream where bit i corresponds to dimension i
    pub fn encode(&self, vector: &[f32]) -> Vec<u8> {
        let bits = vector.len();
        let bytes = bits.div_ceil(8);
        let mut result = vec![0u8; bytes];

        for (i, &v) in vector.iter().enumerate() {
            if v > 0.0 {
                result[i / 8] |= 1 << (i % 8);
            }
        }
        result
    }

    /// Decode bitstream back to f32 vector
    ///
    /// # Arguments
    /// * `data` - Encoded bitstream
    ///
    /// # Returns
    /// f32 vector where 1 → 1.0, 0 → -1.0
    pub fn decode(&self, data: &[u8]) -> Vec<f32> {
        let mut result = vec![0.0; self.dimension];
        for i in 0..self.dimension {
            let byte = data[i / 8];
            result[i] = if byte & (1 << (i % 8)) != 0 {
                1.0
            } else {
                -1.0
            };
        }
        result
    }

    /// Encode query vector the same way as stored vectors
    pub fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        self.encode(query)
    }

    /// Get size of encoded vector in bytes
    pub fn encoded_size(&self) -> usize {
        self.dimension.div_ceil(8)
    }
}

impl Default for BinaryQuantizer {
    fn default() -> Self {
        Self { dimension: 128 }
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

impl Quantizer for BinaryQuantizer {
    fn encode(&self, vector: &[f32]) -> Vec<u8> {
        BinaryQuantizer::encode(self, vector)
    }

    fn decode(&self, data: &[u8]) -> Vec<f32> {
        BinaryQuantizer::decode(self, data)
    }

    fn encode_query(&self, query: &[f32]) -> Vec<u8> {
        BinaryQuantizer::encode_query(self, query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let quantizer = BinaryQuantizer::new(8);
        let original = vec![1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0];

        let encoded = quantizer.encode(&original);
        assert_eq!(encoded.len(), 1); // 8 bits = 1 byte

        let decoded = quantizer.decode(&encoded);
        assert_eq!(decoded.len(), 8);
        assert_eq!(decoded[0], 1.0);
        assert_eq!(decoded[1], -1.0);
        assert_eq!(decoded[2], 1.0);
        assert_eq!(decoded[3], -1.0);
    }

    #[test]
    fn test_encode_zero() {
        let quantizer = BinaryQuantizer::new(4);
        let zero = vec![0.0, 0.0, 0.0, 0.0];
        let encoded = quantizer.encode(&zero);
        // All zeros should encode to all 0s
        assert_eq!(encoded[0], 0b0000);
    }

    #[test]
    fn test_encode_positive() {
        let quantizer = BinaryQuantizer::new(4);
        let positive = vec![0.1, 0.2, 0.3, 0.4];
        let encoded = quantizer.encode(&positive);
        // All positive should encode to all 1s
        assert_eq!(encoded[0], 0b1111);
    }

    #[test]
    fn test_dimension_768() {
        let quantizer = BinaryQuantizer::new(768);
        let vector = vec![1.0; 768];
        let encoded = quantizer.encode(&vector);
        assert_eq!(encoded.len(), 96); // (768 + 7) / 8 = 96
    }
}
