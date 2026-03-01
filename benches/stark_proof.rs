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

//! Benchmarks for STARK proof generation and verification
//!
//! Run with: cargo bench --bench stark_proof --features zk
//!
//! Note: The zk feature requires the stwo crate which currently has
//! compilation issues with recent Rust nightly versions.
//! These benchmarks include both mock and real proof generation for testing purposes.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

#[cfg(feature = "zk")]
mod mock_benches {
    use super::*;
    use stoolap::zk::{CairoProgram, STWOProver, STWOVerifier};

    fn generate_batch_inputs(size: usize) -> Vec<u8> {
        // Generate inputs for batch size
        let mut inputs = Vec::new();
        for i in 1..=size {
            inputs.extend_from_slice(&i.to_le_bytes());
        }
        inputs
    }

    pub fn bench_mock_proof_generation_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_mock_proof_generation_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let prover = STWOProver::new();
                let inputs = generate_batch_inputs(size);

                // Create minimal program for benchmark
                let program = CairoProgram {
                    hash: [0u8; 32],
                    source: String::new(),
                    sierra: vec![],
                    casm: vec![],
                    version: 2_06_00,
                };

                b.iter(|| {
                    prover.generate_mock_proof(&program, &inputs);
                });
            });
        }
        group.finish();
    }

    pub fn bench_mock_proof_verification_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_mock_proof_verification_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let prover = STWOProver::new();
                let inputs = generate_batch_inputs(size);

                let program = CairoProgram {
                    hash: [0u8; 32],
                    source: String::new(),
                    sierra: vec![],
                    casm: vec![],
                    version: 2_06_00,
                };

                // Generate proof first
                let proof = prover.generate_mock_proof(&program, &inputs).unwrap();

                b.iter(|| {
                    prover.verify(&proof, &inputs);
                });
            });
        }
        group.finish();
    }
}

#[cfg(feature = "zk")]
mod real_benches {
    use super::*;
    use stoolap::zk::{CairoProgram, STWOProver, STWOVerifier};

    fn generate_batch_inputs(size: usize) -> Vec<u8> {
        // Generate inputs for batch size
        let mut inputs = Vec::new();
        for i in 1..=size {
            inputs.extend_from_slice(&i.to_le_bytes());
        }
        inputs
    }

    pub fn bench_real_proof_generation_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_generation_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let prover = STWOProver::new();
                let inputs = generate_batch_inputs(size);

                let program = CairoProgram {
                    hash: [0u8; 32],
                    source: String::new(),
                    sierra: vec![],
                    casm: vec![],
                    version: 2_06_00,
                };

                b.iter(|| {
                    prover.generate_real_proof(&program, &inputs);
                });
            });
        }
        group.finish();
    }

    pub fn bench_real_proof_verification_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_verification_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let prover = STWOProver::new();
                let verifier = STWOVerifier::new();
                let inputs = generate_batch_inputs(size);

                let program = CairoProgram {
                    hash: [0u8; 32],
                    source: String::new(),
                    sierra: vec![],
                    casm: vec![],
                    version: 2_06_00,
                };

                // Generate real proof first
                let proof = prover.generate_real_proof(&program, &inputs).unwrap();

                b.iter(|| {
                    verifier.verify(&proof, &inputs);
                });
            });
        }
        group.finish();
    }
}

// Stub benchmarks for when zk feature is not available
// These allow the benchmark to compile even when stwo has issues
#[cfg(not(feature = "zk"))]
mod stub_benches {
    use super::*;

    fn generate_batch_inputs(size: usize) -> Vec<u8> {
        let mut inputs = Vec::new();
        for i in 1..=size {
            inputs.extend_from_slice(&i.to_le_bytes());
        }
        inputs
    }

    pub fn bench_mock_proof_generation_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_mock_proof_generation_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let inputs = generate_batch_inputs(size);

                b.iter(|| {
                    // Stub: just process the inputs without actual proof generation
                    std::hint::black_box(&inputs);
                    let mut sum = 0u8;
                    for &b in &inputs {
                        sum = sum.wrapping_add(b);
                    }
                    sum
                });
            });
        }
        group.finish();
    }

    pub fn bench_mock_proof_verification_merkle_batch(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_mock_proof_verification_merkle_batch");

        for size in [10, 100, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                let inputs = generate_batch_inputs(size);

                b.iter(|| {
                    // Stub: just verify inputs exist
                    std::hint::black_box(&inputs);
                    inputs.len() > 0
                });
            });
        }
        group.finish();
    }
}

#[cfg(feature = "zk")]
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets =
        mock_benches::bench_mock_proof_generation_merkle_batch,
        mock_benches::bench_mock_proof_verification_merkle_batch,
        real_benches::bench_real_proof_generation_merkle_batch,
        real_benches::bench_real_proof_verification_merkle_batch,
}

#[cfg(not(feature = "zk"))]
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets =
        stub_benches::bench_mock_proof_generation_merkle_batch,
        stub_benches::bench_mock_proof_verification_merkle_batch,
}

criterion_main!(benches);
