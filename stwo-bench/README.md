# STWO Real Benchmarks

This crate benchmarks real STARK proof generation and verification using the STWO prover.

## Requirements

### Rust Toolchain
- **Nightly version:** `nightly-2025-06-23` (pinned in `rust-toolchain.toml`)
- This specific version is required because the STWO prover uses unstable Rust features

### External Dependencies
- **scarb** - Cairo package manager
- **starknet-foundry** - StarkNet development framework

These are required because STWO compiles Cairo programs to CASM at runtime.

## Setup

1. Install the required Rust nightly toolchain:
   ```bash
   rustup install nightly-2025-06-23
   ```

2. Install scarb:
   ```bash
   # Follow instructions at https://docs.swmansion.com/scarb/
   ```

3. Install starknet-foundry:
   ```bash
   # Follow instructions at https://github.com/foundry-rs/starknet-foundry
   ```

## Running Benchmarks

```bash
cd stwo-bench
cargo bench
```

The `rust-toolchain.toml` file will automatically use the correct nightly version when running cargo commands in this directory.

## Benchmark Results

The benchmarks measure:
- **Real proof generation** - Actual STWO prover execution time
- **Real proof verification** - STWO verifier execution time

For each of 3 Cairo programs:
- merkle_batch
- hexary_verify
- state_transition

At 3 batch sizes: 10, 100, 1000

## Troubleshooting

### Compilation errors about unstable features
Ensure you're using `nightly-2025-06-23`. Check with:
```bash
rustup show
```

### Missing scarb or starknet-foundry
Install them according to their documentation. The prover requires these tools to compile Cairo to CASM.
