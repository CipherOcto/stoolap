# Mission: Real STWO Benchmarks Sub-crate

## Status
Completed

## RFC
RFC-0106: STARK Proof Benchmarks with Real STWO

## Acceptance Criteria
- [x] Create stwo-bench sub-crate with GitHub dependencies
- [x] Implement real proof generation using prove_cairo
- [x] Implement real proof verification using verify_cairo
- [x] Pin nightly-2025-06-23 toolchain
- [x] Document benchmark results

## Claimant
AI Agent (Claude)

## Pull Request
N/A (Implemented directly in feature branch)

## Implementation Notes

**Files Created:**
- `stwo-bench/Cargo.toml` - Sub-crate with GitHub v1.1.0 dependencies
- `stwo-bench/rust-toolchain.toml` - Pins nightly-2025-06-23
- `stwo-bench/stwo_proof.rs` - Real STWO benchmark implementation
- `stwo-bench/README.md` - Documentation

**Benchmark Results:**

| Operation | Time |
|-----------|------|
| Proof Generation (merkle_batch) | ~25-28 seconds |
| Proof Verification (merkle_batch) | ~15 ms |

**API Used:**
- `prove_cairo::<Blake2sMerkleChannel>()` from stwo-cairo-prover
- `verify_cairo::<Blake2sMerkleChannel>()` from cairo-air
- `ProverInput` from stwo-cairo-adapter
- `ProverParameters` with Blake2s channel

**Dependencies:**
```toml
stwo-cairo-prover = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
stwo-cairo-adapter = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
cairo-air = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
```

## Completion Date
2026-03-01
