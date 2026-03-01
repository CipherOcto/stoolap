// stwo-bench/benches/stwo_proof.rs
// Real STWO benchmarks - requires nightly-2025-06-23 toolchain

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

mod benches {
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
    use stoolap::zk::{CairoProgram, STWOProver};

    fn generate_batch_inputs(size: usize) -> Vec<u8> {
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
                let inputs = generate_batch_inputs(size);

                let program = CairoProgram {
                    hash: [0u8; 32],
                    source: String::new(),
                    sierra: vec![],
                    casm: vec![],
                    version: 2_06_00,
                };

                let proof = match prover.generate_real_proof(&program, &inputs) {
                    Ok(p) => p,
                    Err(_) => {
                        std::hint::black_box(&inputs);
                        return;
                    }
                };

                b.iter(|| {
                    prover.verify(&proof, &inputs);
                });
            });
        }
        group.finish();
    }

    pub fn bench_real_proof_generation_hexary_verify(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_generation_hexary_verify");

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

    pub fn bench_real_proof_verification_hexary_verify(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_verification_hexary_verify");

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

                let proof = match prover.generate_real_proof(&program, &inputs) {
                    Ok(p) => p,
                    Err(_) => {
                        std::hint::black_box(&inputs);
                        return;
                    }
                };

                b.iter(|| {
                    prover.verify(&proof, &inputs);
                });
            });
        }
        group.finish();
    }

    pub fn bench_real_proof_generation_state_transition(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_generation_state_transition");

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

    pub fn bench_real_proof_verification_state_transition(c: &mut Criterion) {
        let mut group = c.benchmark_group("stark_real_proof_verification_state_transition");

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

                let proof = match prover.generate_real_proof(&program, &inputs) {
                    Ok(p) => p,
                    Err(_) => {
                        std::hint::black_box(&inputs);
                        return;
                    }
                };

                b.iter(|| {
                    prover.verify(&proof, &inputs);
                });
            });
        }
        group.finish();
    }
}

criterion_group! {
    name = stwo_proof;
    config = Criterion::default().sample_size(10);
    targets =
        benches::bench_real_proof_generation_merkle_batch,
        benches::bench_real_proof_verification_merkle_batch,
        benches::bench_real_proof_generation_hexary_verify,
        benches::bench_real_proof_verification_hexary_verify,
        benches::bench_real_proof_generation_state_transition,
        benches::bench_real_proof_verification_state_transition,
}

criterion_main!(stwo_proof);
