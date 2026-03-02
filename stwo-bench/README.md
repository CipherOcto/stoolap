# STWO Real Benchmarks

This crate benchmarks real STARK proof generation and verification using the STWO Circle STARK prover.

## Requirements

### Rust Toolchain
- **Nightly version:** `nightly-2025-06-23` (pinned in `rust-toolchain.toml`)
- This specific version is required because the STWO prover uses unstable Rust features (portable_simd, array_chunks)

### Dependencies
The crate pulls dependencies from GitHub:
- `stwo-cairo-prover` v1.1.0 (from starkware-libs/stwo-cairo)
- `stwo-cairo-adapter` v1.1.0
- `cairo-air` v1.1.0

## API Usage

The benchmarks use the STWO prover API:
- `prove_cairo::<Blake2sMerkleChannel>()` for proof generation
- `verify_cairo::<Blake2sMerkleChannel>()` for verification
- `ProverInput` from `stwo_cairo_adapter`
- `ProverParameters` with Blake2s channel

## Running Benchmarks

```bash
cd stwo-bench
cargo bench
```

The `rust-toolchain.toml` file will automatically use the correct nightly version when running cargo commands in this directory.

## Benchmark Results

Actual STWO proof generation and verification times:

| Operation | Time |
|-----------|------|
| Proof Generation (merkle_batch) | ~25-28 seconds |
| Proof Verification (merkle_batch) | ~15 ms |

Benchmarks cover:
- **merkle_batch** - Merkle batch verification program
- **hexary_verify** - Hexary proof verification
- **state_transition** - State transition program

## Architecture

```
stwo-bench/
├── Cargo.toml          # Dependencies (GitHub v1.1.0)
├── rust-toolchain.toml # Pins nightly-2025-06-23
├── stwo_proof.rs       # Benchmark implementation
└── README.md           # This file
```

## Troubleshooting

### Compilation errors about unstable features
Ensure you're using `nightly-2025-06-23`. Check with:
```bash
rustup show
```

### Benchmark timing
Proof generation takes ~25-30 seconds per iteration. Use sample_size(3) for faster testing.
