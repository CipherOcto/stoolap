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

//! Distance computation for quantized vectors

/// Compute Hamming distance between two binary vectors
///
/// Hamming distance = count of bits that differ
/// Lower = more similar
///
/// # Arguments
/// * `a` - First encoded vector
/// * `b` - Second encoded vector
///
/// # Returns
/// Number of bits that differ
pub fn hamming_distance(a: &[u8], b: &[u8]) -> usize {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");
    let mut distance = 0;
    for i in 0..a.len() {
        distance += (a[i] ^ b[i]).count_ones() as usize;
    }
    distance
}

/// Convert Hamming distance to similarity score
///
/// Returns value in range [0, 1] where:
/// - 1.0 = identical (distance = 0)
/// - 0.0 = completely different (distance = dimension)
///
/// # Arguments
/// * `distance` - Hamming distance
/// * `dimension` - Original vector dimension
pub fn hamming_to_similarity(distance: usize, dimension: usize) -> f32 {
    if dimension == 0 {
        return 1.0;
    }
    1.0 - (distance as f32 / dimension as f32)
}

/// Compute cosine similarity from binary vectors
///
/// For BQ vectors encoded as ±1, cosine similarity approximates to:
/// cos(θ) ≈ (matching_bits - non_matching_bits) / dimension
///         = (dimension - 2 * distance) / dimension
///         = 1 - 2 * (distance / dimension)
///
/// # Arguments
/// * `a` - First encoded vector
/// * `b` - Second encoded vector
/// * `dimension` - Original vector dimension
pub fn binary_cosine_similarity(a: &[u8], b: &[u8], dimension: usize) -> f32 {
    let distance = hamming_distance(a, b);
    1.0 - 2.0 * (distance as f32 / dimension as f32)
}

/// Compute squared Euclidean distance between two quantized vectors
///
/// Used for SQ (Scalar Quantization) distance calculation.
/// Returns squared distance (sqrt not needed for ranking).
///
/// # Arguments
/// * `a` - First quantized vector (decoded to f32)
/// * `b` - Second quantized vector (decoded to f32)
///
/// # Returns
/// Squared Euclidean distance
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");
    let mut sum = 0.0;
    for i in 0..a.len() {
        let diff = a[i] - b[i];
        sum += diff * diff;
    }
    sum
}

/// Compute cosine similarity between two vectors
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Cosine similarity in range [-1, 1]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = (norm_a * norm_b).sqrt();
    if denominator == 0.0 {
        return 0.0;
    }

    dot_product / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hamming_identical() {
        let a = vec![0b11111111, 0b11111111];
        let b = vec![0b11111111, 0b11111111];
        assert_eq!(hamming_distance(&a, &b), 0);
    }

    #[test]
    fn test_hamming_all_different() {
        let a = vec![0b11111111, 0b11111111];
        let b = vec![0b00000000, 0b00000000];
        assert_eq!(hamming_distance(&a, &b), 16);
    }

    #[test]
    fn test_hamming_mixed() {
        let a = vec![0b10101010];
        let b = vec![0b01010101];
        assert_eq!(hamming_distance(&a, &b), 8);
    }

    #[test]
    fn test_hamming_to_similarity() {
        assert_eq!(hamming_to_similarity(0, 100), 1.0);
        assert_eq!(hamming_to_similarity(50, 100), 0.5);
        assert_eq!(hamming_to_similarity(100, 100), 0.0);
    }

    #[test]
    fn test_binary_cosine_identical() {
        let a = vec![0b11111111];
        let b = vec![0b11111111];
        assert!((binary_cosine_similarity(&a, &b, 8) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_binary_cosine_opposite() {
        let a = vec![0b11111111];
        let b = vec![0b00000000];
        assert!((binary_cosine_similarity(&a, &b, 8) - (-1.0)).abs() < 0.001);
    }
}
