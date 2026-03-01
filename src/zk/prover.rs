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

//! STWO prover wrapper
//!
//! Provides Rust interface to STWO's Circle STARK prover/verifier.

use std::time::Duration;

use crate::zk::cairo::CairoProgram;
use crate::zk::proof::StarkProof;

/// Configuration for STWO prover
#[derive(Debug, Clone)]
pub struct ProverConfig {
    /// Maximum proof size in bytes
    pub max_proof_size: usize,

    /// Timeout for proof generation
    pub timeout: Duration,

    /// Number of threads for parallel proof generation
    pub num_threads: usize,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            max_proof_size: 500 * 1024, // 500 KB
            timeout: Duration::from_secs(30),
            num_threads: rayon::current_num_threads().max(1),
        }
    }
}

/// STWO prover for generating STARK proofs
///
/// This prover wraps the STWO library to generate and verify STARK proofs
/// for Cairo program executions.
#[derive(Debug, Clone)]
pub struct STWOProver {
    config: ProverConfig,
}

impl STWOProver {
    /// Create a new STWO prover with default configuration
    pub fn new() -> Self {
        Self {
            config: ProverConfig::default(),
        }
    }

    /// Create a new STWO prover with custom configuration
    pub fn with_config(config: ProverConfig) -> Self {
        Self { config }
    }

    /// Get the prover configuration
    pub fn config(&self) -> &ProverConfig {
        &self.config
    }

    /// Set the maximum proof size
    pub fn with_max_proof_size(mut self, size: usize) -> Self {
        self.config.max_proof_size = size;
        self
    }

    /// Set the proof generation timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Generate a STARK proof for a Cairo program
    ///
    /// This method executes the Cairo program with the given inputs
    /// and generates a STARK proof using the STWO prover.
    ///
    /// # Arguments
    ///
    /// * `program` - The compiled Cairo program to execute
    /// * `inputs` - Input data for the program execution
    ///
    /// # Returns
    ///
    /// A `StarkProof` containing the proof and execution outputs
    ///
    /// # Errors
    ///
    /// Returns `ProverError` if:
    /// - Program is not compiled (missing CASM)
    /// - Execution fails
    /// - Proof generation times out
    /// - Proof exceeds size limits
    pub fn prove(
        &self,
        program: &CairoProgram,
        inputs: &[u8],
    ) -> Result<StarkProof, ProverError> {
        // Check if program is compiled
        if !program.is_compiled() {
            return Err(ProverError::CompilationFailed(
                "Program must be compiled to CASM before proving".to_string(),
            ));
        }

        // Check input size
        if inputs.len() > self.config.max_proof_size / 4 {
            return Err(ProverError::InputsTooLarge(inputs.len()));
        }

        // For now, this is a stub that will be integrated with STWO later
        // The actual implementation will:
        // 1. Load CASM bytecode
        // 2. Execute program with inputs
        // 3. Generate STARK proof via STWO
        // 4. Return serialized proof

        // Placeholder: simulate proof generation
        let proof = self.generate_mock_proof(program, inputs)?;

        Ok(proof)
    }

    /// Verify a STARK proof
    ///
    /// Verifies that the STARK proof is valid and that the outputs
    /// match the expected values.
    ///
    /// # Arguments
    ///
    /// * `proof` - The STARK proof to verify
    /// * `expected_outputs` - Expected outputs from program execution
    ///
    /// # Returns
    ///
    /// `true` if the proof is valid and outputs match, `false` otherwise
    ///
    /// # Errors
    ///
    /// Returns `VerifyError` if:
    /// - Proof format is invalid
    /// - Proof verification fails
    /// - Outputs don't match
    pub fn verify(
        &self,
        proof: &StarkProof,
        expected_outputs: &[u8],
    ) -> Result<bool, VerifyError> {
        // Validate proof structure
        proof.validate().map_err(|e| VerifyError::InvalidProofFormat(format!("{:?}", e)))?;

        // For now, this is a stub that will be integrated with STWO later
        // The actual implementation will:
        // 1. Parse STARK proof
        // 2. Verify using STWO verifier
        // 3. Check outputs match

        // Placeholder: verify outputs match
        Ok(proof.outputs == expected_outputs)
    }

    /// Generate a mock STARK proof for testing
    ///
    /// This is a temporary implementation until STWO integration is complete.
    fn generate_mock_proof(
        &self,
        program: &CairoProgram,
        inputs: &[u8],
    ) -> Result<StarkProof, ProverError> {
        use std::time::SystemTime;

        // Combine program hash and inputs to create "proof"
        let proof_data = format!(
            "{:?}-{}-{:?}",
            program.hash,
            hex::encode(inputs),
            SystemTime::now()
        );

        // Create a simple proof hash
        let proof_bytes = proof_data.as_bytes().to_vec();

        Ok(StarkProof {
            program_hash: program.hash,
            inputs: inputs.to_vec(),
            outputs: vec![0x42], // Mock output
            proof: proof_bytes,
            public_inputs: vec![],
        })
    }

    /// Generate a real STARK proof using STWO prover
    ///
    /// This method uses the STWO Cairo prover to generate a real STARK proof
    /// for a Cairo program execution.
    #[cfg(feature = "zk")]
    fn generate_real_proof(
        &self,
        program: &CairoProgram,
        inputs: &[u8],
    ) -> Result<StarkProof, ProverError> {
        use stwo_cairo_prover::CairoProver;

        // Use STWO to generate real proof
        let prover = stwo_cairo_prover::CairoProver::new();

        // Compile program if needed
        let compiled = program.compile_to_casm()
            .map_err(|e| ProverError::CompilationFailed(e.to_string()))?;

        // Generate proof
        let proof_output = prover.prove(&compiled, inputs)
            .map_err(|e| ProverError::ProvingFailed(e.to_string()))?;

        Ok(StarkProof {
            program_hash: program.hash,
            inputs: inputs.to_vec(),
            outputs: proof_output.outputs,
            proof: proof_output.proof,
            public_inputs: proof_output.public_inputs,
        })
    }
}

impl Default for STWOProver {
    fn default() -> Self {
        Self::new()
    }
}

/// Error during proof generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProverError {
    /// Cairo program compilation failed
    CompilationFailed(String),
    /// Program execution failed
    ExecutionFailed(String),
    /// Proof generation timed out
    ProofGenerationTimeout,
    /// Out of memory during proving
    OutOfMemory,
    /// Input data too large
    InputsTooLarge(usize),
    /// STWO prover not found
    ProverNotFound,
    /// Invalid program state
    InvalidProgramState(String),
    /// Proving failed
    ProvingFailed(String),
}

impl std::fmt::Display for ProverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProverError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            ProverError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            ProverError::ProofGenerationTimeout => write!(f, "Proof generation timed out"),
            ProverError::OutOfMemory => write!(f, "Out of memory during proving"),
            ProverError::InputsTooLarge(size) => write!(f, "Inputs too large: {} bytes", size),
            ProverError::ProverNotFound => write!(f, "STWO prover not found"),
            ProverError::InvalidProgramState(msg) => write!(f, "Invalid program state: {}", msg),
            ProverError::ProvingFailed(msg) => write!(f, "Proving failed: {}", msg),
        }
    }
}

impl std::error::Error for ProverError {}

/// Error during proof verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// Invalid proof format
    InvalidProofFormat(String),
    /// Proof verification failed
    VerificationFailed,
    /// Outputs don't match expected
    OutputsMismatch,
    /// Invalid program hash
    InvalidProgramHash,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::InvalidProofFormat(msg) => write!(f, "Invalid proof format: {}", msg),
            VerifyError::VerificationFailed => write!(f, "Proof verification failed"),
            VerifyError::OutputsMismatch => write!(f, "Outputs don't match expected"),
            VerifyError::InvalidProgramHash => write!(f, "Invalid program hash"),
        }
    }
}

impl std::error::Error for VerifyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_creation() {
        let prover = STWOProver::new();
        assert_eq!(prover.config().max_proof_size, 500 * 1024);
    }

    #[test]
    fn test_prover_with_config() {
        let config = ProverConfig {
            max_proof_size: 1024,
            timeout: Duration::from_secs(10),
            num_threads: 4,
        };
        let prover = STWOProver::with_config(config.clone());
        assert_eq!(prover.config().max_proof_size, 1024);
    }

    #[test]
    fn test_prover_builder_pattern() {
        let prover = STWOProver::new()
            .with_max_proof_size(2048)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(prover.config().max_proof_size, 2048);
        assert_eq!(prover.config().timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_default_prover() {
        let prover = STWOProver::default();
        assert_eq!(prover.config().max_proof_size, 500 * 1024);
    }

    #[test]
    fn test_prove_with_uncompiled_program() {
        let prover = STWOProver::new();
        let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

        let result = prover.prove(&program, &[1, 2, 3]);
        assert!(result.is_err());
        match result {
            Err(ProverError::CompilationFailed(_)) => {}
            _ => panic!("Expected CompilationFailed error"),
        }
    }

    #[test]
    fn test_prove_with_mock_proof() {
        let prover = STWOProver::new();
        let mut program = CairoProgram::from_source("fn main() {}".to_string(), 2);
        // Mock compiled state
        program.sierra = vec![1, 2, 3];
        program.casm = vec![4, 5, 6];

        let result = prover.prove(&program, &[1, 2, 3]);
        assert!(result.is_ok());

        let proof = result.unwrap();
        assert_eq!(proof.inputs, vec![1, 2, 3]);
        assert_eq!(proof.program_hash, program.hash);
    }

    #[test]
    fn test_verify_valid_proof() {
        let prover = STWOProver::new();
        let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

        let proof = StarkProof::new(
            program.hash,
            vec![1, 2, 3],
            vec![42],
            vec![7, 8, 9],
            vec![],
        );

        // Verify with matching outputs
        let result = prover.verify(&proof, &[42]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_verify_outputs_mismatch() {
        let prover = STWOProver::new();
        let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

        let proof = StarkProof::new(
            program.hash,
            vec![1, 2, 3],
            vec![42],
            vec![7, 8, 9],
            vec![],
        );

        // Verify with different outputs
        let result = prover.verify(&proof, &[99]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_verify_invalid_proof() {
        let prover = STWOProver::new();
        let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

        // Create an invalid proof (empty)
        let proof = StarkProof::new(
            program.hash,
            vec![],
            vec![],
            vec![],
            vec![],
        );

        let result = prover.verify(&proof, &[42]);
        assert!(result.is_err());
        match result {
            Err(VerifyError::InvalidProofFormat(_)) => {}
            _ => panic!("Expected InvalidProofFormat error"),
        }
    }

    #[test]
    fn test_prover_error_display() {
        let err = ProverError::CompilationFailed("test error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Compilation failed"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_verify_error_display() {
        let err = VerifyError::OutputsMismatch;
        let display = format!("{}", err);
        assert!(display.contains("Outputs don't match"));
    }

    #[test]
    fn test_prover_config_timeout() {
        let config = ProverConfig {
            max_proof_size: 1024,
            timeout: Duration::from_millis(500),
            num_threads: 2,
        };

        assert_eq!(config.timeout, Duration::from_millis(500));
    }

    #[test]
    fn test_prover_config_threads() {
        let config = ProverConfig::default();
        assert!(config.num_threads > 0);
    }
}
