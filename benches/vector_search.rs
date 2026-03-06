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

//! Vector search benchmark
//!
//! Run with: cargo bench --bench vector_search
//! Verify: <50ms query latency requirement

use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use stoolap::storage::vector::{VectorConfig, VectorMvcc, VectorSearch};

/// Benchmark vector search latency
fn bench_vector_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Search");

    // Setup: Create database with 1000 vectors (dimension 128)
    let config = VectorConfig::new(128);
    let mvcc = Arc::new(VectorMvcc::new(config.clone()));

    // Insert 1000 vectors
    for i in 0..1000 {
        let embedding: Vec<f32> = (0..128).map(|j| ((i * j) % 1000) as f32 / 1000.0).collect();
        mvcc.insert(i, embedding).unwrap();
    }

    let search = VectorSearch::new(mvcc.clone(), config.clone());

    // Warmup
    for _ in 0..10 {
        let _ = search.search(&[0.5; 128], 10);
    }

    // Benchmark: Single vector search (k=10)
    group.bench_function("search_1000_vectors_k10", |b| {
        b.iter(|| {
            let query = [0.5; 128];
            search.search(&query, 10)
        })
    });

    // Benchmark: Different k values
    for k in &[1, 5, 10, 50, 100] {
        group.bench_function(format!("search_k{}", k), |b| {
            b.iter(|| {
                let query = [0.5; 128];
                search.search(&query, *k)
            })
        });
    }

    group.finish();
}

/// Benchmark vector insert throughput
fn bench_vector_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Insert");

    // Setup: Create fresh MVCC
    let config = VectorConfig::new(128);

    // Benchmark: Single insert
    group.bench_function("insert_single", |b| {
        let mvcc = Arc::new(VectorMvcc::new(config.clone()));
        let mut counter = 0;

        b.iter(|| {
            counter += 1;
            let embedding: Vec<f32> = vec![0.1; 128];
            mvcc.insert(counter, embedding).ok();
        })
    });

    // Benchmark: Batch insert (100 at a time)
    group.bench_function("insert_batch_100", |b| {
        b.iter(|| {
            let mvcc = Arc::new(VectorMvcc::new(config.clone()));
            for i in 0..100 {
                let embedding: Vec<f32> = vec![0.1; 128];
                mvcc.insert(i, embedding).ok();
            }
        })
    });

    group.finish();
}

/// Benchmark vector search with different dataset sizes
fn bench_vector_search_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Search Scaling");

    for size in &[100, 500, 1000, 5000] {
        let config = VectorConfig::new(128);
        let mvcc = Arc::new(VectorMvcc::new(config.clone()));

        // Insert vectors
        for i in 0..*size {
            let embedding: Vec<f32> = (0..128).map(|j| ((i * j) % 1000) as f32 / 1000.0).collect();
            mvcc.insert(i as i64, embedding).unwrap();
        }

        let search = VectorSearch::new(mvcc, config);

        group.bench_function(format!("search_{}_vectors", size), |b| {
            b.iter(|| {
                let query = [0.5; 128];
                search.search(&query, 10)
            })
        });
    }

    group.finish();
}

/// Benchmark MVCC delete operation
fn bench_vector_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vector Delete");

    let config = VectorConfig::new(128);
    let mvcc = Arc::new(VectorMvcc::new(config.clone()));

    // Insert 100 vectors
    for i in 0..100 {
        let embedding: Vec<f32> = vec![0.1; 128];
        mvcc.insert(i, embedding).unwrap();
    }

    group.bench_function("delete_single", |b| {
        b.iter(|| {
            // Delete different IDs each time
            let id = (rand_simple() % 100) as i64;
            mvcc.delete(id).ok();
        })
    });

    group.finish();
}

/// Simple random number generator (no external deps)
fn rand_simple() -> usize {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    nanos.wrapping_mul(1103515245).wrapping_add(12345)
}

criterion_group!(
    benches,
    bench_vector_search,
    bench_vector_insert,
    bench_vector_search_scaling,
    bench_vector_delete
);
criterion_main!(benches);
