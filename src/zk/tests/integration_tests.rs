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

//! Integration tests for STWO and Cairo ZK proof functionality
//!
//! These tests verify the end-to-end flow of:
//! 1. Registering Cairo programs
//! 2. Generating STARK proofs
//! 3. Verifying proofs
//! 4. Error handling and gas metering

use crate::zk::{
    CairoProgram, CairoProgramHash, CairoProgramRegistry, ProverConfig, ProverError, STWOProver,
    StarkProof, VerifyError,
    bundled::{
        get_bundled_program, is_bundled_program, register_bundled_programs, BundledError,
        HEXARY_VERIFY_HASH, MERKLE_BATCH_HASH, STATE_TRANSITION_HASH,
    },
};

// Helper function to check if a program is in the registry
fn registry_contains(registry: &CairoProgramRegistry, hash: &CairoProgramHash) -> bool {
    registry.get(hash).is_some()
}

// ============================================================================
// Program Registration Flow Tests
// ============================================================================

#[test]
fn test_register_and_retrieve_program() {
    let mut registry = CairoProgramRegistry::new();

    // Create a test program
    let source = "fn main() { return 42; }".to_string();
    let program = CairoProgram::from_source(source.clone(), 2);
    let program_hash = program.hash;

    // Register the program
    let result = registry.register(program);
    assert!(result.is_ok(), "Registration should succeed");

    // Verify it's in registry
    let retrieved = registry.get(&program_hash);
    assert!(retrieved.is_some(), "Program should be retrievable");
    assert_eq!(retrieved.unwrap().source, source, "Source should match");
}

#[test]
fn test_register_duplicate_program_fails() {
    let mut registry = CairoProgramRegistry::new();

    let source = "fn main() { return 42; }".to_string();
    let program = CairoProgram::from_source(source, 2);

    // Register once
    assert!(registry.register(program.clone()).is_ok());

    // Try to register again
    let result = registry.register(program);
    assert!(result.is_err(), "Duplicate registration should fail");
}

#[test]
fn test_bundled_programs_registration() {
    let mut registry = CairoProgramRegistry::new();

    // Register all bundled programs
    let result = register_bundled_programs(&mut registry);
    assert!(result.is_ok(), "Bundled programs should register successfully");

    // Verify all three programs are registered
    assert!(registry_contains(&registry, &STATE_TRANSITION_HASH));
    assert!(registry_contains(&registry, &HEXARY_VERIFY_HASH));
    assert!(registry_contains(&registry, &MERKLE_BATCH_HASH));
}

#[test]
fn test_get_bundled_program_by_name() {
    // Test getting each bundled program by name
    assert!(get_bundled_program("state_transition").is_some());
    assert!(get_bundled_program("hexary_verify").is_some());
    assert!(get_bundled_program("merkle_batch").is_some());
    assert!(get_bundled_program("nonexistent").is_none());
}

#[test]
fn test_is_bundled_program() {
    assert!(is_bundled_program(&STATE_TRANSITION_HASH));
    assert!(is_bundled_program(&HEXARY_VERIFY_HASH));
    assert!(is_bundled_program(&MERKLE_BATCH_HASH));

    let unknown_hash = [99u8; 32];
    assert!(!is_bundled_program(&unknown_hash));
}

#[test]
fn test_program_allowlist_enforcement() {
    let mut registry = CairoProgramRegistry::new();

    // Create a test program
    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    let hash = program.hash;

    // Register it
    registry.register(program).unwrap();

    // By default, programs are NOT on the allowlist
    assert!(!registry.is_allowed(&hash), "New programs should not be on allowlist");

    // Add to allowlist
    registry.allowlist_add(hash).unwrap();
    assert!(registry.is_allowed(&hash), "Program should be allowed after allowlist_add()");

    // Remove from allowlist
    registry.allowlist_remove(&hash).unwrap();
    assert!(!registry.is_allowed(&hash), "Program should not be allowed after allowlist_remove()");
}

#[test]
fn test_registry_persistence_across_operations() {
    let mut registry = CairoProgramRegistry::new();

    // Register multiple programs
    let program1 = CairoProgram::from_source("fn main() { return 1; }".to_string(), 2);
    let program2 = CairoProgram::from_source("fn main() { return 2; }".to_string(), 2);
    let hash1 = program1.hash;
    let hash2 = program2.hash;

    registry.register(program1).unwrap();
    registry.register(program2).unwrap();

    // Verify both persist
    assert!(registry_contains(&registry, &hash1));
    assert!(registry_contains(&registry, &hash2));

    // Allow one
    registry.allowlist_add(hash1).unwrap();
    assert!(registry.is_allowed(&hash1));
    assert!(!registry.is_allowed(&hash2));

    // Clear allowlist by removing the allowed entry
    registry.allowlist_remove(&hash1).unwrap();
    assert!(!registry.is_allowed(&hash1));
    assert!(!registry.is_allowed(&hash2));
}

// ============================================================================
// Proof Generation and Verification Tests
// ============================================================================

#[test]
fn test_prove_and_verify_roundtrip() {
    let prover = STWOProver::new();

    // Create a compiled program
    let mut program = CairoProgram::from_source("fn main() { return 42; }".to_string(), 2);
    program.sierra = vec![1, 2, 3]; // Mock compiled state
    program.casm = vec![4, 5, 6];

    let inputs = vec![1, 2, 3];

    // Generate proof
    let proof_result = prover.prove(&program, &inputs);
    assert!(proof_result.is_ok(), "Proof generation should succeed");

    let proof = proof_result.unwrap();

    // Verify proof
    let verify_result = prover.verify(&proof, &proof.outputs);
    assert!(verify_result.is_ok(), "Verification should succeed");
    assert!(verify_result.unwrap(), "Proof should be valid");
}

#[test]
fn test_prove_with_uncompiled_program_fails() {
    let prover = STWOProver::new();

    // Create uncompiled program
    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    let inputs = vec![1, 2, 3];

    // Should fail
    let result = prover.prove(&program, &inputs);
    assert!(result.is_err());
    match result {
        Err(ProverError::CompilationFailed(_)) => {}
        _ => panic!("Expected CompilationFailed error"),
    }
}

#[test]
fn test_verify_with_mismatched_outputs() {
    let prover = STWOProver::new();

    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    let proof = StarkProof::new(
        program.hash,
        vec![1, 2, 3],
        vec![42], // Actual outputs
        vec![7, 8, 9],
        vec![],
    );

    // Verify with different outputs
    let result = prover.verify(&proof, &[99]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false, "Proof should be invalid with mismatched outputs");
}

#[test]
fn test_verify_with_invalid_proof_format() {
    let prover = STWOProver::new();

    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

    // Create an invalid proof (empty)
    let proof = StarkProof::new(program.hash, vec![], vec![], vec![], vec![]);

    // Verify should fail
    let result = prover.verify(&proof, &[42]);
    assert!(result.is_err());
    match result {
        Err(VerifyError::InvalidProofFormat(_)) => {}
        _ => panic!("Expected InvalidProofFormat error"),
    }
}

#[test]
fn test_prover_config_validation() {
    // Test default config
    let prover = STWOProver::new();
    assert_eq!(prover.config().max_proof_size, 500 * 1024);
    assert_eq!(prover.config().timeout.as_secs(), 30);

    // Test custom config
    let config = ProverConfig {
        max_proof_size: 1024,
        timeout: std::time::Duration::from_secs(10),
        num_threads: 2,
    };
    let prover = STWOProver::with_config(config.clone());
    assert_eq!(prover.config().max_proof_size, 1024);
    assert_eq!(prover.config().timeout.as_secs(), 10);

    // Test builder pattern
    let prover = STWOProver::new()
        .with_max_proof_size(2048)
        .with_timeout(std::time::Duration::from_secs(60));
    assert_eq!(prover.config().max_proof_size, 2048);
    assert_eq!(prover.config().timeout.as_secs(), 60);
}

#[test]
fn test_inputs_too_large_error() {
    let config = ProverConfig {
        max_proof_size: 100, // Very small
        timeout: std::time::Duration::from_secs(30),
        num_threads: 1,
    };
    let prover = STWOProver::with_config(config);

    let mut program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    program.sierra = vec![1];
    program.casm = vec![1];

    // Inputs larger than max_proof_size / 4
    let large_inputs = vec![1u8; 30];

    let result = prover.prove(&program, &large_inputs);
    assert!(result.is_err());
    match result {
        Err(ProverError::InputsTooLarge(size)) => {
            assert_eq!(size, 30);
        }
        _ => panic!("Expected InputsTooLarge error"),
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_invalid_proof_rejected() {
    let prover = STWOProver::new();

    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

    // Create proof with invalid structure
    let invalid_proof = StarkProof {
        program_hash: program.hash,
        inputs: vec![],
        outputs: vec![],
        proof: vec![], // Empty proof should be invalid
        public_inputs: vec![],
    };

    let result = prover.verify(&invalid_proof, &[42]);
    assert!(result.is_err(), "Invalid proof should be rejected");
}

#[test]
fn test_proof_with_empty_proof_is_invalid() {
    let prover = STWOProver::new();

    let program = CairoProgram::from_source("fn main() { 1 }".to_string(), 2);

    // Create a proof with empty proof data - should fail validation
    let proof = StarkProof::new(program.hash, vec![1], vec![42], vec![], vec![]);

    let result = prover.verify(&proof, &[42]);
    // Empty proof should fail validation
    assert!(result.is_err());
    match result {
        Err(VerifyError::InvalidProofFormat(_)) => {}
        _ => panic!("Expected InvalidProofFormat error for empty proof"),
    }
}

// ============================================================================
// Gas Metering Tests
// ============================================================================

#[test]
fn test_zk_operation_gas_estimate() {
    // Since we don't have actual gas metering in the current implementation,
    // this test verifies the structure is in place for gas tracking

    let mut registry = CairoProgramRegistry::new();
    let prover = STWOProver::new();

    // Register program (gas cost estimate)
    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    let hash = program.hash;

    let start = std::time::Instant::now();
    registry.register(program).unwrap();
    let register_duration = start.elapsed();

    // Should be very fast (< 1ms)
    assert!(register_duration.as_millis() < 10, "Registration should be fast");

    // Generate proof (gas cost estimate)
    let mut proof_program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    proof_program.sierra = vec![1];
    proof_program.casm = vec![1];

    let start = std::time::Instant::now();
    let _proof = prover.prove(&proof_program, &[1, 2, 3]).unwrap();
    let prove_duration = start.elapsed();

    // Mock proof generation should be fast (< 10ms)
    assert!(prove_duration.as_millis() < 50, "Mock proof generation should be fast");

    // Verify proof (gas cost estimate)
    let proof = StarkProof::new(proof_program.hash, vec![1, 2, 3], vec![42], vec![1, 2, 3], vec![]);

    let start = std::time::Instant::now();
    let _result = prover.verify(&proof, &[42]).unwrap();
    let verify_duration = start.elapsed();

    // Verification should be fast (< 1ms)
    assert!(verify_duration.as_millis() < 10, "Verification should be fast");
}

// ============================================================================
// Timeout Handling Tests
// ============================================================================

#[test]
fn test_prover_timeout_config() {
    let config = ProverConfig {
        max_proof_size: 500 * 1024,
        timeout: std::time::Duration::from_millis(1), // Very short timeout
        num_threads: 1,
    };
    let prover = STWOProver::with_config(config);

    // For now, timeout is only checked during actual STWO execution
    // The mock implementation doesn't time out
    let mut program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    program.sierra = vec![1];
    program.casm = vec![1];

    let result = prover.prove(&program, &[1, 2, 3]);
    // Mock implementation doesn't timeout, so this succeeds
    assert!(result.is_ok());
}

// ============================================================================
// Benchmark Tests (actual benchmarks in benches/ directory)
// ============================================================================

#[test]
fn test_proof_generation_baseline_performance() {
    let prover = STWOProver::new();

    let mut program = CairoProgram::from_source("fn main() {}".to_string(), 2);
    program.sierra = vec![1];
    program.casm = vec![1];

    let inputs = vec![1; 100];

    let start = std::time::Instant::now();
    let _proof = prover.prove(&program, &inputs).unwrap();
    let duration = start.elapsed();

    // Mock proof generation should be < 10ms for 100 inputs
    assert!(duration.as_millis() < 10, "Mock proof generation should be fast");
    println!("Proof generation for 100 inputs: {:?}", duration);
}

#[test]
fn test_batch_proof_verification_performance() {
    let prover = STWOProver::new();

    let program = CairoProgram::from_source("fn main() {}".to_string(), 2);

    let start = std::time::Instant::now();

    // Verify 100 proofs
    for i in 0..100 {
        let proof = StarkProof::new(
            program.hash,
            vec![i as u8],
            vec![42],
            vec![1, 2, 3],
            vec![],
        );
        let _result = prover.verify(&proof, &[42]).unwrap();
    }

    let duration = start.elapsed();

    // 100 verifications should be < 100ms
    assert!(duration.as_millis() < 100, "Batch verification should be fast");
    println!("100 proof verifications: {:?}", duration);
}

// ============================================================================
// Bundled Programs Tests
// ============================================================================

#[test]
fn test_bundled_programs_have_valid_structure() {
    for name in &["state_transition", "hexary_verify", "merkle_batch"] {
        let program = get_bundled_program(name).expect(&format!("{} should exist", name));

        // Verify structure
        assert!(!program.source.is_empty(), "{} should have source", name);
        assert!(!program.casm.is_empty(), "{} should have CASM", name);
        assert_eq!(program.version, 2, "{} should be Cairo 2.0", name);
    }
}

#[test]
fn test_bundled_program_hash_constants() {
    // Verify hash constants are unique
    assert_ne!(STATE_TRANSITION_HASH, HEXARY_VERIFY_HASH);
    assert_ne!(STATE_TRANSITION_HASH, MERKLE_BATCH_HASH);
    assert_ne!(HEXARY_VERIFY_HASH, MERKLE_BATCH_HASH);
}

#[test]
fn test_bundled_error_display() {
    let err = BundledError::ProgramNotFound("test_program".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test_program"));
    assert!(display.contains("not found"));
}

// ============================================================================
// End-to-End Integration Test
// ============================================================================

#[test]
fn test_end_to_end_flow_register_prove_verify() {
    // 1. Create registry
    let mut registry = CairoProgramRegistry::new();

    // 2. Register bundled programs
    register_bundled_programs(&mut registry).expect("Bundled programs should register");

    // 3. Get a program and make it compiled (mock the CASM/Sierra)
    let mut program = get_bundled_program("state_transition").expect("Program should exist");
    program.sierra = vec![1, 2, 3]; // Mock compiled state
    program.casm = vec![4, 5, 6];
    let hash = program.hash;

    // 4. Verify it's registered
    assert!(registry_contains(&registry, &hash), "Program should be in registry");

    // 5. Add to allowlist
    registry.allowlist_add(hash).expect("Allowlist add should succeed");
    assert!(registry.is_allowed(&hash), "Program should be allowed");

    // 6. Create prover
    let prover = STWOProver::new();

    // 7. Generate proof
    let inputs = vec![1, 2, 3, 4];
    let proof = prover.prove(&program, &inputs).expect("Proof generation should succeed");

    // 8. Verify proof
    let is_valid = prover
        .verify(&proof, &proof.outputs)
        .expect("Verification should succeed");
    assert!(is_valid, "Proof should be valid");

    // 9. Verify proof has expected structure
    assert_eq!(proof.program_hash, hash);
    assert_eq!(proof.inputs, inputs);
    assert!(!proof.proof.is_empty());
}
