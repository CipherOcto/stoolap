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

//! Quantization configuration

use serde::{Deserialize, Serialize};

/// Quantization type
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum QuantizationType {
    /// Binary Quantization: 1 bit per dimension
    #[default]
    Binary,
    /// Scalar Quantization: 4 bits per dimension (future)
    Scalar,
    /// Product Quantization: sub-vector quantization (future)
    Product,
}

impl QuantizationType {
    /// Get compression ratio (original_size / compressed_size)
    pub fn compression_ratio(&self, dimension: usize) -> f32 {
        match self {
            QuantizationType::Binary => dimension as f32 / 8.0, // 32x for 768-dim
            QuantizationType::Scalar => dimension as f32 / (dimension / 2) as f32, // 4x
            QuantizationType::Product => 32.0,                  // default PQ
        }
    }
}

/// Quantization configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuantizationConfig {
    /// Type of quantization to use
    pub quantization_type: QuantizationType,
    /// Whether quantization is enabled
    pub enabled: bool,
    /// Vector dimension
    pub dimension: usize,
}

impl QuantizationConfig {
    /// Create new quantization config
    pub fn new(dimension: usize) -> Self {
        Self {
            quantization_type: QuantizationType::Binary,
            enabled: true,
            dimension,
        }
    }

    /// Create disabled config
    pub fn disabled() -> Self {
        Self {
            quantization_type: QuantizationType::Binary,
            enabled: false,
            dimension: 0,
        }
    }

    /// Get compressed size in bytes
    pub fn compressed_size(&self) -> usize {
        if !self.enabled {
            return 0;
        }
        match self.quantization_type {
            QuantizationType::Binary => self.dimension.div_ceil(8),
            QuantizationType::Scalar => self.dimension / 2,
            QuantizationType::Product => self.dimension / 8, // rough estimate
        }
    }

    /// Get original size in bytes
    pub fn original_size(&self) -> usize {
        self.dimension * 4 // f32 = 4 bytes
    }
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_compression() {
        let config = QuantizationConfig::new(768);
        assert!(config.enabled);
        assert_eq!(config.quantization_type, QuantizationType::Binary);
        assert_eq!(config.original_size(), 3072);
        assert_eq!(config.compressed_size(), 96); // (768 + 7) / 8 = 96
    }

    #[test]
    fn test_disabled_config() {
        let config = QuantizationConfig::disabled();
        assert!(!config.enabled);
        assert_eq!(config.compressed_size(), 0);
    }
}
