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

## API Status: crates.io v1.1 Limitation

**Important Finding:** The `stwo-cairo-prover` crate from crates.io (v1.1) does **not** re-export the `adapter` module, which contains `ProverInput`. This is required for real proof generation.

The main API available in crates.io v1.1:
- `stwo_cairo_prover::prover::ProverParameters` - Available
- `stwo_cairo_prover::prover::ChannelHash` - Available
- `stwo_cairo_prover::prover::create_and_serialize_proof` - Available
- `stwo_cairo_prover::prover::prove_cairo` - Requires `ProverInput` (not available)
- `stwo_cairo_prover::adapter::ProverInput` - **Not exported** in crates.io version

### For Full Integration

To generate real proofs, you need the `ProverInput` type which is only available in the local stwo-cairo repository:

```bash
# Use local stwo-cairo with adapter module
cd /path/to/stwo-cairo
git checkout v1.1.0  # or main branch
```

Then update `stwo-bench/Cargo.toml` to use the local path:
```toml
stwo-cairo-prover = { path = "../../../crypto/stwo-cairo/stwo_cairo_prover/crates/prover" }
```

## Running Benchmarks

```bash
cd stwo-bench
cargo bench
```

The `rust-toolchain.toml` file will automatically use the correct nightly version when running cargo commands in this directory.

## Benchmark Results

The benchmarks measure:
- **Real proof generation** - Actual STWO prover execution time (currently placeholder with error)
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

### "ProverInput not found" error
This is expected with crates.io v1.1. Use local stwo-cairo for full integration.
