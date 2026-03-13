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

//! Benchmark for HexaryProof operations

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use stoolap::determ::{DetermRow, DetermValue};
use stoolap::trie::row_trie::RowTrie;

fn bench_proof_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_generation");

    for size in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut trie = RowTrie::new();
            let row_ids: Vec<i64> = (1..=size).collect();

            // Insert rows
            for &id in &row_ids {
                let row = DetermRow::from_values(vec![DetermValue::integer(id)]);
                trie.insert(id, row);
            }

            b.iter(|| {
                for &id in &row_ids {
                    std::hint::black_box(trie.get_hexary_proof(id));
                }
            });
        });
    }

    group.finish();
}

fn bench_proof_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_verification");

    for size in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut trie = RowTrie::new();
            let mut proofs = Vec::new();

            // Insert rows and generate proofs
            for id in 1..=size {
                let row = DetermRow::from_values(vec![DetermValue::integer(id)]);
                trie.insert(id, row);
                if let Some(proof) = trie.get_hexary_proof(id) {
                    proofs.push(proof);
                }
            }

            b.iter(|| {
                for proof in &proofs {
                    std::hint::black_box(proof.verify());
                }
            });
        });
    }

    group.finish();
}

fn bench_proof_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_serialization");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            use stoolap::trie::proof::SolanaSerialize;

            let mut trie = RowTrie::new();
            let mut proofs = Vec::new();

            // Insert rows and generate proofs
            for id in 1..=size {
                let row = DetermRow::from_values(vec![DetermValue::integer(id)]);
                trie.insert(id, row);
                if let Some(proof) = trie.get_hexary_proof(id) {
                    proofs.push(proof);
                }
            }

            b.iter(|| {
                for proof in &proofs {
                    std::hint::black_box(proof.serialize());
                }
            });
        });
    }

    group.finish();
}

fn bench_proof_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_deserialization");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            use stoolap::trie::proof::{HexaryProof, SolanaSerialize};

            let mut trie = RowTrie::new();
            let mut serialized = Vec::new();

            // Insert rows and serialize proofs
            for id in 1..=size {
                let row = DetermRow::from_values(vec![DetermValue::integer(id)]);
                trie.insert(id, row);
                if let Some(proof) = trie.get_hexary_proof(id) {
                    serialized.push(proof.serialize());
                }
            }

            b.iter(|| {
                for data in &serialized {
                    std::hint::black_box(HexaryProof::deserialize(data));
                }
            });
        });
    }

    group.finish();
}

fn bench_batch_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verification");

    for size in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            use stoolap::trie::proof::HexaryProof;

            let mut trie = RowTrie::new();
            let mut proofs = Vec::new();

            // Insert rows and generate proofs
            for id in 1..=size {
                let row = DetermRow::from_values(vec![DetermValue::integer(id)]);
                trie.insert(id, row);
                if let Some(proof) = trie.get_hexary_proof(id) {
                    proofs.push(proof);
                }
            }

            b.iter(|| {
                std::hint::black_box(HexaryProof::verify_batch_sequential(&proofs));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_proof_generation,
    bench_proof_verification,
    bench_proof_serialization,
    bench_proof_deserialization,
    bench_batch_verification
);

criterion_main!(benches);
