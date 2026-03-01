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

//! Zero-knowledge proof support using STWO and Cairo
//!
//! This module provides zk-SNARK functionality for:
//! - Proof compression (aggregate many HexaryProofs into one STARK proof)
//! - Confidential queries (prove query results without revealing data)
//! - L2 rollup (off-chain execution with on-chain verification)
//!
//! # Feature Flag
//!
//! This module requires the `zk` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! stoolap = { version = "0.3", features = ["zk"] }
//! ```
//!
//! # STWO Integration
//!
//! STWO (Stwo prover) is a Circle STARK prover/verifier written in Rust.
//! It enables efficient proof generation and verification for Cairo programs.

#[cfg(feature = "zk")]
pub mod cairo;
#[cfg(feature = "zk")]
pub mod prover;
#[cfg(feature = "zk")]
pub mod proof;

#[cfg(feature = "zk")]
pub use cairo::{CairoProgram, CairoProgramHash, CairoProgramRegistry, CompileError, RegistryError};
#[cfg(feature = "zk")]
pub use prover::{ProverConfig, ProverError, STWOProver, VerifyError};
#[cfg(feature = "zk")]
pub use proof::{
    CairoProgramForRegistration, MAX_INPUTS_SIZE, MAX_OUTPUTS_SIZE, MAX_PROOF_SIZE,
    MAX_PUBLIC_INPUTS_SIZE, ProofSummary, ProofValidationError, SerializationError,
    SolanaSerialize, StarkProof, ZKOperation,
};
