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

//! Vector Quantization Module
//!
//! Provides compression for vector storage:
//! - Binary Quantization (BQ): 1 bit per dimension, 32x compression
//! - Scalar Quantization (SQ): 1 byte per dimension, 4x compression
//! - Product Quantization (PQ): Sub-vector quantization, 4-64x configurable

pub mod config;
pub mod distance;
pub mod product;
pub mod quantizer;
pub mod scalar;

pub use config::{QuantizationConfig, QuantizationType};
pub use distance::{euclidean_distance, hamming_distance, hamming_to_similarity};
pub use product::ProductQuantizer;
pub use quantizer::BinaryQuantizer;
pub use scalar::ScalarQuantizer;
