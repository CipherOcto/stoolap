// stwo-bench/src/main.rs
use criterion::criterion_main;

mod benches;
criterion_main!(benches::stwo_proof);
