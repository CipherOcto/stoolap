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
pub mod bundled;
#[cfg(feature = "zk")]
pub mod cairo;
#[cfg(feature = "commitment")]
pub mod commitment;
#[cfg(feature = "zk")]
pub mod compressed;
#[cfg(feature = "commitment")]
pub mod confidential;
#[cfg(feature = "zk")]
pub mod proof;
#[cfg(feature = "zk")]
pub mod prover;

// Plugin loading - doesn't require zk feature, uses dynamic loading
pub mod plugin;

#[cfg(test)]
#[cfg(feature = "zk")]
mod tests;

#[cfg(feature = "zk")]
pub use bundled::{
    get_bundled_program, get_bundled_program_name, is_bundled_program, register_bundled_programs,
    BundledError, HEXARY_VERIFY_CASM, HEXARY_VERIFY_HASH, MERKLE_BATCH_CASM, MERKLE_BATCH_HASH,
    STATE_TRANSITION_CASM, STATE_TRANSITION_HASH,
};
#[cfg(feature = "zk")]
pub use cairo::{
    CairoProgram, CairoProgramHash, CairoProgramRegistry, CompileError, RegistryError,
};
#[cfg(feature = "commitment")]
pub use commitment::{
    open_commitment, open_commitment_batch, pedersen_commit, pedersen_commit_batch, Commitment,
};
#[cfg(feature = "zk")]
pub use compressed::{
    BatchVerifyError, BatchVerifyInput, CompressedProof, CompressedProofError,
    AVG_HEXARY_PROOF_SIZE, MAX_BATCH_SIZE, MAX_COMPRESSED_SIZE,
};
#[cfg(feature = "commitment")]
pub use confidential::{ConfidentialResult, EncryptedFilter, EncryptedQuery, FilterOp, RangeProof};
#[cfg(feature = "zk")]
pub use proof::{
    CairoProgramForRegistration, ProofSummary, ProofValidationError, SerializationError,
    SolanaSerialize, StarkProof, ZKOperation, MAX_INPUTS_SIZE, MAX_OUTPUTS_SIZE, MAX_PROOF_SIZE,
    MAX_PUBLIC_INPUTS_SIZE,
};
#[cfg(feature = "zk")]
pub use prover::{ProverConfig, ProverError, STWOProver, VerifyError};

// Plugin exports (always available)
pub use plugin::{load_plugin, PluginError, STWOPlugin};
