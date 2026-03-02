# RFC-0106: STARK Proof Benchmarks with Real STWO

## Status
Implemented

## Summary

Add benchmark infrastructure for STARK proof generation and verification using real STWO prover, with comparison to current mock implementation. Duplicated benchmarks for both mock and real (not feature-gated).

## Motivation

The current `STWOProver::prove()` method uses `generate_mock_proof()` which creates fake proof data. This doesn't reflect real STWO performance, making it impossible to:
- Measure actual proving time
- Compare mock vs real overhead
- Plan production infrastructure

## Specification

### Architecture

Instead of adding to the main crate, a separate sub-crate was created:

```
stwo-bench/
├── Cargo.toml              # Uses GitHub v1.1.0 tag
├── rust-toolchain.toml     # Pins nightly-2025-06-23
├── stwo_proof.rs          # Benchmark implementation
└── README.md              # Documentation
```

The main crate keeps mock benchmarks only, while real STWO benchmarks are isolated in `stwo-bench/`.

### Dependencies

The `stwo-bench` crate uses GitHub source with v1.1.0 tag:

```toml
stwo-cairo-prover = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
stwo-cairo-adapter = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
cairo-air = { git = "https://github.com/starkware-libs/stwo-cairo.git", tag = "v1.1.0" }
```

### Benchmark Implementation

The benchmarks use actual STWO API:
- `prove_cairo::<Blake2sMerkleChannel>()` for proof generation
- `verify_cairo::<Blake2sMerkleChannel>()` for verification
- `ProverInput` from `stwo_cairo_adapter`
- `ProverParameters` with Blake2s channel

### Benchmark Parameters

- **Batch sizes:** 10, 100, 1000 rows
- **Cairo programs:**
  - `hexary_verify.cairo` - Single proof verification
  - `merkle_batch.cairo` - Batch verification
  - `state_transition.cairo` - State transitions

### Cairo Program Compilation

Create `cairo/build.rs` to compile `.cairo` → `.casm` at build time:
```rust
fn main() {
    // Compile Cairo programs to CASM
    compile_cairo("hexary_verify");
    compile_cairo("merkle_batch");
    compile_cairo("state_transition");
}
```

### Prover Integration

Two separate methods in `STWOProver`:

```rust
// Existing - always available
fn generate_mock_proof(&self, program: &CairoProgram, inputs: &[u8]) -> Result<StarkProof, ProverError> {
    // Current implementation - creates fake proof
}

// New - requires zk feature (for real benchmarks)
#[cfg(feature = "zk")]
fn generate_real_proof(&self, program: &CairoProgram, inputs: &[u8]) -> Result<StarkProof, ProverError> {
    use stwo_cairo_prover::CairoProver;

    let prover = CairoProver::new(self.config.clone());
    prover.prove(program.casm(), inputs)
}
```

### Benchmark Implementation

All benchmarks behind `zk` feature:

```rust
#[cfg(feature = "zk")]
mod benches {
    use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

    // Mock
    pub fn bench_mock_proof_generation(c: &mut Criterion) {
        // ...
    }

    // Real
    pub fn bench_real_proof_generation(c: &mut Criterion) {
        // ...
    }
}
```

## Rationale

### Why Duplicated Benchmarks?

1. **No conditional compilation** - Clear, explicit code
2. **CI friendly** - Both run when zk is enabled
3. **Easy comparison** - Same structure, different implementation
4. **No runtime feature detection** - Compile-time selection

### Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| A: Feature-gated functions | Single code path | Complex, harder to compare |
| B: Runtime detection | Single binary | Slower, added complexity |
| C: Duplicated (chosen) | Clear, comparable | Slightly more code |

## Implementation

### Files Created

1. `stwo-bench/Cargo.toml` - Sub-crate with GitHub dependencies
2. `stwo-bench/rust-toolchain.toml` - Pins nightly-2025-06-23
3. `stwo-bench/stwo_proof.rs` - Real STWO benchmark implementation
4. `stwo-bench/README.md` - Documentation

### Files Modified

1. `benches/stark_proof.rs` - Simplified to mock-only benchmarks
2. `src/zk/prover.rs` - Added `generate_real_proof()` method (returns error with explanation)

### Cairo Programs

| Program | Hash | Purpose |
|--------|------|---------|
| `hexary_verify.cairo` | HEXARY_VERIFY_HASH | Verify hexary proofs |
| `merkle_batch.cairo` | MERKLE_BATCH_HASH | Batch verification |
| `state_transition.cairo` | STATE_TRANSITION_HASH | State transitions |

## Testing Requirements

- [x] Benchmarks compile with GitHub v1.1.0
- [x] Real proof generation works (~25-28s)
- [x] Real proof verification works (~15ms)

## Performance Expectations

- Mock generation: ~0ms (instant)
- Real generation: varies by program size (~25-28s for merkle_batch)
- Mock verification: ~0ms (instant)
- Real verification: ~15ms for merkle_batch

## Actual Results

| Operation | Time |
|-----------|------|
| Proof Generation (merkle_batch) | ~25-28 seconds |
| Proof Verification (merkle_batch) | ~15 ms |

## Related Use Cases

- [STARK Proof Benchmarks](../docs/use-cases/stark-proof-benchmarks.md)

## Related RFCs

- [RFC-0201: STWO and Cairo Integration](./0201-stwo-cairo-integration.md)
- [RFC-0202: Compressed Proofs](./0202-compressed-proofs.md)
